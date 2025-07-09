use crate::{Service, ServiceParams};

use std::io::Write as _;

use anyhow::Context;
#[cfg(target_os = "windows")]
use runas::Command;

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub struct WindowsService;

#[cfg(target_os = "windows")]
impl Service for WindowsService {
    async fn install(service_params: ServiceParams) -> anyhow::Result<()> {
        /* https://github.com/rustonbsd/iroh-ssh/issues/5
        # nssm making a decent Windows service
        nssm.exe install iroh-ssh C:\ProgramData\iroh-ssh\iroh-ssh.exe
        nssm.exe set iroh-ssh AppParameters server
        nssm.exe set iroh-ssh AppDirectory C:\ProgramData\iroh-ssh
        nssm.exe set iroh-ssh AppExit Default Restart
        nssm.exe set iroh-ssh AppPriority HIGH_PRIORITY_CLASS
        nssm.exe set iroh-ssh AppStdout C:\ProgramData\iroh-ssh\iroh-ssh.log
        nssm.exe set iroh-ssh AppStderr C:\ProgramData\iroh-ssh\iroh-ssh.error.log
        nssm.exe set iroh-ssh AppTimestampLog 1
        nssm.exe set iroh-ssh DependOnService :sshd
        nssm.exe set iroh-ssh Description "SSHD over Iroh"
        nssm.exe set iroh-ssh DisplayName iroh-ssh

        nssm.exe set iroh-ssh ObjectName "NT Service\iroh-ssh"
        nssm.exe set iroh-ssh Start SERVICE_AUTO_START
        nssm.exe set iroh-ssh Type SERVICE_WIN32_OWN_PROCESS
         */

        let nssm_path = WindowsService::init_nssm()?;
        let ps1_script = WindowsService::INSTALL_PS1_BYTES
            .replace("[__SERVICE_NAME__]", WindowsService::SERVICE_NAME)
            .replace("[__BINARY_DIR__]", WindowsService::BINARY_DIR)
            .replace(
                "[__NSSM_PATH__]",
                &nssm_path
                    .to_str()
                    .with_context(|| "failed to get nssm path")?,
            )
            .replace(
                "[__CURRENT_EXE_PATH__]",
                &std::env::current_exe()?
                    .to_str()
                    .with_context(|| "failed to get current exe path")?,
            )
            .replace("[__SSH_PORT__]", &service_params.ssh_port.to_string());

        // NOTE: RUNNING AS LOCAL USER NOT VIRTUAL SERVICE ACCOUNT
        // for some reason: .ssh folder in C:\WINDOWS\system32\config\systemprofile\.ssh
            Command::new("powershell")
                .args(&["-ExecutionPolicy", "Bypass", "-Command", &ps1_script])
                .show(false)
                .status()
                .with_context(|| "failed to install service")
                .map(|_| ())?;

        Ok(())
    }
    async fn info() -> anyhow::Result<()> {
        todo!("service info is not yet supported")
    }

    async fn uninstall() -> anyhow::Result<()> {
        let nssm_path = WindowsService::init_nssm()?;
        let ps1_script = WindowsService::UNINSTALL_PS1_BYTES
            .replace("[__SERVICE_NAME__]", WindowsService::SERVICE_NAME)
            .replace("[__BINARY_DIR__]", WindowsService::BINARY_DIR)
            .replace(
                "[__NSSM_PATH__]",
                &nssm_path
                    .to_str()
                    .with_context(|| "failed to get nssm path")?,
            )
            .replace(
                "[__CURRENT_EXE_PATH__]",
                &std::env::current_exe()?
                    .to_str()
                    .with_context(|| "failed to get current exe path")?,
            );

        Command::new("powershell")
            .args(&["-ExecutionPolicy", "Bypass", "-Command", &ps1_script])
            .show(false)
            .status()
            .with_context(|| "failed to uninstall service")
            .map(|_| ())
    }
}

#[cfg(windows)]
impl WindowsService {
    const SERVICE_NAME: &str = "iroh-ssh";
    const BINARY_DIR: &str = "C:\\ProgramData\\iroh-ssh";

    const NSSM_BYTES: &[u8] = include_bytes!("../../win/nssm.exe");
    const INSTALL_PS1_BYTES: &str = include_str!("../../win/install.ps1");
    const UNINSTALL_PS1_BYTES: &str = include_str!("../../win/uninstall.ps1");

    fn init_nssm() -> anyhow::Result<std::path::PathBuf> {
        let mut temp_exe = tempfile::Builder::new()
            .prefix("nssm-")
            .suffix(".exe")
            .tempfile_in("C:\\Windows\\Temp")?;
        temp_exe.write_all(WindowsService::NSSM_BYTES)?;
        let nssm_path = temp_exe.path().to_path_buf();
        temp_exe.keep()?;

        Ok(nssm_path)
    }
}
