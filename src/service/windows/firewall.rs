#[cfg(target_os = "windows")]
use anyhow::{Context, Result, bail};
use std::path::Path;
use std::process::Command;

pub fn add_firewall_rules(executable_path: &Path) -> Result<()> {
    let exe_path = executable_path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("executable path contains invalid UTF-8"))?;

    tracing::info!("Adding Windows Firewall rules for: {}", exe_path);

    let ps_script = format!(
        r#"
$ErrorActionPreference = 'Stop'

# Remove old rules if they exist (ignore errors)
Remove-NetFirewallRule -DisplayName 'iroh-ssh Service Outbound' -ErrorAction SilentlyContinue
Remove-NetFirewallRule -DisplayName 'iroh-ssh Service Inbound' -ErrorAction SilentlyContinue

# Add outbound UDP rule for relay connections and STUN
New-NetFirewallRule `
    -DisplayName 'iroh-ssh Service Outbound' `
    -Description 'Allow outbound UDP for iroh-ssh QUIC, relay, and holepunching' `
    -Direction Outbound `
    -Action Allow `
    -Protocol UDP `
    -Program '{}' `
    -Profile Any `
    -Enabled True | Out-Null

Write-Host 'Added outbound rule'

# Add inbound UDP rule for accepting holepunched connections
New-NetFirewallRule `
    -DisplayName 'iroh-ssh Service Inbound' `
    -Description 'Allow inbound UDP for iroh-ssh holepunching and direct connections' `
    -Direction Inbound `
    -Action Allow `
    -Protocol UDP `
    -Program '{}' `
    -Profile Any `
    -Enabled True | Out-Null

Write-Host 'Added inbound rule'

# Also add outbound TCP rule for HTTPS relay connections
New-NetFirewallRule `
    -DisplayName 'iroh-ssh Service HTTPS' `
    -Description 'Allow outbound HTTPS for iroh-ssh relay server connections' `
    -Direction Outbound `
    -Action Allow `
    -Protocol TCP `
    -Program '{}' `
    -RemotePort 443 `
    -Profile Any `
    -Enabled True | Out-Null

Write-Host 'Added HTTPS rule'
"#,
        exe_path, exe_path, exe_path
    );

    let output = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &ps_script,
        ])
        .output()
        .context("Failed to execute PowerShell to add firewall rules")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "PowerShell failed to add firewall rules.\nStdout: {}\nStderr: {}",
            stdout,
            stderr
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    tracing::info!("Firewall rules added successfully: {}", stdout);

    Ok(())
}

pub fn remove_firewall_rules() -> Result<()> {
    tracing::info!("Removing Windows Firewall rules for iroh-ssh");

    let ps_script = r#"
$ErrorActionPreference = 'Stop'

Remove-NetFirewallRule -DisplayName 'iroh-ssh Service Outbound' -ErrorAction Stop
Write-Host 'Removed outbound rule'

Remove-NetFirewallRule -DisplayName 'iroh-ssh Service Inbound' -ErrorAction Stop
Write-Host 'Removed inbound rule'

Remove-NetFirewallRule -DisplayName 'iroh-ssh Service HTTPS' -ErrorAction Stop
Write-Host 'Removed HTTPS rule'
"#;

    let output = Command::new("powershell")
        .args(&[
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            ps_script,
        ])
        .output()
        .context("Failed to execute PowerShell to remove firewall rules")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        // Don't fail on cleanup - just log the error
        tracing::warn!(
            "Failed to remove some firewall rules (may not exist).\nStdout: {}\nStderr: {}",
            stdout,
            stderr
        );
    } else {
        let stdout = String::from_utf8_lossy(&output.stdout);
        tracing::info!("Firewall rules removed successfully: {}", stdout);
    }

    Ok(())
}