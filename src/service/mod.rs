
#[cfg(target_os = "linux")]
mod linux;

mod windows;

#[derive(Debug, Clone)]
pub struct ServiceParams {
    pub ssh_port: u16,
}

pub async fn install_service(service_params: ServiceParams) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        return linux::install_service(service_params).await
    }
    anyhow::bail!("Service mode is only supported on linux")
}
