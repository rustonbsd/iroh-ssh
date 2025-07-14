$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

# Check if the service is already installed.
$service = Get-Service -Name [__SERVICE_NAME__] -ErrorAction SilentlyContinue
if ($service) {
    Write-Host "Service '[__SERVICE_NAME__]' is already installed."
    exit 0
}

New-Item -Path [__BINARY_DIR__] -ItemType Directory -Force | Out-Null

$userProfile = [Environment]::GetFolderPath('UserProfile')
$serviceProfileDir = Join-Path $userProfile ".ssh"
New-Item -Path $serviceProfileDir -ItemType Directory -Force | Out-Null

$destinationExe = Join-Path "[__BINARY_DIR__]" "[__SERVICE_NAME__].exe"
Copy-Item -Path "[__CURRENT_EXE_PATH__]" -Destination $destinationExe -Force

function Set-NssmParameter {
    param($Parameter, [string[]]$Value)
    & "[__NSSM_PATH__]" set [__SERVICE_NAME__] $Parameter $Value
}

& "[__NSSM_PATH__]" install [__SERVICE_NAME__] $destinationExe
Set-NssmParameter "AppParameters" "server --persist --ssh-port [__SSH_PORT__]"
Set-NssmParameter "AppDirectory" "[__BINARY_DIR__]"
Set-NssmParameter "AppExit" @("Default", "Restart")
Set-NssmParameter "AppPriority" "HIGH_PRIORITY_CLASS"
Set-NssmParameter "AppStdout" (Join-Path "[__BINARY_DIR__]" "[__SERVICE_NAME__].log")
Set-NssmParameter "AppStderr" (Join-Path "[__BINARY_DIR__]" "[__SERVICE_NAME__].error.log")
Set-NssmParameter "AppTimestampLog" "1"
Set-NssmParameter "DependOnService" ":sshd"
Set-NssmParameter "Description" "ssh without ip"
Set-NssmParameter "DisplayName" [__SERVICE_NAME__]
Set-NssmParameter "Start" "SERVICE_AUTO_START"
Set-NssmParameter "Type" "SERVICE_WIN32_OWN_PROCESS"

Start-Service -Name [__SERVICE_NAME__]