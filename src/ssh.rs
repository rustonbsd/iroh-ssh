use crate::{Builder, Inner, IrohSsh, api::ClientOptions};
use std::process::Stdio;

use anyhow::bail;
use ed25519_dalek::SECRET_KEY_LENGTH;
use homedir::my_home;
use iroh::{
    Endpoint, NodeId, SecretKey, Watcher,
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

    pub fn dot_ssh_integration(mut self, persist: bool, service: bool) -> Self {
        if let Ok(secret_key) = dot_ssh(&SecretKey::from_bytes(&self.secret_key), persist, service)
        {
            self.secret_key = secret_key.to_bytes();
        }
        self
    }

    pub async fn build(&mut self) -> anyhow::Result<IrohSsh> {
        // Iroh setup
        let secret_key = SecretKey::from_bytes(&self.secret_key);
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .bind()
            .await?;

        let _ = endpoint.home_relay().initialized().await?;

        let mut iroh_ssh = IrohSsh {
            public_key: *endpoint.node_id().as_bytes(),
            secret_key: self.secret_key,
            inner: None,
            ssh_port: self.accept_port.unwrap_or(22),
        };

        let router = if self.accept_incoming {
            Router::builder(endpoint.clone()).accept(IrohSsh::ALPN(), iroh_ssh.clone())
        } else {
            Router::builder(endpoint.clone())
        }
        .spawn();

        iroh_ssh.add_inner(endpoint, router);

        Ok(iroh_ssh)
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl IrohSsh {
    pub fn builder() -> Builder {
        Builder::new()
    }

    #[allow(non_snake_case)]
    pub fn ALPN() -> Vec<u8> {
        b"/iroh/ssh".to_vec()
    }

    fn add_inner(&mut self, endpoint: Endpoint, router: Router) {
        self.inner = Some(Inner { endpoint, router });
    }

    pub async fn connect(
        &self,
        ssh_user: &str,
        node_id: NodeId,
        client_options: ClientOptions,
        execute_command: Vec<String>,
    ) -> anyhow::Result<Child> {
        let inner = self.inner.as_ref().expect("inner not set");
        let conn = inner.endpoint.connect(node_id, &IrohSsh::ALPN()).await?;
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();

        tokio::spawn(async move {
            loop {
                let _ = handle_next(&listener, &conn).await;
            }

            async fn handle_next(
                listener: &TcpListener,
                conn: &Connection,
            ) -> Result<(), std::io::Error> {
                let (mut stream, _) = listener.accept().await?;
                let (mut iroh_send, mut iroh_recv) = conn.open_bi().await?;
                tokio::spawn(async move {
                    let (mut local_read, mut local_write) = stream.split();
                    let a_to_b =
                        async move { tokio::io::copy(&mut local_read, &mut iroh_send).await };
                    let b_to_a =
                        async move { tokio::io::copy(&mut iroh_recv, &mut local_write).await };

                    tokio::select! {
                        result = a_to_b => {
                            let _ = result;
                        },
                        result = b_to_a => {
                            let _ = result;
                        },
                    };
                });
                Ok(())
            }
        });
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        let mut cmd = &mut Command::new("ssh");
        cmd = cmd
            .arg("-tt") // Force pseudo-terminal allocation
            .arg(format!("{ssh_user}@127.0.0.1"))
            .arg("-p")
            .arg(port.to_string())
            .arg("-o")
            .arg("StrictHostKeyChecking=no")
            .arg("-o")
            .arg("UserKnownHostsFile=/dev/null")
            .arg("-o")
            .arg("LogLevel=ERROR"); // Reduce SSH debug output

        if let Some(identity_file) = client_options.identity_file {
            cmd.arg("-i").arg(identity_file);
        }
        if let Some(local_forward) = client_options.local_forward {
            cmd.arg("-L").arg(local_forward);
        }
        if let Some(remote_forward) = client_options.remote_forward {
            cmd.arg("-R").arg(remote_forward);
        }
        cmd.arg(execute_command.join(" "));

        let ssh_process = cmd
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
}

impl ProtocolHandler for IrohSsh {
    async fn accept(&self, connection: Connection) -> Result<(), iroh::protocol::AcceptError> {
        let alpn = connection
            .alpn()
            .ok_or_else(|| iroh::protocol::AcceptError::NotAllowed {})?;
        if alpn != IrohSsh::ALPN() {
            return Err(iroh::protocol::AcceptError::NotAllowed {});
        }

        let node_id = connection.remote_node_id()?;
        println!("{}: {node_id} connected", String::from_utf8_lossy(&alpn));

        match connection.accept_bi().await {
            Ok((mut iroh_send, mut iroh_recv)) => {
                println!("Accepted bidirectional stream from {node_id}");

                match TcpStream::connect(format!("127.0.0.1:{}", self.ssh_port)).await {
                    Ok(mut ssh_stream) => {
                        println!("Connected to local SSH server on port {}", self.ssh_port);

                        let (mut local_read, mut local_write) = ssh_stream.split();

                        let a_to_b =
                            async move { tokio::io::copy(&mut local_read, &mut iroh_send).await };
                        let b_to_a =
                            async move { tokio::io::copy(&mut iroh_recv, &mut local_write).await };

                        tokio::select! {
                            result = a_to_b => {
                                println!("SSH->Iroh stream ended: {result:?}");
                            },
                            result = b_to_a => {
                                println!("Iroh->SSH stream ended: {result:?}");
                            },
                        };
                    }
                    Err(e) => {
                        println!("Failed to connect to SSH server: {e}");
                    }
                }
            }
            Err(e) => {
                println!("Failed to accept bidirectional stream: {e}");
            }
        }

        Ok(())
    }
}

pub fn dot_ssh(
    default_secret_key: &SecretKey,
    persist: bool,
    _service: bool,
) -> anyhow::Result<SecretKey> {
    let distro_home = my_home()?.ok_or_else(|| anyhow::anyhow!("home directory not found"))?;
    #[allow(unused_mut)]
    let mut ssh_dir = distro_home.join(".ssh");

    // For now linux services are installed as "sudo'er" so
    // we need to use the root .ssh directory
    #[cfg(target_os = "linux")]
    if _service {
        ssh_dir = std::path::PathBuf::from("/root/.ssh");
    }

    // Weird windows System service profile location:
    // "C:\WINDOWS\system32\config\systemprofile\.ssh"
    #[cfg(target_os = "windows")]
    if _service {
        ssh_dir = std::path::PathBuf::from(r#"C:\WINDOWS\system32\config\systemprofile\.ssh"#);
    }

    let pub_key = ssh_dir.join("irohssh_ed25519.pub");
    let priv_key = ssh_dir.join("irohssh_ed25519");

    match (ssh_dir.exists(), persist) {
        (false, false) => {
            bail!(
                "no .ssh folder found in {}, use --persist flag to create it",
                distro_home.display()
            )
        }
        (false, true) => {
            std::fs::create_dir_all(&ssh_dir)?;
            println!("[INFO] created .ssh folder: {}", ssh_dir.display());
            dot_ssh(default_secret_key, persist, _service)
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
            bail!(
                "no iroh-ssh keys found in {}, use --persist flag to create it",
                ssh_dir.display()
            )
        }
    }
}
