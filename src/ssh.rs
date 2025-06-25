macro_rules! ok_or_continue {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(_) => continue,
        }
    };
}

use std::{pin::Pin};

use crate::{Builder, Inner, IrohSsh};

use ed25519_dalek::{SECRET_KEY_LENGTH};
use iroh::{
    endpoint::{Connection}, protocol::{ProtocolHandler, Router}, Endpoint, NodeId, SecretKey
};
use tokio::{
    
    net::TcpStream,
};

impl Builder {
    pub fn new() -> Self {
        Self {
            secret_key: SecretKey::generate(rand::rngs::OsRng).to_bytes(),
            accept_incoming: false,
            accept_port: None,
        }
    }

    pub fn accept_incoming(mut self, accept_incoming: bool) -> Self {
        self.accept_incoming = accept_incoming;
        self
    }

    pub fn accept_port(mut self, accept_port: u16) -> Self {
        self.accept_port = Some(accept_port);
        self
    }

    pub fn secret_key(mut self, secret_key: &[u8; SECRET_KEY_LENGTH]) -> Self {
        self.secret_key = *secret_key;
        self
    }

    pub async fn build(self: &mut Self) -> anyhow::Result<IrohSsh> {
        // Iroh setup
        let secret_key = SecretKey::from_bytes(&self.secret_key);
        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .discovery_n0()
            .bind()
            .await?;

        let iroh_ssh = IrohSsh {
            public_key: endpoint.node_id().as_bytes().clone(),
            secret_key: self.secret_key,
            inner: None,
        };

        let router = if self.accept_incoming {
            Router::builder(endpoint.clone()).accept(&IrohSsh::ALPN(), iroh_ssh.clone())
        } else {
            Router::builder(endpoint.clone())
        }
        .spawn();

        iroh_ssh.clone().add_inner(endpoint, router);

        if self.accept_incoming && self.accept_port.is_some() {
            tokio::spawn({
                let iroh_ssh = iroh_ssh.clone();
                let accept_port = self.accept_port.expect("accept_port not set");
                async move {
                    iroh_ssh._spawn(accept_port).await.expect("spawn failed");
                }
            });
        }

        Ok(iroh_ssh)
    }
}

impl IrohSsh {
    pub fn new() -> Builder {
        Builder::new()
    }

    #[allow(non_snake_case)]
    pub fn ALPN() -> Vec<u8> {
        format!("/iroh/ssh").into_bytes()
    }

    fn add_inner(&mut self, endpoint: Endpoint, router: Router) {
        self.inner = Some(Inner { endpoint, router });
    }

    pub async fn connect(&self, node_id: NodeId) -> anyhow::Result<Connection> {
        let inner = self.inner.as_ref().expect("inner not set");
        inner.endpoint.connect(node_id, &IrohSsh::ALPN()).await
    }

    pub fn node_id(&self) -> NodeId {
        self.inner.as_ref().expect("inner not set").endpoint.node_id()
    }

    async fn _spawn(self, port: u16) -> anyhow::Result<()> {
        while let Some(incoming) = self
            .inner
            .clone()
            .expect("inner not set")
            .endpoint
            .accept()
            .await
        {
            let mut connecting = match incoming.accept() {
                Ok(connecting) => connecting,
                Err(err) => {
                    println!("incomming connection failure: {err:#}");
                    continue;
                }
            };
            let alpn = ok_or_continue!(connecting.alpn().await);

            let conn = ok_or_continue!(connecting.await);
            let node_id = ok_or_continue!(conn.remote_node_id());
            println!("{}: {node_id} incoming...", String::from_utf8_lossy(&alpn),);

            tokio::spawn(async move {
                if let Ok((mut send, mut recv)) = conn.accept_bi().await {
                    if let Ok(mut ssh_stream) = TcpStream::connect(format!("127.0.0.1:{}", port)).await {
                    
                    let (mut local_read, mut local_write) = ssh_stream.split();
                    let a_to_b = async move { tokio::io::copy(&mut local_read, &mut send).await };
                    let b_to_a = async move { tokio::io::copy(&mut recv, &mut local_write).await };

                    tokio::select! {
                        result = a_to_b => { let _ = result; },
                        result = b_to_a => { let _ = result; },
                    };
                }};
            });
        }
        Ok(())
    }
}

impl ProtocolHandler for IrohSsh {
    fn accept(
        &self,
        conn: Connection,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'static>> {
        let iroh_ssh = self.clone();

        Box::pin(async move {
            iroh_ssh.accept(conn).await?;
            Ok(())
        })
    }
}
