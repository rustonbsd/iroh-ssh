use crate::ServiceParams;
use crate::Service;

#[cfg(target_os = "linux")]
#[derive(Debug, Clone)]
pub struct LinuxService;

#[cfg(target_os = "linux")]
impl Service for LinuxService {
    async fn install(service_params: ServiceParams) -> anyhow::Result<()> {
        let path = LinuxService::init_install_script(service_params)?;

        runas::Command::new("sh")
            .arg(path)
            .show(false)
            .force_prompt(false)
            .status()?;

        Ok(())
    }

    async fn info() -> anyhow::Result<()> {
        todo!("service info is not yet supported")
    }

    async fn uninstall() -> anyhow::Result<()> {
        let path = LinuxService::init_uninstall_script()?;

        runas::Command::new("sh")
            .arg(path)
            .show(false)
            .force_prompt(false)
            .status()?;

        Ok(())
    }
}


#[cfg(target_os = "linux")]
impl LinuxService {

    const INSTALL_SH_BYTES: &str = include_str!("../../service/install_linux.sh");
    const UNINSTALL_SH_BYTES: &str = include_str!("../../service/uninstall_linux.sh");

    fn init_install_script(service_params: ServiceParams) -> anyhow::Result<std::path::PathBuf> {
        use std::io::Write as _;

        let mut temp_sh = tempfile::Builder::new()
            .prefix("iroh_ssh_install-")
            .suffix(".sh")
            .tempfile_in("/tmp")?;
        temp_sh.write_all(LinuxService::INSTALL_SH_BYTES.replace("[SSHPORT]", &service_params.ssh_port.to_string()).as_bytes())?;
        let sh_path = temp_sh.path().to_path_buf();
        temp_sh.keep()?;

        Ok(sh_path)
    }

    fn init_uninstall_script() -> anyhow::Result<std::path::PathBuf> {
        use std::io::Write as _;

        let mut temp_sh = tempfile::Builder::new()
            .prefix("iroh_ssh_uninstall-")
            .suffix(".sh")
            .tempfile_in("/tmp")?;
        temp_sh.write_all(LinuxService::UNINSTALL_SH_BYTES.as_bytes())?;
        let sh_path = temp_sh.path().to_path_buf();
        temp_sh.keep()?;

        Ok(sh_path)
    }
}
