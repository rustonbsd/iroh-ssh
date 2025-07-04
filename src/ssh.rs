macro_rules! ok_or_continue {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(_) => continue,
        }
    };
}

use crate::{Builder, Inner, IrohSsh};
use std::{pin::Pin, process::Stdio};

use anyhow::bail;
use ed25519_dalek::SECRET_KEY_LENGTH;
use homedir::my_home;
use iroh::{
    Endpoint, NodeId, SecretKey,
    endpoint::Connection,
    protocol::{ProtocolHandler, Router},
};
use tokio::{
    net::{TcpListener, TcpStream},
    process::{Child, Command},
};

impl Builder {
    pub fn new() -> Self {
        Self {
            secret_key: SecretKey::generate(rand::rngs::OsRng).to_bytes(),
            accept_incoming: false,
            accept_port: None,
        }
    }

    pub fn accept_incoming(mut self, accept_incoming: bool) -> Self {
        self.accept_incoming = accept_incoming;
        self
    }

    pub fn accept_port(mut self, accept_port: u16) -> Self {
        self.accept_port = Some(accept_port);
        self
    }

    pub fn secret_key(mut self, secret_key: &[u8; SECRET_KEY_LENGTH]) -> Self {
        self.secret_key = *secret_key;
        self
    }

    pub fn dot_ssh_integration(mut self, persist: bool) -> Self {
        if let Ok(secret_key) = dot_ssh(&SecretKey::from_bytes(&self.secret_key), persist) {
            self.secret_key = secret_key.to_bytes();
        }
        self
    }

    pub async fn build(self: &mut Self) -> anyhow::Result<IrohSsh> {
        // Iroh setup
        let secret_key = SecretKey::from_bytes(&self.secret_key);
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .bind()
            .await?;

        wait_for_relay(&endpoint).await?;

        let mut iroh_ssh = IrohSsh {
            public_key: endpoint.node_id().as_bytes().clone(),
            secret_key: self.secret_key,
            inner: None,
        };

        let router = if self.accept_incoming {
            Router::builder(endpoint.clone()).accept(&IrohSsh::ALPN(), iroh_ssh.clone())
        } else {
            Router::builder(endpoint.clone())
        }
        .spawn();

        iroh_ssh.add_inner(endpoint, router);

        if self.accept_incoming && self.accept_port.is_some() {
            tokio::spawn({
                let iroh_ssh = iroh_ssh.clone();
                let accept_port = self.accept_port.expect("accept_port not set");
                async move {
                    iroh_ssh._spawn(accept_port).await.expect("spawn failed");
                }
            });
        }

        Ok(iroh_ssh)
    }
}

impl IrohSsh {
    pub fn new() -> Builder {
        Builder::new()
    }

    #[allow(non_snake_case)]
    pub fn ALPN() -> Vec<u8> {
        format!("/iroh/ssh").into_bytes()
    }

    fn add_inner(&mut self, endpoint: Endpoint, router: Router) {
        self.inner = Some(Inner { endpoint, router });
    }

    pub async fn connect(&self, ssh_user: &str, node_id: NodeId) -> anyhow::Result<Child> {
        let inner = self.inner.as_ref().expect("inner not set");
        let conn = inner.endpoint.connect(node_id, &IrohSsh::ALPN()).await?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();

        tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((mut stream, _)) => match conn.open_bi().await {
                        Ok((mut iroh_send, mut iroh_recv)) => {
                            tokio::spawn(async move {
                                let (mut local_read, mut local_write) = stream.split();
                                let a_to_b = async move {
                                    tokio::io::copy(&mut local_read, &mut iroh_send).await
                                };
                                let b_to_a = async move {
                                    tokio::io::copy(&mut iroh_recv, &mut local_write).await
                                };

                                tokio::select! {
                                    result = a_to_b => {
                                        let _ = result;
                                    },
                                    result = b_to_a => {
                                        let _ = result;
                                    },
                                };
                            });
                        }
                        Err(_) => break,
                    },
                    Err(_) => break,
                }
            }
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let ssh_process = Command::new("ssh")
            .arg("-tt") // Force pseudo-terminal allocation
            .arg(format!("{}@127.0.0.1", ssh_user))
            .arg("-p")
            .arg(port.to_string())
            .arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg("-o")
            .arg("UserKnownHostsFile=/dev/null")
            .arg("-o")
            .arg("LogLevel=ERROR") // Reduce SSH debug output
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        Ok(ssh_process)
    }

    pub fn node_id(&self) -> NodeId {
        self.inner
            .as_ref()
            .expect("inner not set")
            .endpoint
            .node_id()
    }

    async fn _spawn(self, port: u16) -> anyhow::Result<()> {
        println!("Server listening for iroh connections...");

        while let Some(incoming) = self
            .inner
            .clone()
            .expect("inner not set")
            .endpoint
            .accept()
            .await
        {
            let mut connecting = match incoming.accept() {
                Ok(connecting) => connecting,
                Err(err) => {
                    println!("Incoming connection failure: {err:#}");
                    continue;
                }
            };

            let alpn = ok_or_continue!(connecting.alpn().await);
            let conn = ok_or_continue!(connecting.await);
            let node_id = ok_or_continue!(conn.remote_node_id());

            println!("{}: {node_id} connected", String::from_utf8_lossy(&alpn));

            tokio::spawn(async move {
                match conn.accept_bi().await {
                    Ok((mut iroh_send, mut iroh_recv)) => {
                        println!("Accepted bidirectional stream from {}", node_id);

                        match TcpStream::connect(format!("127.0.0.1:{}", port)).await {
                            Ok(mut ssh_stream) => {
                                println!("Connected to local SSH server on port {}", port);

                                let (mut local_read, mut local_write) = ssh_stream.split();

                                let a_to_b = async move {
                                    tokio::io::copy(&mut local_read, &mut iroh_send).await
                                };
                                let b_to_a = async move {
                                    tokio::io::copy(&mut iroh_recv, &mut local_write).await
                                };

                                tokio::select! {
                                    result = a_to_b => {
                                        println!("SSH->Iroh stream ended: {:?}", result);
                                    },
                                    result = b_to_a => {
                                        println!("Iroh->SSH stream ended: {:?}", result);
                                    },
                                };
                            }
                            Err(e) => {
                                println!("Failed to connect to SSH server: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!("Failed to accept bidirectional stream: {}", e);
                    }
                }
            });
        }
        Ok(())
    }
}

impl ProtocolHandler for IrohSsh {
    fn accept(
        &self,
        conn: Connection,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'static>> {
        let iroh_ssh = self.clone();

        Box::pin(async move {
            iroh_ssh.accept(conn).await?;
            Ok(())
        })
    }
}

pub fn dot_ssh(default_secret_key: &SecretKey, persist: bool) -> anyhow::Result<SecretKey> {
    let distro_home = my_home()?.ok_or_else(|| anyhow::anyhow!("home directory not found"))?;
    let ssh_dir = distro_home.join(".ssh");
    let pub_key = ssh_dir.join("irohssh_ed25519.pub");
    let priv_key = ssh_dir.join("irohssh_ed25519");

    match (ssh_dir.exists(), persist) {
        (false, false) => {
            bail!("no .ssh folder found in {}, use --persist flag to create it", distro_home.display())
        }
        (false, true) => {
            std::fs::create_dir_all(&ssh_dir)?;
            println!("[INFO] created .ssh folder: {}", ssh_dir.display());
            dot_ssh(default_secret_key, persist)
        }
        (true, true) => {
            // check pub and priv key already exists
            if pub_key.exists() && priv_key.exists() {
                // read secret key
                if let Ok(secret_key) = std::fs::read(priv_key.clone()) {
                    let mut sk_bytes = [0u8; SECRET_KEY_LENGTH];
                    sk_bytes.copy_from_slice(z32::decode(secret_key.as_slice())?.as_slice());
                    Ok(SecretKey::from_bytes(&sk_bytes))
                } else {
                    bail!("failed to read secret key from {}", priv_key.display())
                }
            } else {
                let key = default_secret_key.clone();
                let secret_key = key.secret();
                let public_key = key.public();

                std::fs::write(pub_key, z32::encode(public_key.as_bytes()))?;
                std::fs::write(priv_key, z32::encode(secret_key.as_bytes()))?;
                Ok(key)
            }
        }
        (true, false) => {
            // check pub and priv key already exists
            if pub_key.exists() && priv_key.exists() {
                // read secret key
                if let Ok(secret_key) = std::fs::read(priv_key.clone()) {
                    let mut sk_bytes = [0u8; SECRET_KEY_LENGTH];
                    sk_bytes.copy_from_slice(z32::decode(secret_key.as_slice())?.as_slice());
                    return Ok(SecretKey::from_bytes(&sk_bytes));
                }
            }
            bail!("no iroh-ssh keys found in {}, use --persist flag to create it", ssh_dir.display())
        }
    }
}

async fn wait_for_relay(endpoint: &Endpoint) -> anyhow::Result<()> {
    while endpoint.home_relay().get().is_err() {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    Ok(())
}
