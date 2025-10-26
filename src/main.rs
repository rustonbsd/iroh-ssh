use clap::Parser;
use iroh_ssh::{Cli, Cmd, ConnectArgs, ServiceCmd, api};

#[cfg(not(target_os = "windows"))]
use anyhow::bail;

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
                iroh: args.iroh,
            };
            api::client_mode(conn_args).await
        }
        Some(Cmd::Server(args)) => api::server_mode(args, false).await,
        Some(Cmd::Service { op }) => {
            if !self_runas::is_elevated() {
                return self_runas::admin();
            } else {
                match op {
                    ServiceCmd::Install { ssh_port } => api::service::install(ssh_port).await,
                    ServiceCmd::Uninstall => api::service::uninstall().await,
                }
            }
        }
        Some(Cmd::Info) => api::info_mode().await,
        Some(Cmd::Proxy(args)) => api::proxy_mode(args).await,
        #[cfg(target_os = "windows")]
        Some(Cmd::RunService(args)) => iroh_ssh::run_service(args.ssh_port).await,
        #[cfg(not(target_os = "windows"))]
        Some(Cmd::RunService(_)) => {
            bail!("service runtime is only available on windows");
        }
        None => {
            let conn_args = ConnectArgs {
                ssh: cli.ssh,
                remote_cmd: cli.remote_cmd.unwrap_or_default(),
                target: cli.target.unwrap_or_default(),
                iroh: cli.iroh,
            };
            api::client_mode(conn_args).await
        }
    }
}
