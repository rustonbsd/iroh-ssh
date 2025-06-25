use std::{process::Stdio, str::FromStr};

use clap::{Parser, Subcommand, command};
use iroh::NodeId;
use iroh_ssh::IrohSsh;
use tokio::process::Command;


#[derive(Parser)]
#[command(name = "irohssh")]
#[command(about = "SSH over iroh tunnels")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Connect { target } => client_mode(target).await,
        Commands::Server { ssh_port } => server_mode(ssh_port).await,
    }
}

async fn server_mode(ssh_port: u16) -> anyhow::Result<()> {
    let iroh_ssh = IrohSsh::new().accept_incoming(true).accept_port(ssh_port).build().await?;
    println!("Connect to this this machine:\n\n");
    println!("      irohssh root@{}\n\n", iroh_ssh.node_id());
    println!("where root is the username you want to connect to.");
    println!("");
    println!("client -> irohssh -> direct connect -> irohssh -> local ssh :{}", ssh_port);
    
    println!("Waiting for incoming connections...");
    println!("Press Ctrl+C to exit");
    tokio::signal::ctrl_c().await?;
    Ok(())
}

async fn client_mode(target: String) -> anyhow::Result<()> {
    let (ssh_user, iroh_node_id) = parse_iroh_target(&target)?;

    // Start SSH process
    let mut ssh_process = Command::new("ssh")
        .arg(format!("{}@dummy", ssh_user))
        .arg("-o")
        .arg("StrictHostKeyChecking=no")
        .arg("-o")
        .arg("UserKnownHostsFile=/dev/null")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;

    let mut ssh_stdin =  ssh_process.stdin.take().unwrap();
    let mut ssh_stdout =  ssh_process.stdout.take().ok_or_else(|| anyhow::anyhow!("Failed to get stdout"))?;

    // Create iroh connection
    let iroh_ssh = IrohSsh::new().accept_incoming(false).build().await?;
    let (mut iroh_write, mut iroh_read) = iroh_ssh.connect(iroh_node_id).await?.accept_bi().await?;


    tokio::spawn(async move {
        let a_to_b = async move { tokio::io::copy(&mut iroh_read, &mut ssh_stdin).await };
        let b_to_a = async move { tokio::io::copy(&mut ssh_stdout, &mut iroh_write).await };

        tokio::select! {
            result = a_to_b => { let _ = result; },
            result = b_to_a => { let _ = result; },
        };
    });
    Ok(())
}

fn parse_iroh_target(target: &str) -> anyhow::Result<(String, NodeId)> {
    let (user, node_id_str) = target
        .split_once('@')
        .ok_or_else(|| anyhow::anyhow!("Invalid format, use user@node_id"))?;
    let node_id = NodeId::from_str(node_id_str)?;
    Ok((user.to_string(), node_id))
}
