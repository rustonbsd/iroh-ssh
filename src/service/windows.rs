use crate::{Service, ServiceParams};

use anyhow::{Context, anyhow, bail};

#[cfg(target_os = "windows")]
mod firewall;

#[cfg(target_os = "windows")]
use std::{
    ffi::{OsStr, OsString, c_void},
    fs, io, iter, mem,
    os::windows::ffi::OsStrExt,
    path::{Path, PathBuf},
    ptr,
    sync::OnceLock,
    time::{Duration, Instant},
};

#[cfg(target_os = "windows")]
use tokio::task;

#[cfg(target_os = "windows")]
use windows_service::{
    Error as WinServiceError,
    service::{
        Service as WinService, ServiceAccess, ServiceAction, ServiceActionType, ServiceDependency,
        ServiceErrorControl, ServiceFailureActions, ServiceFailureResetPeriod, ServiceInfo,
        ServiceSidType, ServiceStartType, ServiceType,
    },
    service_manager::{ServiceManager, ServiceManagerAccess},
};

#[cfg(target_os = "windows")]
use windows_sys::Win32::{
    Foundation::{
        ERROR_INSUFFICIENT_BUFFER, ERROR_SERVICE_ALREADY_RUNNING, ERROR_SERVICE_DOES_NOT_EXIST,
        ERROR_SERVICE_NOT_ACTIVE, ERROR_SUCCESS, GetLastError, LocalFree,
    },
    Security::{
        ACL,
        Authorization::{
            EXPLICIT_ACCESS_W, GetNamedSecurityInfoW, SE_FILE_OBJECT, SET_ACCESS, SetEntriesInAclW,
            SetNamedSecurityInfoW, TRUSTEE_IS_SID, TRUSTEE_IS_UNKNOWN, TRUSTEE_W,
        },
        CONTAINER_INHERIT_ACE, DACL_SECURITY_INFORMATION, LookupAccountNameW, OBJECT_INHERIT_ACE,
        UNPROTECTED_DACL_SECURITY_INFORMATION,
    },
    Storage::FileSystem::{DELETE, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE},
};

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub struct WindowsService;

#[cfg(target_os = "windows")]
static SERVICE_SSH_PORT: OnceLock<u16> = OnceLock::new();

#[cfg(target_os = "windows")]
impl Service for WindowsService {
    async fn install(service_params: ServiceParams) -> anyhow::Result<()> {
        task::spawn_blocking(move || WindowsService::install_blocking(service_params))
            .await
            .context("windows service install task panicked")??;
        Ok(())
    }

    async fn info() -> anyhow::Result<()> {
        todo!("service info is not yet supported")
    }

    async fn uninstall() -> anyhow::Result<()> {
        task::spawn_blocking(WindowsService::uninstall_blocking)
            .await
            .context("windows service uninstall task panicked")??;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
impl WindowsService {
    pub async fn run_service(service_params: ServiceParams) -> anyhow::Result<()> {
        task::spawn_blocking(move || WindowsService::run_service_dispatcher(service_params))
            .await
            .context("windows service dispatcher task panicked")??;
        Ok(())
    }

    fn run_service_dispatcher(service_params: ServiceParams) -> anyhow::Result<()> {
        SERVICE_SSH_PORT
            .set(service_params.ssh_port)
            .ok()
            .or_else(|| {
                if SERVICE_SSH_PORT.get().copied() != Some(service_params.ssh_port) {
                    None
                } else {
                    Some(())
                }
            })
            .ok_or_else(|| anyhow!("service port already initialized with different value"))?;

        service_runtime::run().context("failed to start windows service dispatcher")?;
        Ok(())
    }

    fn service_port() -> anyhow::Result<u16> {
        SERVICE_SSH_PORT
            .get()
            .copied()
            .ok_or_else(|| anyhow!("service port not initialized"))
    }

    pub const SERVICE_NAME: &'static str = "iroh-ssh";
    pub const SERVICE_DISPLAY_NAME: &'static str = "iroh-ssh";
    pub const SERVICE_DESCRIPTION: &'static str = "SSH to any machine without ip";
    pub const SERVICE_ACCOUNT: &'static str = "NT SERVICE\\iroh-ssh";
    pub const SERVICE_DEPENDENCY: &'static str = "sshd";
    pub const INSTALL_ROOT: &'static str = r"C:\\ProgramData\\iroh-ssh";
    pub const SERVICE_BINARY_NAME: &'static str = "iroh-ssh.exe";
    pub const SERVICE_PROFILE_ROOT: &'static str = r"C:\\Windows\\ServiceProfiles\\iroh-ssh";
    pub const SERVICE_SSH_DIR: &'static str = r"C:\\Windows\\ServiceProfiles\\iroh-ssh\\.ssh";

    fn install_blocking(service_params: ServiceParams) -> anyhow::Result<()> {
        let staged_binary = Self::stage_binary().context("failed to stage service binary")?;

        tracing::info!("Adding Windows Firewall rules for service executable");
        firewall::add_firewall_rules(&staged_binary)
            .context("failed to add Windows Firewall rules - ensure running as administrator")?;

        let service = Self::create_or_configure_service(&staged_binary, &service_params)
            .context("failed to create or configure windows service")?;

        let service_sid = Self::lookup_service_sid().context("failed to resolve service SID")?;

        Self::ensure_runtime_directories().context("failed to ensure runtime directories")?;

        Self::apply_service_permissions(Path::new(Self::INSTALL_ROOT), &service_sid)
            .with_context(|| format!("failed to set permissions for {}", Self::INSTALL_ROOT))?;
        Self::apply_service_permissions(Path::new(Self::SERVICE_PROFILE_ROOT), &service_sid)
            .with_context(|| {
                format!(
                    "failed to set permissions for {}",
                    Self::SERVICE_PROFILE_ROOT
                )
            })?;
        Self::apply_service_permissions(Path::new(Self::SERVICE_SSH_DIR), &service_sid)
            .with_context(|| format!("failed to set permissions for {}", Self::SERVICE_SSH_DIR))?;

        Self::start_service(&service).context("failed to start windows service")?;
        Ok(())
    }

    fn uninstall_blocking() -> anyhow::Result<()> {
        Self::remove_service().context("failed to remove windows service")?;

        // Remove Windows Firewall rules (ignore errors on cleanup)
        tracing::info!("Removing Windows Firewall rules for service");
        if let Err(e) = firewall::remove_firewall_rules() {
            tracing::warn!("Failed to remove firewall rules (may not exist): {}", e);
        }
        let staged_binary = Path::new(Self::INSTALL_ROOT).join(Self::SERVICE_BINARY_NAME);
        match fs::remove_file(&staged_binary) {
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err).context("failed to remove staged service binary"),
        }
        Ok(())
    }

    fn stage_binary() -> anyhow::Result<PathBuf> {
        let source = std::env::current_exe().context("could not determine current executable")?;
        let target_dir = Path::new(Self::INSTALL_ROOT);
        Self::ensure_directory(target_dir).context("failed to create install root")?;

        let target = target_dir.join(Self::SERVICE_BINARY_NAME);

        fs::copy(&source, &target).with_context(|| {
            format!(
                "failed to copy service binary from {} to {}",
                source.display(),
                target.display()
            )
        })?;

        Ok(target)
    }

    fn create_or_configure_service(
        binary_path: &Path,
        service_params: &ServiceParams,
    ) -> anyhow::Result<WinService> {
        let manager_access = ServiceManagerAccess::CONNECT | ServiceManagerAccess::CREATE_SERVICE;
        let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)
            .context("failed to connect to service control manager")?;

        let desired_access = ServiceAccess::CHANGE_CONFIG
            | ServiceAccess::QUERY_CONFIG
            | ServiceAccess::QUERY_STATUS
            | ServiceAccess::START
            | ServiceAccess::STOP;

        let service_info = ServiceInfo {
            name: OsString::from(Self::SERVICE_NAME),
            display_name: OsString::from(Self::SERVICE_DISPLAY_NAME),
            service_type: ServiceType::OWN_PROCESS,
            start_type: ServiceStartType::AutoStart,
            error_control: ServiceErrorControl::Normal,
            executable_path: binary_path.to_path_buf(),
            launch_arguments: vec![
                OsString::from("run-service"),
                OsString::from("--ssh-port"),
                OsString::from(service_params.ssh_port.to_string()),
            ],
            dependencies: vec![ServiceDependency::Service(OsString::from(
                Self::SERVICE_DEPENDENCY,
            ))],
            account_name: Some(OsString::from(Self::SERVICE_ACCOUNT)),
            account_password: None,
        };

        let service = match service_manager.open_service(Self::SERVICE_NAME, desired_access) {
            Ok(service) => {
                service
                    .change_config(&service_info)
                    .context("failed to update existing service configuration")?;
                service
            }
            Err(WinServiceError::Winapi(err))
                if err.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) =>
            {
                service_manager
                    .create_service(&service_info, desired_access)
                    .context("failed to create windows service")?
            }
            Err(err) => return Err(err).context("failed to open existing windows service"),
        };

        service
            .set_description(Self::SERVICE_DESCRIPTION)
            .context("failed to set service description")?;
        service
            .set_delayed_auto_start(true)
            .context("failed to configure delayed auto-start")?;
        service
            .set_config_service_sid_info(ServiceSidType::Unrestricted)
            .context("failed to set unrestricted service SID type")?;
        Self::configure_failure_actions(&service)
            .context("failed to configure service failure actions")?;

        Ok(service)
    }

    fn configure_failure_actions(service: &WinService) -> anyhow::Result<()> {
        let actions = vec![
            ServiceAction {
                action_type: ServiceActionType::Restart,
                delay: Duration::from_millis(5_000),
            },
            ServiceAction {
                action_type: ServiceActionType::Restart,
                delay: Duration::from_millis(60_000),
            },
            ServiceAction {
                action_type: ServiceActionType::Restart,
                delay: Duration::from_millis(600_000),
            },
        ];

        let failure_actions = ServiceFailureActions {
            reset_period: ServiceFailureResetPeriod::After(Duration::from_secs(86_400)),
            reboot_msg: None,
            command: None,
            actions: Some(actions),
        };

        service
            .update_failure_actions(failure_actions)
            .context("failed to set service failure actions")?;
        service
            .set_failure_actions_on_non_crash_failures(true)
            .context("failed to enable service failure actions on non-crash failures")?;
        Ok(())
    }

    fn lookup_service_sid() -> anyhow::Result<Vec<u8>> {
        let account = Self::SERVICE_ACCOUNT;
        let account_wide: Vec<u16> = OsStr::new(account)
            .encode_wide()
            .chain(iter::once(0))
            .collect();

        let mut sid_len = 0u32;
        let mut domain_len = 0u32;
        let mut sid_use: i32 = 0;

        unsafe {
            LookupAccountNameW(
                ptr::null(),
                account_wide.as_ptr(),
                ptr::null_mut(),
                &mut sid_len,
                ptr::null_mut(),
                &mut domain_len,
                &mut sid_use,
            );

            if GetLastError() != ERROR_INSUFFICIENT_BUFFER {
                bail!("LookupAccountNameW failed to size service SID");
            }

            let mut sid_buffer = vec![0u8; sid_len as usize];
            let mut domain_buffer = vec![0u16; domain_len as usize];

            if LookupAccountNameW(
                ptr::null(),
                account_wide.as_ptr(),
                sid_buffer.as_mut_ptr() as *mut _,
                &mut sid_len,
                domain_buffer.as_mut_ptr(),
                &mut domain_len,
                &mut sid_use,
            ) == 0
            {
                bail!("LookupAccountNameW failed with {:#x}", GetLastError());
            }

            sid_buffer.truncate(sid_len as usize);
            Ok(sid_buffer)
        }
    }

    fn ensure_runtime_directories() -> anyhow::Result<()> {
        Self::ensure_directory(Path::new(Self::INSTALL_ROOT))
            .context("failed to ensure install root")?;
        Self::ensure_directory(Path::new(Self::SERVICE_PROFILE_ROOT))
            .context("failed to ensure service profile root")?;
        Self::ensure_directory(Path::new(Self::SERVICE_SSH_DIR))
            .context("failed to ensure service .ssh directory")?;
        Ok(())
    }

    fn ensure_directory(path: &Path) -> io::Result<()> {
        if !path.exists() {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }

    fn apply_service_permissions(path: &Path, service_sid: &[u8]) -> anyhow::Result<()> {
        if !path.exists() {
            return Ok(());
        }

        Self::grant_modify_acl(path, service_sid)
    }

    fn grant_modify_acl(path: &Path, service_sid: &[u8]) -> anyhow::Result<()> {
        let path_wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(iter::once(0))
            .collect();

        unsafe {
            let mut trustee: TRUSTEE_W = mem::zeroed();
            trustee.TrusteeForm = TRUSTEE_IS_SID;
            trustee.TrusteeType = TRUSTEE_IS_UNKNOWN;
            trustee.ptstrName = service_sid.as_ptr() as *mut _;

            let mut access: EXPLICIT_ACCESS_W = mem::zeroed();
            access.grfAccessPermissions =
                FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE | DELETE;
            access.grfAccessMode = SET_ACCESS;
            access.grfInheritance = OBJECT_INHERIT_ACE | CONTAINER_INHERIT_ACE;
            access.Trustee = trustee;

            let mut security_descriptor: *mut c_void = ptr::null_mut();
            let mut existing_dacl: *mut ACL = ptr::null_mut();

            let status = GetNamedSecurityInfoW(
                path_wide.as_ptr() as _,
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                &mut existing_dacl,
                ptr::null_mut(),
                &mut security_descriptor,
            );

            if status != ERROR_SUCCESS {
                return Err(anyhow!(
                    "GetNamedSecurityInfoW failed for {} with status {status}",
                    path.display()
                ));
            }

            let mut new_acl: *mut ACL = ptr::null_mut();
            let status = SetEntriesInAclW(1, &mut access, existing_dacl, &mut new_acl);
            if status != ERROR_SUCCESS {
                LocalFree(security_descriptor as _);
                return Err(anyhow!(
                    "SetEntriesInAclW failed for {} with status {status}",
                    path.display()
                ));
            }

            let status = SetNamedSecurityInfoW(
                path_wide.as_ptr() as _,
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION | UNPROTECTED_DACL_SECURITY_INFORMATION,
                ptr::null_mut(),
                ptr::null_mut(),
                new_acl,
                ptr::null_mut(),
            );

            if !new_acl.is_null() {
                LocalFree(new_acl as _);
            }
            LocalFree(security_descriptor as _);

            if status != ERROR_SUCCESS {
                return Err(anyhow!(
                    "SetNamedSecurityInfoW failed for {} with status {status}",
                    path.display()
                ));
            }
        }

        Ok(())
    }

    fn start_service(service: &WinService) -> anyhow::Result<()> {
        match service.start::<&OsStr>(&[]) {
            Ok(()) => Ok(()),
            Err(WinServiceError::Winapi(err))
                if err.raw_os_error() == Some(ERROR_SERVICE_ALREADY_RUNNING as i32) =>
            {
                Ok(())
            }
            Err(err) => Err(err).context("failed to start service"),
        }
    }

    fn remove_service() -> anyhow::Result<()> {
        let manager_access = ServiceManagerAccess::CONNECT;
        let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)
            .context("failed to connect to service control manager")?;

        let service_access =
            ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
        let service = match service_manager.open_service(Self::SERVICE_NAME, service_access) {
            Ok(service) => service,
            Err(WinServiceError::Winapi(err))
                if err.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) =>
            {
                return Ok(());
            }
            Err(err) => return Err(err).context("failed to open existing service"),
        };

        if let Err(err) = service.delete() {
            return Err(err).context("failed to mark service for deletion");
        }

        if let Err(err) = service.stop() {
            match err {
                WinServiceError::Winapi(io_err)
                    if io_err.raw_os_error() == Some(ERROR_SERVICE_NOT_ACTIVE as i32) => {}
                other => return Err(other).context("failed to stop service"),
            }
        }

        drop(service);

        let start = Instant::now();
        let timeout = Duration::from_secs(5);
        while start.elapsed() < timeout {
            match service_manager.open_service(Self::SERVICE_NAME, ServiceAccess::QUERY_STATUS) {
                Ok(_) => {
                    std::thread::sleep(Duration::from_secs(1));
                }
                Err(WinServiceError::Winapi(err))
                    if err.raw_os_error() == Some(ERROR_SERVICE_DOES_NOT_EXIST as i32) =>
                {
                    return Ok(());
                }
                Err(err) => return Err(err).context("failed while waiting for service deletion"),
            }
        }

        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod service_runtime {
    use super::WindowsService;
    use crate::{IrohOpts, ServerArgs};
    use std::{ffi::OsString, io, sync::mpsc, time::Duration};
    use tokio::runtime::Builder;
    use windows_service::{
        Result as WinResult, define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
    };

    const STOP_EVENT_CODE: u32 = 130;

    pub(super) fn run() -> WinResult<()> {
        service_dispatcher::start(WindowsService::SERVICE_NAME, ffi_service_main)
    }

    define_windows_service!(ffi_service_main, service_main);

    fn service_main(_arguments: Vec<OsString>) {
        let log_dir = std::path::PathBuf::from(WindowsService::SERVICE_PROFILE_ROOT);
        let file_appender = tracing_appender::rolling::never(&log_dir, "iroh-ssh-service.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        tracing_subscriber::fmt()
            .with_writer(non_blocking)
            .with_ansi(false)
            .init();

        tracing::info!("=== iroh-ssh service starting ===");

        if let Err(error) = run_service_worker() {
            tracing::error!("iroh-ssh service failed: {error:?}");
        }
    }

    fn run_service_worker() -> WinResult<()> {
        tracing::info!("run_service_worker: Starting");

        let ssh_port = WindowsService::service_port().map_err(anyhow_to_win_error)?;

        tracing::info!("run_service_worker: SSH port = {}", ssh_port);

        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        let event_handler = move |control_event| -> ServiceControlHandlerResult {
            match control_event {
                ServiceControl::Stop => {
                    let _ = shutdown_tx.send(());
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                ServiceControl::UserEvent(code) if code.to_raw() == STOP_EVENT_CODE => {
                    let _ = shutdown_tx.send(());
                    ServiceControlHandlerResult::NoError
                }
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        };

        let status_handle =
            service_control_handler::register(WindowsService::SERVICE_NAME, event_handler)?;

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;

        let runtime = Builder::new_multi_thread()
            .enable_all()
            .build()
            .map_err(|err| anyhow_to_win_error(err.into()))?;

        let server_handle = runtime.spawn(async move {
            tracing::info!("Spawning server_mode task");

            let result = crate::api::server_mode(
                ServerArgs {
                    ssh_port,
                    persist: true,
                    iroh: IrohOpts { relay_url: vec![] },
                },
                true,
            )
            .await;

            if let Err(err) = result {
                tracing::error!("iroh-ssh server task failed: {err:?}");
            }
        });

        shutdown_rx.recv().ok();

        runtime.block_on(async {
            server_handle.abort();
            let _ = server_handle.await;
        });

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })?;

        Ok(())
    }

    fn anyhow_to_win_error(error: anyhow::Error) -> windows_service::Error {
        windows_service::Error::Winapi(io::Error::new(io::ErrorKind::Other, error.to_string()))
    }
}
