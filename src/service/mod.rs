
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "windows")]
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
    #[cfg(target_os = "windows")]
    {
        let res = windows::install_service(service_params).await;
        println!("{:?}", res);
        return res
    }
    
    anyhow::bail!("Service mode is only supported on linux")
}
