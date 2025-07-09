mod ssh;
mod service;
pub mod api;

use ed25519_dalek::{PUBLIC_KEY_LENGTH, SECRET_KEY_LENGTH};
use iroh::{protocol::Router, Endpoint};

pub use ssh::dot_ssh;
pub use service::{install_service,uninstall_service};
pub use service::ServiceParams;
pub use service::Service;


#[derive(Debug, Clone)]
pub struct IrohSsh {
    #[allow(dead_code)]
    pub(crate) secret_key: [u8; SECRET_KEY_LENGTH],
    #[allow(dead_code)]
    pub(crate) public_key: [u8; PUBLIC_KEY_LENGTH],
    pub(crate) inner: Option<Inner>,
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
}

