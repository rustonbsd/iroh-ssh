use crate::{Builder, Inner, IrohSsh, cli::SshOpts};
use std::{ffi::OsString, path::Path, process::Stdio};

use anyhow::bail;
use ed25519_dalek::SECRET_KEY_LENGTH;
use homedir::my_home;
use std::sync::Arc;

use iroh::{
    Endpoint, EndpointId, RelayConfig, RelayUrl, SecretKey,
    endpoint::{Connection, RelayMode},
    protocol::{ProtocolHandler, Router},
};
use tokio::{
    io::AsyncWriteExt, net::TcpStream, process::{Child, Command}
};

impl Builder {
    pub fn new() -> Self {
        Self {
            secret_key: SecretKey::generate(&mut rand::rng()).to_bytes(),
            accept_incoming: false,
            accept_port: None,
            key_dir: None,
            relay_urls: Vec::new(),
            extra_relay_urls: Vec::new(),
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

    pub fn relay_urls(mut self, urls: Vec<RelayUrl>) -> Self {
        self.relay_urls = urls;
        self
    }

    pub fn extra_relay_urls(mut self, urls: Vec<RelayUrl>) -> Self {
        self.extra_relay_urls = urls;
        self
    }

    pub fn key_dir(mut self, key_dir: Option<std::path::PathBuf>) -> Self {
        self.key_dir = key_dir;
        self
    }

    pub fn dot_ssh_integration(mut self, persist: bool, service: bool) -> Self {
        tracing::info!(
            "dot_ssh_integration: persist={}, service={}",
            persist,
            service
        );

        match dot_ssh(
            &SecretKey::from_bytes(&self.secret_key),
            persist,
            service,
            self.key_dir.as_deref(),
        ) {
            Ok(secret_key) => {
                tracing::info!("dot_ssh_integration: Successfully loaded/created SSH keys");
                self.secret_key = secret_key.to_bytes();
            }
            Err(e) => {
                tracing::error!(
                    "dot_ssh_integration: Failed to load/create SSH keys: {:#}",
                    e
                );
                eprintln!("Warning: Failed to load/create persistent SSH keys: {e:#}");
                eprintln!("Continuing with ephemeral keys...");
            }
        }
        self
    }

    pub async fn build(&mut self) -> anyhow::Result<IrohSsh> {
        // Iroh setup
        let secret_key = SecretKey::from_bytes(&self.secret_key);
        let mut builder = Endpoint::builder().secret_key(secret_key);

        if !self.relay_urls.is_empty() {
            let relay_map = self.relay_urls.iter().cloned().collect();
            builder = builder.relay_mode(RelayMode::Custom(relay_map));
        } else if !self.extra_relay_urls.is_empty() {
            let relay_map = RelayMode::Default.relay_map();
            for url in &self.extra_relay_urls {
                relay_map.insert(url.clone(), Arc::new(RelayConfig::from(url.clone())));
            }
            builder = builder.relay_mode(RelayMode::Custom(relay_map));
        }

        let endpoint = builder.bind().await?;

        let mut iroh_ssh = IrohSsh {
            public_key: *endpoint.id().as_bytes(),
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

    pub async fn start_ssh(
        &self,
        target: String,
        ssh_opts: SshOpts,
        remote_cmd: Vec<OsString>,
        relay_urls: &[String],
        extra_relay_urls: &[String],
    ) -> anyhow::Result<Child> {
        let c_exe = std::env::current_exe()?;
        let mut cmd = build_ssh_command(
            &c_exe,
            target,
            ssh_opts,
            remote_cmd,
            relay_urls,
            extra_relay_urls,
        );

        let ssh_process = cmd
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        Ok(ssh_process)
    }

    pub async fn connect_pubkey(&self, endpoint_id: EndpointId) -> anyhow::Result<()> {
        let inner = self.inner.as_ref().expect("inner not set");
        let conn = inner
            .endpoint
            .connect(endpoint_id, &IrohSsh::ALPN())
            .await?;
        let (mut iroh_send, mut iroh_recv) = conn.open_bi().await?;
        let (mut local_read, mut local_write) = (tokio::io::stdin(), tokio::io::stdout());
        let a_to_b = async move {
            let res = tokio::io::copy(&mut local_read, &mut iroh_send).await;
            iroh_send.finish().ok();
            res
        };
        let b_to_a = async move { tokio::io::copy(&mut iroh_recv, &mut local_write).await };

        let (_, _) = tokio::join!(a_to_b, b_to_a);
        Ok(())
    }

    pub async fn connect_tcpip(&self, host_addr: &str) -> anyhow::Result<()> {
        let conn = tokio::net::TcpStream::connect(host_addr).await?;
        let (mut tcp_read, mut tcp_write) = conn.into_split();
        let (mut local_read, mut local_write) = (tokio::io::stdin(), tokio::io::stdout());
        let a_to_b = async move {
            let res = tokio::io::copy(&mut local_read, &mut tcp_write).await;
            tcp_write.shutdown().await.ok();
            res
        };
        let b_to_a = async move { tokio::io::copy(&mut tcp_read, &mut local_write).await };

        let (_, _) = tokio::join!(a_to_b, b_to_a);
        Ok(())
    }


    pub fn endpoint_id(&self) -> EndpointId {
        self.inner.as_ref().expect("inner not set").endpoint.id()
    }
}

fn build_ssh_command(
    iroh_ssh_exe: &Path,
    target: String,
    ssh_opts: SshOpts,
    remote_cmd: Vec<OsString>,
    relay_urls: &[String],
    extra_relay_urls: &[String],
) -> Command {
    let mut cmd = Command::new("ssh");

    let mut proxy_cmd = format!("{} proxy", iroh_ssh_exe.display());
    for url in relay_urls {
        proxy_cmd.push_str(&format!(" --relay-url {url}"));
    }
    for url in extra_relay_urls {
        proxy_cmd.push_str(&format!(" --extra-relay-url {url}"));
    }
    proxy_cmd.push_str(" %h:%p");
    cmd.arg("-o").arg(format!("ProxyCommand={proxy_cmd}"));

    if let Some(p) = ssh_opts.port {
        cmd.arg("-p").arg(p.to_string());
    }
    if let Some(u) = &ssh_opts.login_user {
        cmd.arg("-l").arg(u);
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

    cmd
}

impl ProtocolHandler for IrohSsh {
    async fn accept(&self, connection: Connection) -> Result<(), iroh::protocol::AcceptError> {
        let endpoint_id = connection.remote_id()?;

        match connection.accept_bi().await {
            Ok((mut iroh_send, mut iroh_recv)) => {
                println!("Accepted bidirectional stream from {endpoint_id}");

                match TcpStream::connect(format!("127.0.0.1:{}", self.ssh_port)).await {
                    Ok(mut ssh_stream) => {
                        println!("Connected to local SSH server on port {}", self.ssh_port);

                        let (mut local_read, mut local_write) = ssh_stream.split();

                        let a_to_b = async move {
                            let res = tokio::io::copy(&mut local_read, &mut iroh_send).await;
                            iroh_send.finish().ok();
                            res
                        };
                        let b_to_a =
                            async move { tokio::io::copy(&mut iroh_recv, &mut local_write).await };

                        let (_, _) = tokio::join!(a_to_b, b_to_a);
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
    key_dir: Option<&Path>,
) -> anyhow::Result<SecretKey> {
    tracing::info!(
        "dot_ssh: Function called, persist={}, service={}, key_dir={:?}",
        persist,
        _service,
        key_dir,
    );

    #[allow(unused_mut)]
    let mut ssh_dir = if let Some(dir) = key_dir {
        dir.to_path_buf()
    } else {
        let distro_home = my_home()?.ok_or_else(|| anyhow::anyhow!("home directory not found"))?;
        distro_home.join(".ssh")
    };

    // Only apply service-specific overrides when no explicit key_dir is set
    if key_dir.is_none() {
        // For now linux services are installed as "sudo'er" so
        // we need to use the root .ssh directory
        #[cfg(target_os = "linux")]
        if _service {
            ssh_dir = std::path::PathBuf::from("/root/.ssh");
        }

        // Windows virtual service account profile location for NT SERVICE\iroh-ssh
        #[cfg(target_os = "windows")]
        if _service {
            ssh_dir = std::path::PathBuf::from(crate::service::WindowsService::SERVICE_SSH_DIR);
            tracing::info!("dot_ssh: Using service SSH dir: {}", ssh_dir.display());

            // Ensure directory exists when running as service
            if !ssh_dir.exists() {
                tracing::info!("dot_ssh: Service SSH dir doesn't exist, creating it");
                std::fs::create_dir_all(&ssh_dir)?;
            }
        }
    }

    let pub_key = ssh_dir.join("irohssh_ed25519.pub");
    let priv_key = ssh_dir.join("irohssh_ed25519");

    tracing::debug!("dot_ssh: ssh_dir exists = {}", ssh_dir.exists());
    tracing::debug!("dot_ssh: pub_key path = {}", pub_key.display());
    tracing::debug!("dot_ssh: priv_key path = {}", priv_key.display());

    match (ssh_dir.exists(), persist) {
        (false, false) => {
            tracing::error!(
                "dot_ssh: ssh_dir does not exist and persist=false: {}",
                ssh_dir.display()
            );
            bail!(
                "key directory {} does not exist, use --persist flag to create it",
                ssh_dir.display()
            )
        }
        (false, true) => {
            tracing::info!("dot_ssh: Creating ssh_dir: {}", ssh_dir.display());
            std::fs::create_dir_all(&ssh_dir)?;
            println!("[INFO] created .ssh folder: {}", ssh_dir.display());
            dot_ssh(default_secret_key, persist, _service, key_dir)
        }
        (true, true) => {
            tracing::info!("dot_ssh: Branch (true, true) - directory exists, persist enabled");
            tracing::debug!("dot_ssh: pub_key.exists() = {}", pub_key.exists());
            tracing::debug!("dot_ssh: priv_key.exists() = {}", priv_key.exists());

            // check pub and priv key already exists
            if pub_key.exists() && priv_key.exists() {
                tracing::info!("dot_ssh: Keys exist, reading them");
                // read secret key
                if let Ok(secret_key) = std::fs::read(priv_key.clone()) {
                    let mut sk_bytes = [0u8; SECRET_KEY_LENGTH];
                    sk_bytes.copy_from_slice(z32::decode(secret_key.as_slice())?.as_slice());
                    Ok(SecretKey::from_bytes(&sk_bytes))
                } else {
                    bail!("failed to read secret key from {}", priv_key.display())
                }
            } else {
                tracing::info!("dot_ssh: Keys don't exist, creating new keys");
                tracing::debug!("dot_ssh: Writing to pub_key: {}", pub_key.display());
                tracing::debug!("dot_ssh: Writing to priv_key: {}", priv_key.display());

                let secret_key = default_secret_key.clone();
                let public_key = secret_key.public();

                match std::fs::write(&pub_key, z32::encode(public_key.as_bytes())) {
                    Ok(_) => {
                        tracing::info!("dot_ssh: Successfully wrote pub_key");
                    }
                    Err(e) => {
                        tracing::error!(
                            "dot_ssh: Failed to write pub_key: {} (error kind: {:?})",
                            e,
                            e.kind()
                        );
                        return Err(e.into());
                    }
                }

                match std::fs::write(&priv_key, z32::encode(&secret_key.to_bytes())) {
                    Ok(_) => {
                        tracing::info!("dot_ssh: Successfully wrote priv_key");
                    }
                    Err(e) => {
                        tracing::error!(
                            "dot_ssh: Failed to write priv_key: {} (error kind: {:?})",
                            e,
                            e.kind()
                        );
                        return Err(e.into());
                    }
                }

                Ok(secret_key)
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn args_of(cmd: &Command) -> Vec<String> {
        cmd.as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn login_user_flag_is_passed_to_ssh() {
        let mut opts = SshOpts::default();
        opts.login_user = Some("alice".to_string());

        let cmd = build_ssh_command(
            Path::new("/usr/bin/iroh-ssh"),
            "endpoint123".to_string(),
            opts,
            Vec::new(),
            &[],
            &[],
        );

        let args = args_of(&cmd);
        let l_pos = args.iter().position(|a| a == "-l").expect("-l not found");
        assert_eq!(args[l_pos + 1], "alice");
    }

    #[test]
    fn rsync_invocation_parses_and_builds() {
        // Mirrors `rsync -e iroh-ssh /local user@<id>:/remote`, which invokes:
        //   iroh-ssh -l alice <endpoint_id> rsync --server -e.LsfxCIvu . /tmp/dest
        let cli = crate::cli::Cli::try_parse_from([
            "iroh-ssh",
            "-l",
            "alice",
            "endpoint123",
            "rsync",
            "--server",
            "-e.LsfxCIvu",
            ".",
            "/tmp/dest",
        ])
        .expect("CLI should accept rsync's invocation pattern");

        assert_eq!(cli.target.as_deref(), Some("endpoint123"));
        assert_eq!(cli.ssh.login_user.as_deref(), Some("alice"));
        let remote_cmd_raw = cli.remote_cmd.unwrap_or_default();
        let remote_cmd: Vec<String> = remote_cmd_raw
            .iter()
            .map(|o| o.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            remote_cmd,
            vec!["rsync", "--server", "-e.LsfxCIvu", ".", "/tmp/dest"]
        );

        let cmd = build_ssh_command(
            Path::new("/usr/bin/iroh-ssh"),
            cli.target.unwrap(),
            cli.ssh,
            remote_cmd_raw,
            &[],
            &[],
        );
        let args = args_of(&cmd);

        // -l alice should appear before the host arg
        let host_pos = args
            .iter()
            .position(|a| a == "endpoint123")
            .expect("host arg not found");
        let l_pos = args.iter().position(|a| a == "-l").expect("-l not found");
        assert!(l_pos < host_pos);
        assert_eq!(args[l_pos + 1], "alice");

        // remote command should follow the host
        assert_eq!(args[host_pos + 1], "rsync");
        assert_eq!(args[host_pos + 2], "--server");
        assert_eq!(args[host_pos + 3], "-e.LsfxCIvu");
    }
}
