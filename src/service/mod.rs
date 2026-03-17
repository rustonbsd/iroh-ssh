#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use crate::service::linux::LinuxService;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub(crate) use crate::service::windows::WindowsService;

#[cfg(target_os = "windows")]
pub async fn run_service(
    ssh_port: u16,
    key_dir: Option<std::path::PathBuf>,
    relay_url: Vec<String>,
    extra_relay_url: Vec<String>,
) -> anyhow::Result<()> {
    WindowsService::run_service(ServiceParams {
        ssh_port,
        key_dir,
        relay_url,
        extra_relay_url,
    })
    .await
}

#[cfg(not(target_os = "windows"))]
pub async fn run_service(
    _ssh_port: u16,
    _key_dir: Option<std::path::PathBuf>,
    _relay_url: Vec<String>,
    _extra_relay_url: Vec<String>,
) -> anyhow::Result<()> {
    anyhow::bail!("service run is only supported on windows");
}

#[derive(Debug, Clone)]
pub struct ServiceParams {
    pub ssh_port: u16,
    pub key_dir: Option<std::path::PathBuf>,
    pub relay_url: Vec<String>,
    pub extra_relay_url: Vec<String>,
}

pub trait Service {
    fn install(
        service_params: ServiceParams,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
    fn info() -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
    fn uninstall() -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

pub async fn install_service(_service_params: ServiceParams) -> anyhow::Result<()> {
    match std::env::consts::OS {
        #[cfg(target_os = "linux")]
        "linux" => LinuxService::install(_service_params).await,
        #[cfg(target_os = "windows")]
        "windows" => WindowsService::install(_service_params).await,
        _ => anyhow::bail!("service mode is only supported on linux and windows"),
    }
}

pub async fn uninstall_service() -> anyhow::Result<()> {
    match std::env::consts::OS {
        #[cfg(target_os = "linux")]
        "linux" => LinuxService::uninstall().await,
        #[cfg(target_os = "windows")]
        "windows" => WindowsService::uninstall().await,
        _ => anyhow::bail!("service mode is only supported on linux and windows"),
    }
}
