use crate::ServiceParams;

#[cfg(target_os = "windows")]
pub async fn install_service(params: ServiceParams) -> anyhow::Result<()> {
    service::install(params)?;

    Ok(())
}

#[cfg(windows)]
mod service {

    use crate::ServiceParams;
    use std::io::Write as _;

    use anyhow::Context;
    use runas::Command;

    const SERVICE_NAME: &str = "iroh-ssh";
    //const SERVICE_PROFILE_DIR: &str = "C:\\Windows\\ServiceProfiles\\iroh-ssh";
    const BINARY_DIR: &str = "C:\\ProgramData\\iroh-ssh";

    const NSSM_BYTES: &[u8] = include_bytes!("../../win/nssm.exe");
    const PS1_BYTES: &str = include_str!("../../win/install.ps1");

    fn init_nssm() -> anyhow::Result<std::path::PathBuf> {
        let mut temp_exe = tempfile::Builder::new()
            .prefix("nssm-")
            .suffix(".exe")
            .tempfile_in("C:\\Windows\\Temp")?;
        temp_exe.write_all(NSSM_BYTES)?;
        let nssm_path = temp_exe.path().to_path_buf();
        temp_exe.keep()?;

        Ok(nssm_path)
    }

    pub fn install(service_params: ServiceParams) -> anyhow::Result<()> {
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

        let nssm_path = init_nssm()?;
        let ps1_script = PS1_BYTES
            .replace("[__SERVICE_NAME__]", SERVICE_NAME)
            .replace("[__BINARY_DIR__]", BINARY_DIR)
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

        println!("ps: {}", ps1_script);

        // NOTE: RUNNING AS LOCAL USER NOT VIRTUAL SERVICE ACCOUNT
        println!(
            "{:?}",
            Command::new("powershell")
                .args(&["-ExecutionPolicy", "Bypass", "-Command", &ps1_script])
                .show(false)
                .status()
                .with_context(|| "failed to install service")?
        );

        Ok(())
    }
}
