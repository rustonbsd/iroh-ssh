use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, command};
use iroh_ssh::api::{self, ClientOptions};

const TARGET_HELP: &str = "The host to connect to";

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
  iroh-ssh my-user@6598395384059bf969...
  or 
  iroh-ssh connect ...

  // Identity file
  iroh-ssh -i ~/.ssh/id_rsa_my_cert ...

  // Tunneling
  iroh-ssh -L localhost:8000:123.45.67.89:9000 ...
  iroh-ssh -R 123.45.67.89:9000:localhost:8000 ...

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
    cli_command: Option<Commands>,
    #[arg(help = TARGET_HELP)]
    target: Option<String>,
    #[command(flatten)]
    ssh_args: ConnectArgs,
    #[arg(help = "Command to be executed on the target", trailing_var_arg = true)]
    execute_command: Option<Vec<String>>,
}

#[derive(Subcommand)]
enum Commands {
    #[command(
        about = "Connect to a remote server - iroh-ssh `connect` my-user@NODE_ID [command (optional)]"
    )]
    Connect {
        #[arg(help = TARGET_HELP)]
        target: String,
        #[command(flatten)]
        ssh_args: ConnectArgs,
        #[arg(help = "Command to be executed on the target")]
        execute_command: Vec<String>,
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
    #[arg(
        short = 'i',
        long,
        help = "Selects a file from which the identity (private key) for RSA or DSA authentication is read."
    )]
    identity_file: Option<PathBuf>,
    #[arg(
        short = 'L',
        long,
        help = "[bind_address:]port:host:hostport - Specifies that the given port on the local (client) host is to be forwarded to the given host and port on the remote side. Only the superuser can forward privileged ports. By default, the local port is bound in accordance with the GatewayPorts setting."
    )]
    local_forward: Option<String>,
    #[arg(
        short = 'R',
        long,
        help = "[bind_address:]port:host:hostport - Specifies that the given port on the remote (server) host is to be forwarded to the given host and port on the local side. Specifying a remote bind_address will only succeed if the server's GatewayPorts option is enabled in the ssh server config."
    )]
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

    match (cli.cli_command, cli.target) {
        (
            Some(Commands::Connect {
                target,
                ssh_args,
                execute_command,
            }),
            _,
        ) => api::client_mode(ssh_args.into_client_options(target), execute_command).await,
        (Some(Commands::Server { ssh_port, persist }), _) => {
            api::server_mode(ssh_port, persist).await
        }
        (Some(Commands::Service { service_command }), _) => match service_command {
            ServiceCommands::Install { ssh_port } => api::service::install(ssh_port).await,
            ServiceCommands::Uninstall {} => api::service::uninstall().await,
        },
        (Some(Commands::Info {}), _) => api::info_mode().await,
        (None, Some(target)) => {
            api::client_mode(
                cli.ssh_args.into_client_options(target),
                cli.execute_command.unwrap_or_default(),
            )
            .await
        }
        (None, None) => {
            anyhow::bail!("Please provide a target or use the 'connect' subcommand")
        }
    }
}
