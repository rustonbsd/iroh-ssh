use clap::Parser;
use iroh_ssh::{Cli, Cmd, ConnectArgs, ServiceCmd, api};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.cmd {
        Some(Cmd::Connect(args)) => api::client_mode(args).await,
        Some(Cmd::Exec(args)) => {
            let conn_args = ConnectArgs {
                ssh: args.ssh,
                remote_cmd: args.remote_cmd,
                target: args.target,
            };
            api::client_mode(conn_args).await
        }
        Some(Cmd::Server(args)) => api::server_mode(args).await,
        Some(Cmd::Service { op }) => match op {
            ServiceCmd::Install { ssh_port } => api::service::install(ssh_port).await,
            ServiceCmd::Uninstall => api::service::uninstall().await,
        },
        Some(Cmd::Info) => api::info_mode().await,
        Some(Cmd::Proxy(args)) => {
            api::proxy_mode(args).await
        }
        None => {
            let conn_args = ConnectArgs {
                ssh: cli.ssh,
                remote_cmd: cli.remote_cmd.unwrap_or_default(),
                target: cli.target.expect("target is required"),
            };
            api::client_mode(conn_args).await
        }
    }
}
