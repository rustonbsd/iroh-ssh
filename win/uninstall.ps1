$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$service = Get-Service -Name "[__SERVICE_NAME__]" -ErrorAction SilentlyContinue

if (-not $service) {
    if (Test-Path -Path "[__BINARY_DIR__]") {
        Remove-Item -Path "[__BINARY_DIR__]" -Recurse -Force
    }
    exit 0
}

if ($service.Status -eq 'Running') {
    Stop-Service -Name "[__SERVICE_NAME__]" -Force
}

& "[__NSSM_PATH__]" remove "[__SERVICE_NAME__]" confirm

if (Test-Path -Path "[__BINARY_DIR__]") {
    Remove-Item -Path "[__BINARY_DIR__]" -Recurse -Force
}