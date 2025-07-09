use clap::{Parser, Subcommand, command};
use iroh_ssh::api;

#[derive(Parser)]
#[command(
    name = "irohssh",
    about = "SSH without IP",
    after_help = "
Usage Examples:
  iroh-ssh server --persist                // Start server with persistent keys
  iroh-ssh my-user@6598395384059bf969...   // Connect to server
  iroh-ssh service                         // Linux only
"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    target: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(
        about = "Connect to a remote server - iroh-ssh `connect` my-user@NODE_ID (`connect` is optional)"
    )]
    Connect { target: String },
    #[command(about = "Run as server (for exampel in a tmux session)")]
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
        (Some(Commands::Connect { target }), _) => api::client_mode(target).await,
        (Some(Commands::Server { ssh_port, persist }), _) => {
            api::server_mode(ssh_port, persist).await
        }
        (Some(Commands::Service { service_command }), _) => match service_command {
            ServiceCommands::Install { ssh_port } => api::service::install(ssh_port).await,
            ServiceCommands::Uninstall {} => {
                api::service::uninstall().await
            }
        },
        (Some(Commands::Info {}), _) => api::info_mode().await,
        (None, Some(target)) => api::client_mode(target).await,
        (None, None) => {
            anyhow::bail!("Please provide a target or use 'connect' subcommand")
        }
    }
}
