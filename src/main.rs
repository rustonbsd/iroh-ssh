use std::str::FromStr;

use anyhow::bail;
use clap::{Parser, Subcommand, command};
use iroh::{NodeId, SecretKey};
use iroh_ssh::{IrohSsh, dot_ssh};
use tokio::process::Command;

#[derive(Parser)]
#[command(name = "irohssh", about = "SSH without IP", after_help = "
Usage Examples:
  iroh-ssh server --persist                // Start server with persistent keys
  iroh-ssh my-user@6598395384059bf969...   // Connect to server
  iroh-ssh service                         // Linux only
")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    target: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(about = "Connect to a remote server - iroh-ssh `connect` my-user@NODE_ID (`connect` is optional)")]
    Connect {
        target: String,
    },
    #[command(about = "Run as server (for exampel in a tmux session)")]
    Server {
        #[arg(long, default_value = "22")]
        ssh_port: u16,
        #[arg(short, long, default_value = "false")]
        persist: bool,
    },
    #[command(about = "Run as service (linux only, uses persistent keys)")]
    Service {
        #[arg(long, default_value = "22")]
        ssh_port: u16,
    },
    #[command(about = "Display connection information")]
    Info {},
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match (cli.command, cli.target) {
        (Some(Commands::Connect { target }), _) => client_mode(target).await,
        (Some(Commands::Server { ssh_port, persist }), _) => server_mode(ssh_port, persist).await,
        (Some(Commands::Service { ssh_port }), _) => service_mode(ssh_port).await,
        (Some(Commands::Info {}), _) => info_mode().await,
        (None, Some(target)) => client_mode(target).await,
        (None, None) => {
            anyhow::bail!("Please provide a target or use 'connect' subcommand")
        }
    }
}

async fn info_mode() -> anyhow::Result<()> {
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

async fn service_mode(ssh_port: u16) -> anyhow::Result<()> {
    // only on linux with systemctl
    if !cfg!(target_os = "linux") {
        anyhow::bail!("Service mode is only supported on linux")
    }

    let service_raw = r#"[Unit]
Description=SSH over Iroh

[Service]
Type=simple
WorkingDirectory=~
ExecStart=/bin/bash -c 'iroh-ssh server --ssh-port [SSHPORT]'
Restart=on-failure
RestartSec=3s

[Install]
WantedBy=multi-user.target
"#;

    let service_raw = service_raw.replace("[SSHPORT]", &ssh_port.to_string());

    let service_path = std::path::Path::new("/etc/systemd/system/iroh-ssh-server.service");
    std::fs::write(service_path, service_raw)?;

    // check if service is started and running and print status
    let status = Command::new("systemctl")
        .arg("is-active")
        .arg("iroh-ssh-server.service")
        .output()
        .await?;

    if status.status.success() {
        println!("Service is already running");
    } else {
        println!("Starting service...");
        Command::new("systemctl")
            .arg("enable")
            .arg("iroh-ssh-server.service")
            .output()
            .await?;
        Command::new("systemctl")
            .arg("start")
            .arg("iroh-ssh-server.service")
            .output()
            .await?;
    }

    Ok(())
}

async fn server_mode(ssh_port: u16, persist: bool) -> anyhow::Result<()> {
    let mut iroh_ssh_builder = IrohSsh::new().accept_incoming(true).accept_port(ssh_port);
    if persist {
        iroh_ssh_builder = iroh_ssh_builder.dot_ssh_integration(true);
    }
    let iroh_ssh = iroh_ssh_builder.build().await?;

    println!("Connect to this this machine:");
    println!("\n  iroh-ssh my-user@{}\n", iroh_ssh.node_id());
    if persist {
        println!("  (using persistent keys in ~/.ssh/irohssh_ed25519)");
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

async fn client_mode(target: String) -> anyhow::Result<()> {
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
