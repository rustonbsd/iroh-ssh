use std::{path::PathBuf, str::FromStr as _};

use anyhow::bail;
use homedir::my_home;
use iroh::{NodeId, SecretKey};

use crate::{dot_ssh,  IrohSsh};

pub async fn info_mode() -> anyhow::Result<()> {
    let server_key = match dot_ssh(&SecretKey::generate(rand::rngs::OsRng), false,false) {
        Ok(key) => Some(key),
        Err(_) => None,
    };
    let service_key = match dot_ssh(&SecretKey::generate(rand::rngs::OsRng), false,true) {
        Ok(key) => Some(key),
        Err(_) => None,
    };
    
    if server_key.is_none()  && service_key.is_none() {
        println!("No keys found, run for server or service:\n  'iroh-ssh server --persist' or '-p' to create it");
        println!();
        println!("(if an iroh-ssh instance is currently running, it is using ephemeral keys)");
        bail!("No keys found")
    }

    println!("iroh-ssh version {}", env!("CARGO_PKG_VERSION"));
    println!("https://github.com/rustonbsd/iroh-ssh");
    println!("");

    if server_key.is_none() && service_key.is_none() {
        println!("run 'iroh-ssh server --persist' to start the server with persistent keys");
        println!("run 'iroh-ssh server' to start the server with ephemeral keys");
        println!(
            "run 'iroh-ssh service install' to copy the binary, install the service and start the server (always uses persistent keys)"
        );
    }
    
    if let Some(key) = server_key {
        println!("");
        println!("Your server iroh-ssh nodeid:");
        println!("  iroh-ssh {}@{}", whoami::username(), key.clone().public().to_string());
        println!("");
    }
    
    if let Some(key) = service_key {
        println!("");
        println!("Your service iroh-ssh nodeid:");
        println!("  iroh-ssh {}@{}", whoami::username(), key.clone().public().to_string());
        println!("");
    }
    
    Ok(())
}

pub mod service {
    use crate::{install_service, uninstall_service, ServiceParams};

    pub async fn install(ssh_port: u16) -> anyhow::Result<()> {
        if install_service(ServiceParams { ssh_port }).await.is_err() {
            println!("service install is only supported on linux and windows");
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

pub async fn server_mode(ssh_port: u16, persist: bool) -> anyhow::Result<()> {
    let mut iroh_ssh_builder = IrohSsh::new().accept_incoming(true).accept_port(ssh_port);
    if persist {
        iroh_ssh_builder = iroh_ssh_builder.dot_ssh_integration(true,false);
    }
    let iroh_ssh = iroh_ssh_builder.build().await?;

    println!("Connect to this this machine:");
    println!("\n  iroh-ssh {}@{}\n", whoami::username(), iroh_ssh.node_id());
    if persist {
        
        let distro_home = my_home()?.ok_or_else(|| anyhow::anyhow!("home directory not found"))?;
        let ssh_dir = distro_home.join(".ssh");
        println!("  (using persistent keys in {})", ssh_dir.display());
    } else {
        println!("  warning: (using ephemeral keys, run 'iroh-ssh server --persist' to create persistent keys)");
    }
    println!("");
    println!(
        "client -> iroh-ssh -> direct connect -> iroh-ssh -> local ssh :{}",
        ssh_port
    );

    println!("Waiting for incoming connections...");
    println!("Press Ctrl+C to exit");
    tokio::signal::ctrl_c().await?;
    Ok(())
}

pub async fn client_mode(target: String, identity_file: Option<PathBuf>) -> anyhow::Result<()> {
    let (ssh_user, iroh_node_id) = parse_iroh_target(&target)?;
    let iroh_ssh = IrohSsh::new().accept_incoming(false).build().await?;
    let mut ssh_process = iroh_ssh.connect(&ssh_user, iroh_node_id, identity_file).await?;

    ssh_process.wait().await?;

    Ok(())
}

fn parse_iroh_target(target: &str) -> anyhow::Result<(String, NodeId)> {
    let (user, node_id_str) = target
        .split_once('@')
        .ok_or_else(|| anyhow::anyhow!("Invalid format, use user@node_id"))?;
    let node_id = NodeId::from_str(node_id_str)?;
    Ok((user.to_string(), node_id))
}
