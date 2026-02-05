use std::{ffi::OsString, path::PathBuf};

use clap::{ArgAction, Args, Parser, Subcommand};

const TARGET_HELP: &str = "Target in the form user@ENDPOINT_ID";

#[derive(Parser, Debug)]
#[command(name = "iroh-ssh", about = "ssh without ip")]
pub struct Cli {
    #[command(subcommand)]
    pub cmd: Option<Cmd>,

    #[arg(help = TARGET_HELP)]
    pub target: Option<String>,

    #[command(flatten)]
    pub ssh: SshOpts,

    #[arg(trailing_var_arg = true)]
    pub remote_cmd: Option<Vec<OsString>>,
}

#[derive(Subcommand,Debug)]
pub enum Cmd {
    Connect(ConnectArgs),
    #[command(hide = true)]
    Exec(ExecArgs),
    Server(ServerArgs),
    Service {
        #[command(subcommand)]
        op: ServiceCmd,
    },
    Info,
    #[command(hide = true)]
    Proxy(ProxyArgs),
    #[command(hide = true)]
    RunService(ServiceArgs),
    Version,
}

#[derive(Args, Clone, Debug)]
pub struct ProxyArgs {
    #[arg(help = "Proxy Endpoint ID")]
    pub endpoint_id: String,
}

#[derive(Args, Clone, Debug)]
pub struct ConnectArgs {
    #[arg(help = TARGET_HELP)]
    pub target: String,

    #[command(flatten)]
    pub ssh: SshOpts,

    #[arg(trailing_var_arg = true)]
    pub remote_cmd: Vec<OsString>,
}

#[derive(Args, Clone, Debug)]
pub struct ExecArgs {
    #[arg(help = TARGET_HELP)]
    pub target: String,

    #[command(flatten)]
    pub ssh: SshOpts,

    #[arg(trailing_var_arg = true, required = true)]
    pub remote_cmd: Vec<OsString>,
}

#[derive(Args, Clone, Default, Debug)]
pub struct SshOpts {
    #[arg(
        short = 'i',
        long,
        value_name = "PATH",
        help = "Identity file for publickey auth"
    )]
    pub identity_file: Option<PathBuf>,

    #[arg(short = 'L', value_name = "LPORT:HOST:RPORT",
        help = "Local forward [bind_addr:]lport:host:rport (host can't be endpoint_id yet)", action = ArgAction::Append)]
    pub local_forward: Vec<String>,

    #[arg(short = 'R', value_name = "RPORT:HOST:LPORT",
        help = "Remote forward [bind_addr:]rport:host:lport  (host can't be endpoint_id yet)", action = ArgAction::Append)]
    pub remote_forward: Vec<String>,

    #[arg(
        short = 'p',
        long,
        value_name = "PORT",
        help = "Remote sshd port (default 22)"
    )]
    pub port: Option<u16>,

    #[arg(short = 'o', value_name = "KEY=VALUE",
        help = "Pass an ssh option (repeatable)", action = ArgAction::Append)]
    pub options: Vec<String>,

    #[arg(short = 'A', help = "Enable agent forwarding", action = ArgAction::SetTrue)]
    pub agent: bool,

    #[arg(short = 'a', help = "Disable agent forwarding", action = ArgAction::SetTrue)]
    pub no_agent: bool,

    #[arg(short = 'X', help = "Enable X11 forwarding", action = ArgAction::SetTrue)]
    pub x11: bool,

    #[arg(short = 'Y', help = "Enable trusted X11 forwarding", action = ArgAction::SetTrue)]
    pub x11_trusted: bool,

    #[arg(short = 'N', help = "Do not execute remote command", action = ArgAction::SetTrue)]
    pub no_cmd: bool,

    #[arg(short = 't', help = "Force pseudo-terminal", action = ArgAction::SetTrue)]
    pub force_tty: bool,

    #[arg(short = 'T', help = "Disable pseudo-terminal", action = ArgAction::SetTrue)]
    pub no_tty: bool,

    #[arg(short = 'v', help = "Increase verbosity",
        action = ArgAction::Count)]
    pub verbose: u8,

    #[arg(short = 'q', help = "Quiet mode", action = ArgAction::SetTrue)]
    pub quiet: bool,
}

#[derive(Args, Clone, Debug)]
pub struct ServerArgs {
    #[arg(long, default_value = "22")]
    pub ssh_port: u16,

    #[arg(short, long, default_value_t = false)]
    pub persist: bool,
}

#[derive(Subcommand, Clone, Debug)]
pub enum ServiceCmd {
    Install {
        #[arg(long, default_value = "22")]
        ssh_port: u16,
    },
    Uninstall,
}

#[derive(Args, Clone, Debug)]
pub struct ServiceArgs {
    #[arg(long, default_value = "22")]
    pub ssh_port: u16,
}
