use std::{path::PathBuf, process::ExitStatus, str::FromStr as _};

use anyhow::bail;
use homedir::my_home;
use iroh::{EndpointId, RelayUrl, SecretKey};

use crate::{
    IrohSsh,
    cli::{ConnectArgs, ProxyArgs, ServerArgs},
    dot_ssh,
};

fn parse_relay_urls(urls: &[String]) -> anyhow::Result<Vec<RelayUrl>> {
    urls.iter()
        .map(|s| RelayUrl::from_str(s).map_err(|e| anyhow::anyhow!("invalid relay URL '{s}': {e}")))
        .collect()
}

pub async fn info_mode(key_dir: Option<PathBuf>) -> anyhow::Result<()> {
    let server_key = dot_ssh(
        &SecretKey::generate(&mut rand::rng()),
        false,
        false,
        key_dir.as_deref(),
    )
    .ok();
    let service_key = dot_ssh(
        &SecretKey::generate(&mut rand::rng()),
        false,
        true,
        key_dir.as_deref(),
    )
    .ok();

    if server_key.is_none() && service_key.is_none() {
        println!(
            "No keys found, run for server or service:\n  'iroh-ssh server --persist' or '-p' to create it"
        );
        println!();
        println!("(if an iroh-ssh instance is currently running, it is using ephemeral keys)");
        bail!("No keys found")
    }

    println!("iroh-ssh version {}", env!("CARGO_PKG_VERSION"));
    println!("https://github.com/rustonbsd/iroh-ssh");
    println!();

    if server_key.is_none() && service_key.is_none() {
        println!("run 'iroh-ssh server --persist' to start the server with persistent keys");
        println!("run 'iroh-ssh server' to start the server with ephemeral keys");
        println!(
            "run 'iroh-ssh service install' to copy the binary, install the service and start the server (always uses persistent keys)"
        );
    }

    if let Some(key) = server_key {
        println!();
        println!("Your server iroh-ssh endpoint id:");
        println!(
            "  iroh-ssh {}@{}",
            whoami::username().unwrap_or("UNKNOWN_USER".to_string()),
            key.clone().public()
        );
        println!();
    }

    if let Some(key) = service_key {
        println!();
        println!("Your service iroh-ssh endpoint id:");
        println!(
            "  iroh-ssh {}@{}",
            whoami::username().unwrap_or("UNKNOWN_USER".to_string()),
            key.clone().public()
        );
        println!();
    }

    Ok(())
}

pub mod service {
    use std::path::PathBuf;

    use crate::{ServiceParams, install_service, uninstall_service};

    pub async fn install(
        ssh_port: u16,
        key_dir: Option<PathBuf>,
        relay_url: Vec<String>,
        extra_relay_url: Vec<String>,
    ) -> anyhow::Result<()> {
        if install_service(ServiceParams {
            ssh_port,
            key_dir,
            relay_url,
            extra_relay_url,
        })
        .await
        .is_err()
        {
            anyhow::bail!("service install is only supported on linux and windows");
        }
        Ok(())
    }

    pub async fn uninstall() -> anyhow::Result<()> {
        if uninstall_service().await.is_err() {
            println!("service uninstall is only supported on linux or windows");
            anyhow::bail!("service uninstall is only supported on linux or windows");
        }
        Ok(())
    }
}

pub async fn server_mode(server_args: ServerArgs, service: bool) -> anyhow::Result<()> {
    let mut iroh_ssh_builder = IrohSsh::builder()
        .accept_incoming(true)
        .accept_port(server_args.ssh_port)
        .key_dir(server_args.key_dir.clone())
        .relay_urls(parse_relay_urls(&server_args.relay_url)?)
        .extra_relay_urls(parse_relay_urls(&server_args.extra_relay_url)?);
    if server_args.persist {
        iroh_ssh_builder = iroh_ssh_builder.dot_ssh_integration(true, service);
    }
    let iroh_ssh = iroh_ssh_builder.build().await?;

    println!("Connect to this this machine:");
    println!(
        "\n  iroh-ssh {}@{}\n",
        whoami::username().unwrap_or("UNKNOWN_USER".to_string()),
        iroh_ssh.endpoint_id()
    );
    if server_args.persist {
        let ssh_dir = match server_args.key_dir {
            Some(dir) => dir,
            None => {
                let distro_home =
                    my_home()?.ok_or_else(|| anyhow::anyhow!("home directory not found"))?;
                distro_home.join(".ssh")
            }
        };
        println!("  (using persistent keys in {})", ssh_dir.display());
    } else {
        println!(
            "  warning: (using ephemeral keys, run 'iroh-ssh server --persist' to create persistent keys)"
        );
    }
    println!();
    println!(
        "client -> iroh-ssh -> direct connect -> iroh-ssh -> local ssh :{}",
        server_args.ssh_port
    );

    println!("Waiting for incoming connections...");
    println!("Press Ctrl+C to exit");
    tokio::signal::ctrl_c().await?;
    Ok(())
}

pub async fn proxy_mode(proxy_args: ProxyArgs) -> anyhow::Result<()> {
    let iroh_ssh = IrohSsh::builder()
        .accept_incoming(false)
        .relay_urls(parse_relay_urls(&proxy_args.relay_url)?)
        .extra_relay_urls(parse_relay_urls(&proxy_args.extra_relay_url)?)
        .build()
        .await?;
    let hostname = proxy_args.endpoint_id.split(":").next().ok_or_else(|| anyhow::anyhow!("failed to parse hostname"))?;
    if hostname.len() == 64 && hostname.chars().all(|c| c.is_ascii_hexdigit()) {
        let endpoint_id = EndpointId::from_str(hostname)?;
        iroh_ssh.connect_pubkey(endpoint_id).await
    } else {
        // fallback to dns base (or ip) HostName connection (no iroh)
        iroh_ssh.connect_tcpip(&proxy_args.endpoint_id).await
    }
}

pub async fn client_mode(connect_args: ConnectArgs) -> anyhow::Result<()> {
    let iroh_ssh = IrohSsh::builder()
        .accept_incoming(false)
        .relay_urls(parse_relay_urls(&connect_args.relay_url)?)
        .extra_relay_urls(parse_relay_urls(&connect_args.extra_relay_url)?)
        .build()
        .await?;
    let mut ssh_process = iroh_ssh
        .start_ssh(
            connect_args.target,
            connect_args.ssh,
            connect_args.remote_cmd,
            &connect_args.relay_url,
            &connect_args.extra_relay_url,
        )
        .await?;

    let status = ssh_process.wait().await?;

    // this kills the process (ok is just here for now compile errors)
    exit_with_code(status);

    Ok(())
}

#[cfg(unix)]
pub(crate) fn exit_with_code(status: ExitStatus) {
    use std::os::unix::process::ExitStatusExt;

    if let Some(code) = status.code() {
        std::process::exit(code);
    }

    // if ssh gets killed locally
    if let Some(sig) = status.signal() {
        unsafe {
            libc::signal(sig, libc::SIG_DFL);
            libc::kill(libc::getpid(), sig);
        }
        // fallback if kill fails (same as windows)
        std::process::exit(128 + sig);
    }

    // fallback to 1 if don't know
    std::process::exit(1);
}

#[cfg(not(unix))]
pub(crate) fn exit_with_code(status: ExitStatus) {
    std::process::exit(status.code().unwrap_or(1));
}
