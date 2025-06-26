use std::str::FromStr;

use clap::{Parser, Subcommand, command};
use iroh::{NodeId, SecretKey};
use iroh_ssh::{dot_ssh, IrohSsh};
use tokio::process::Command;

#[derive(Parser)]
#[command(name = "irohssh")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    target: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    Connect {
        target: String,
    },
    Server {
        #[arg(long, default_value = "22")]
        ssh_port: u16,
    },
    Service {
        #[arg(long, default_value = "22")]
        ssh_port: u16,
    },
    Info {}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match (cli.command, cli.target) {
        (Some(Commands::Connect { target }), _) => client_mode(target).await,
        (Some(Commands::Server { ssh_port }), _) => server_mode(ssh_port).await,
        (Some(Commands::Service { ssh_port }), _) => service_mode(ssh_port).await,
        (Some(Commands::Info { }),_) => info_mode().await,
        (None, Some(target)) => client_mode(target).await,
        (None, None) => {
            anyhow::bail!("Please provide a target or use 'connect' subcommand")
        }
    }
}

async fn info_mode() -> anyhow::Result<()> {
    let key = dot_ssh(&SecretKey::generate(rand::rngs::OsRng))?;

    println!("iroh-ssh version 0.1.3");
    println!("https://github.com/rustonbsd/iroh-ssh");
    println!("");
    println!("run 'iroh-ssh server' to start the server");
    println!("run 'iroh-ssh service' to start the server as a service");
    println!("");
    println!("Your iroh-ssh nodeid:");
    println!("  iroh-ssh root@{}\n\n", z32::encode(key.public().as_bytes()));
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
        .output().await?;

    if status.status.success() {
        println!("Service is already running");
    } else {
        println!("Starting service...");
        Command::new("systemctl")
            .arg("start")
            .arg("iroh-ssh-server.service")
            .output().await?;
    }

    Ok(())
}

async fn server_mode(ssh_port: u16) -> anyhow::Result<()> {
    let iroh_ssh = IrohSsh::new()
        .accept_incoming(true)
        .accept_port(ssh_port)
        .dot_ssh_integration()
        .build()
        .await?;
    println!("Connect to this this machine:\n\n");
    println!("\niroh-ssh root@{}\n\n", iroh_ssh.node_id());
    println!("where root is the username you want to connect to.");
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
    let iroh_ssh = IrohSsh::new().accept_incoming(true).build().await?;
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
