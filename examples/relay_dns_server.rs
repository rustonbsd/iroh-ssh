use std::net::Ipv4Addr;

use iroh::RelayUrl;
use iroh_relay::{
    RelayQuicConfig,
    server::{AccessConfig, CertConfig, QuicConfig, RelayConfig, Server, ServerConfig, TlsConfig},
};

async fn relay_server() -> anyhow::Result<iroh_relay::server::Server> {
    let (certs, server_config) = iroh_relay::server::testing::self_signed_tls_certs_and_config();

    let tls = TlsConfig {
        cert: CertConfig::<(), ()>::Manual { certs },
        https_bind_addr: (Ipv4Addr::LOCALHOST, 0).into(),
        quic_bind_addr: (Ipv4Addr::LOCALHOST, 0).into(),
        server_config,
    };
    let quic = QuicConfig {
        server_config: tls.server_config.clone(),
        bind_addr: tls.quic_bind_addr,
    };
    let config = ServerConfig {
        relay: Some(RelayConfig {
            http_bind_addr: (Ipv4Addr::LOCALHOST, 0).into(),
            tls: Some(tls),
            limits: Default::default(),
            key_cache_capacity: Some(1024),
            access: AccessConfig::Everyone,
        }),
        quic: Some(quic),

        ..Default::default()
    };
    let server = Server::spawn(config).await?;
    let url: RelayUrl = format!("https://{}", server.https_addr().expect("configured"))
        .parse()
        .expect("invalid relay url");

    let quic = server
        .quic_addr()
        .map(|addr| RelayQuicConfig { port: addr.port() });

    println!("Relay server running at {url}");
    if let Some(quic) = &quic {
        println!("QUIC enabled on port {}", quic.port);
    }

    Ok(server)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let relay = relay_server().await?;

    tokio::signal::ctrl_c().await?;
    relay.shutdown().await?;
    Ok(())
}
