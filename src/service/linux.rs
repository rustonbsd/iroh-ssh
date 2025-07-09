use tokio::process::Command;

use crate::ServiceParams;
use crate::Service;

#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct LinuxService;

#[cfg(target_os = "linux")]
impl Service for LinuxService {
    async fn install(service_params: ServiceParams) -> anyhow::Result<()> {
        let service_raw = r#"[Unit]
Description=SSH over Iroh

[Service]
Type=simple
WorkingDirectory=~
ExecStart=/bin/bash -c 'iroh-ssh server -p --ssh-port [SSHPORT]'
Restart=on-failure
RestartSec=3s

[Install]
WantedBy=multi-user.target
"#;

        let service_raw = service_raw.replace("[SSHPORT]", &service_params.ssh_port.to_string());

        let service_path = std::path::Path::new("/etc/systemd/system/iroh-ssh-server.service");
        std::fs::write(service_path, service_raw)?;

        // check if service is started and running and print status
        let status = Command::new("systemctl")
            .arg("is-active")
            .arg("iroh-ssh-server.service")
            .output()
            .await?;

        if status.status.success() {
            println!("Service is already running");
        } else {
            println!("Starting service...");
            Command::new("systemctl")
                .arg("enable")
                .arg("iroh-ssh-server.service")
                .output()
                .await?;
            Command::new("systemctl")
                .arg("start")
                .arg("iroh-ssh-server.service")
                .output()
                .await?;
        }

        Ok(())
    }

    async fn info() -> anyhow::Result<()> {
        todo!("service info is not yet supported")
    }

    async fn uninstall() -> anyhow::Result<()> {
        todo!("uninstalling service is not yet supported")
    }
}
