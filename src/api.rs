use std::str::FromStr as _;

use anyhow::bail;
use homedir::my_home;
use iroh::{NodeId, SecretKey};

use crate::{dot_ssh, install_service, IrohSsh, ServiceParams};

pub async fn info_mode() -> anyhow::Result<()> {
    let key = dot_ssh(&SecretKey::generate(rand::rngs::OsRng), false);
    if key.is_err() {
        println!("No keys found, run 'iroh-ssh server --persist' or with '-p' to create it");
        println!("(if an iroh-ssh instance is currently running, it is using ephemeral keys)");
        bail!("No keys found")
    }

    let key = key.unwrap();
    let node_id = key.public();
    println!("Your iroh-ssh nodeid: {}", node_id.to_string());

    println!("iroh-ssh version {}", env!("CARGO_PKG_VERSION"));
    println!("https://github.com/rustonbsd/iroh-ssh");
    println!("");
    println!("run 'iroh-ssh server --persist' to start the server with persistent keys");
    println!("run 'iroh-ssh server' to start the server with ephemeral keys");
    println!(
        "run 'iroh-ssh service' to start the server as a service (always uses persistent keys)"
    );
    println!("");
    println!("Your iroh-ssh nodeid:");
    println!("  iroh-ssh root@{}\n\n", key.public().to_string());
    Ok(())
}

pub async fn service_mode(ssh_port: u16) -> anyhow::Result<()> {
    if install_service(ServiceParams { ssh_port }).await.is_err() {
        println!("Service mode is only supported on linux and windows");
    }
    Ok(())
}

pub async fn server_mode(ssh_port: u16, persist: bool) -> anyhow::Result<()> {
    let mut iroh_ssh_builder = IrohSsh::new().accept_incoming(true).accept_port(ssh_port);
    if persist {
        iroh_ssh_builder = iroh_ssh_builder.dot_ssh_integration(true);
    }
    let iroh_ssh = iroh_ssh_builder.build().await?;

    println!("Connect to this this machine:");
    println!("\n  iroh-ssh my-user@{}\n", iroh_ssh.node_id());
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

pub async fn client_mode(target: String) -> anyhow::Result<()> {
    let (ssh_user, iroh_node_id) = parse_iroh_target(&target)?;
    let iroh_ssh = IrohSsh::new().accept_incoming(false).build().await?;
    let mut ssh_process = iroh_ssh.connect(&ssh_user, iroh_node_id).await?;

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
