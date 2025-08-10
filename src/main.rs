use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, command};
use iroh_ssh::api::{self, ClientOptions};

#[derive(Parser)]
#[command(
    name = "irohssh",
    about = "SSH without IP",
    after_help = "
Server:
  // Start server with persistent keys in home directory (~/.ssh/irohssh_ed25519)
  iroh-ssh server --persist

  // Start server with ephemeral keys (useful for testing or short-lived connections)
  iroh-ssh server

Connect:
  // Connect to server
  iroh-ssh -i ~/.ssh/id_rsa_my_cert my-user@6598395384059bf969...
  iroh-ssh connect my-user@6598395384059bf969...

Service:
  // Install as service (linux and windows only, uses persistent keys)
  iroh-ssh service install

  // Uninstall service
  iroh-ssh service uninstall

Info:
  // Display connection information if keys are persisted
  iroh-ssh info
"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    target: Option<String>,
    #[command(flatten)]
    implicit_connect: ConnectArgs,
}

#[derive(Subcommand)]
enum Commands {
    #[command(
        about = "Connect to a remote server - iroh-ssh `connect` my-user@NODE_ID (`connect` is optional)"
    )]
    Connect {
        target: String,
        #[command(flatten)]
        args: ConnectArgs,
    },
    #[command(about = "Run as server (for example in a tmux session)")]
    Server {
        #[arg(long, default_value = "22")]
        ssh_port: u16,
        #[arg(short, long, default_value = "false")]
        persist: bool,
    },
    #[command(about = "Manage service (linux and windows only, uses persistent keys)")]
    Service {
        #[command(subcommand)]
        service_command: ServiceCommands,
    },
    #[command(about = "Display connection information")]
    Info {},
}

#[derive(Args)]
struct ConnectArgs {
    #[arg(short, long)]
    identity_file: Option<PathBuf>,
    #[arg(short = 'L', long)]
    local_forward: Option<String>,
    #[arg(short = 'R', long)]
    remote_forward: Option<String>,
}

impl ConnectArgs {
    fn into_client_options(self, target: String) -> ClientOptions {
        ClientOptions {
            target,
            identity_file: self.identity_file,
            local_forward: self.local_forward,
            remote_forward: self.remote_forward,
        }
    }
}

#[derive(Subcommand)]
enum ServiceCommands {
    #[command(about = "Run as service (linux and windows only, uses persistent keys)")]
    Install {
        #[arg(long, default_value = "22")]
        ssh_port: u16,
    },
    Uninstall {},
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match (cli.command, cli.target) {
        (Some(Commands::Connect { target, args }), _) => {
            api::client_mode(args.into_client_options(target)).await
        }
        (Some(Commands::Server { ssh_port, persist }), _) => {
            api::server_mode(ssh_port, persist).await
        }
        (Some(Commands::Service { service_command }), _) => match service_command {
            ServiceCommands::Install { ssh_port } => api::service::install(ssh_port).await,
            ServiceCommands::Uninstall {} => api::service::uninstall().await,
        },
        (Some(Commands::Info {}), _) => api::info_mode().await,
        (None, Some(target)) => api::client_mode(cli.implicit_connect.into_client_options(target)).await,
        (None, None) => {
            anyhow::bail!("Please provide a target or use the 'connect' subcommand")
        }
    }
}
