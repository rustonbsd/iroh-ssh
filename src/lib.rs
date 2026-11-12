mod cli;
mod service;
mod ssh;

use std::path::PathBuf;

use ed25519_dalek::{PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use iroh::{Endpoint, RelayUrl, protocol::Router};

pub mod api;

pub use cli::*;
pub use service::Service;
pub use service::ServiceParams;
pub use service::{install_service, run_service, uninstall_service};
pub use ssh::dot_ssh;
use tokio::io;

#[derive(Debug, Clone)]
pub struct IrohSsh {
    #[allow(dead_code)]
    pub(crate) secret_key: [u8; SECRET_KEY_LENGTH],
    #[allow(dead_code)]
    pub(crate) public_key: [u8; PUBLIC_KEY_LENGTH],
    pub(crate) inner: Option<Inner>,
    pub(crate) ssh_port: u16,
}

#[derive(Debug, Clone)]
pub(crate) struct Inner {
    pub endpoint: Endpoint,
    #[allow(dead_code)]
    pub router: Router,
}

#[derive(Debug, Clone)]
pub struct Builder {
    secret_key: [u8; SECRET_KEY_LENGTH],
    accept_incoming: bool,
    accept_port: Option<u16>,
    key_dir: Option<PathBuf>,
    relay_urls: Vec<RelayUrl>,
    extra_relay_urls: Vec<RelayUrl>,
    skip_server_probe: bool,
}

/// Resolves on Ctrl-C, or on SIGTERM, which service managers send to stop or
/// restart the server. Dying on SIGTERM without closing the endpoint would leave
/// clients hanging until their connection's idle timeout, about 30s.
async fn shutdown_signal() -> io::Result<()> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let mut terminate = signal(SignalKind::terminate())?;
        tokio::select! {
            res = tokio::signal::ctrl_c() => res,
            _ = terminate.recv() => Ok(()),
        }
    }
    #[cfg(not(unix))]
    tokio::signal::ctrl_c().await
}
