
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::install_service;

#[derive(Debug, Clone)]
pub struct ServiceParams {
    pub ssh_port: u16,
}

pub fn install_service(service_params: ServiceParams) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    {
        return linux::install_service(service_params)
    }
    anyhow::bail!("Service mode is only supported on linux")
}
