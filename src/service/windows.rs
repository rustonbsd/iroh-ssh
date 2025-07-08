use crate::ServiceParams;

#[cfg(target_os = "windows")]
pub async fn install_service(params: ServiceParams) -> anyhow::Result<()> {
    service::install(params)?;

    Ok(())
}

#[cfg(windows)]
mod service {

    use crate::ServiceParams;
    use std::{io::Write as _};

    use anyhow::Context;
    use runas::Command;

    const SERVICE_NAME: &str = "iroh-ssh";
    const PROFILE_SERVICE_DIR: &str = "C:\\Windows\\ServiceProfiles\\iroh-ssh";
    const BINARY_DIR: &str = "C:\\ProgramData\\iroh-ssh";

    const NSSM_BYTES: &[u8] = include_bytes!("../../win/nssm.exe");

    fn init_nssm() -> anyhow::Result<std::path::PathBuf> {
        let mut temp_exe = tempfile::Builder::new()
            .prefix("nssm-")
            .suffix(".exe")
            .tempfile()?;
        temp_exe.write_all(NSSM_BYTES)?;
        temp_exe.flush()?;
        let path = temp_exe.path().to_path_buf();
        temp_exe.keep()?;

        
        Ok(path)
    }

    pub fn install(service_params: ServiceParams) -> anyhow::Result<()> {
        let nssm_path = init_nssm()?;

        if Command::new(&nssm_path).args(&["status", SERVICE_NAME]).gui(false).force_prompt(true).status()?.code() == Some(0) {
            println!("Service is already installed");
            return Ok(())
        }

        let profile_ssh_dir = std::path::Path::new(PROFILE_SERVICE_DIR)
            .join(".ssh");
        let service_binary_dir = std::path::Path::new(BINARY_DIR);


        std::fs::create_dir_all(&profile_ssh_dir)
            .with_context(|| format!("Failed to create service directory at {:?}", &profile_ssh_dir))?;
        std::fs::create_dir_all(&service_binary_dir)
            .with_context(|| format!("Failed to create binary directory {:?}", &service_binary_dir))?;

        std::fs::copy(std::env::current_exe()?, service_binary_dir.join(format!("{}.exe", SERVICE_NAME)))?;

        /*
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
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["install", SERVICE_NAME, &format!("{BINARY_DIR}\\{SERVICE_NAME}.exe")]).gui(false).force_prompt(true).status()?);
        println!("2");
        println!("Setting AppParameters...");
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "AppParameters", &format!("server --persist --ssh-port {}", &service_params.ssh_port)]).gui(false).force_prompt(true).status()?);

        println!("Setting AppDirectory...");
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "AppDirectory",  BINARY_DIR]).gui(false).force_prompt(true).status()?);

        println!("Setting AppExit...");
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "AppExit", "Default","Restart"]).gui(false).force_prompt(true).status()?);

        println!("Setting AppPriority...");
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "AppPriority", "HIGH_PRIORITY_CLASS"]).gui(false).force_prompt(true).status()?);

        println!("Setting AppStdout...");
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "AppStdout", &format!("{BINARY_DIR}\\{SERVICE_NAME}.log")]).gui(false).force_prompt(true).status()?);

        println!("Setting AppStderr...");
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "AppStderr", &format!("{BINARY_DIR}\\{SERVICE_NAME}.error.log")]).gui(false).force_prompt(true).status()?);

        println!("Setting AppTimestampLog...");
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "AppTimestampLog", "1"]).gui(false).force_prompt(true).status()?);

        println!("Setting DependOnService...");
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "DependOnService", ":sshd"]).gui(false).force_prompt(true).status()?);

        println!("Setting Description...");
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "Description", "ssh without ip"]).gui(false).force_prompt(true).status()?);

        println!("Setting DisplayName...");
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "DisplayName", SERVICE_NAME]).gui(false).force_prompt(true).status()?);

        println!("Setting Start...");
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "Start", "SERVICE_AUTO_START"]).gui(false).force_prompt(true).status()?);

        println!("Setting Type...");
        println!("{:#?}", Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "Type", "SERVICE_WIN32_OWN_PROCESS"]).gui(false).force_prompt(true).status()?);

        println!("All NSSM configuration commands completed.");

        println!("Starting service...");
        let start_result = Command::new("powershell")
            .args(&["-Command", &format!("Start-Service -Name '{}'", SERVICE_NAME)])
            .gui(false).force_prompt(true).status()?;
        println!("Start-Service output: {:#?}", start_result);

        // Also check service status
        println!("Checking service status...");
        let status_result = Command::new("powershell")
            .args(&["-Command", &format!("Get-Service -Name '{}'", SERVICE_NAME)])
            .gui(false).force_prompt(true).status()?;
        println!("Get-Service output: {:#?}", status_result);

        
        println!("Setting ObjectName (Service Account)...");
        println!("{:#?}", std::process::Command::new(nssm_path.clone()).args(&["set", SERVICE_NAME, "ObjectName", &format!("NT SERVICE\\{SERVICE_NAME}")]).output()?);


        println!("Service installation process completed.");
    
        Ok(())
    }
}