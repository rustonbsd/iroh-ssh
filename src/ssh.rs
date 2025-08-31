use crate::{cli::SshOpts, Builder, Inner, IrohSsh};
use std::{ffi::OsString, process::Stdio};

use anyhow::bail;
use ed25519_dalek::SECRET_KEY_LENGTH;
use homedir::my_home;
use iroh::{
    Endpoint, NodeId, SecretKey, Watcher,
    endpoint::Connection,
    protocol::{ProtocolHandler, Router},
};
use tokio::{
    net::TcpStream,
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

    pub async fn start_ssh(&self, target: String, ssh_opts: SshOpts, remote_cmd: Vec<OsString>) -> anyhow::Result<Child> {
        let c_exe = std::env::current_exe()?;
        let cmd = &mut Command::new("ssh");
        
        cmd.arg("-o").arg(format!(
            "ProxyCommand={} proxy %h",
            c_exe.display()
        ));

        if let Some(p) = ssh_opts.port {
            cmd.arg("-p").arg(p.to_string());
        }
        if let Some(id) = &ssh_opts.identity_file {
            cmd.arg("-i").arg(id);
        }
        for l in &ssh_opts.local_forward {
            cmd.arg("-L").arg(l);
        }
        for r in &ssh_opts.remote_forward {
            cmd.arg("-R").arg(r);
        }
        for o in &ssh_opts.options {
            cmd.arg("-o").arg(o);
        }
        if ssh_opts.agent {
            cmd.arg("-A");
        }
        if ssh_opts.no_agent {
            cmd.arg("-a");
        }
        if ssh_opts.x11_trusted {
            cmd.arg("-Y");
        } else if ssh_opts.x11 {
            cmd.arg("-X");
        }
        if ssh_opts.no_cmd {
            cmd.arg("-N");
        }
        if ssh_opts.force_tty {
            cmd.arg("-t");
        }
        if ssh_opts.no_tty {
            cmd.arg("-T");
        }
        for _ in 0..ssh_opts.verbose {
            cmd.arg("-v");
        }
        if ssh_opts.quiet {
            cmd.arg("-q");
        }

        cmd.arg(target);

        if !remote_cmd.is_empty() {
            cmd.args(remote_cmd.iter());
        }

        let ssh_process = cmd
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        Ok(ssh_process)
    }

    pub async fn connect(&self, node_id: NodeId) -> anyhow::Result<()> {
        println!("proxying...");
        let inner = self.inner.as_ref().expect("inner not set");
        let conn = inner.endpoint.connect(node_id, &IrohSsh::ALPN()).await?;
        let (mut iroh_send, mut iroh_recv) = conn.open_bi().await?;
        let (mut local_read, mut local_write) = (tokio::io::stdin(), tokio::io::stdout());
        let a_to_b = async move { tokio::io::copy(&mut local_read, &mut iroh_send).await };
        let b_to_a = async move { tokio::io::copy(&mut iroh_recv, &mut local_write).await };

        tokio::select! {
            result = a_to_b => {
                let _ = result;
            },
            result = b_to_a => {
                let _ = result;
            },
        };
        Ok(())
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
