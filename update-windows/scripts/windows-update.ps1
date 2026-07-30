# Install every pending Windows Update (patches, drivers, firmware, Defender
# definitions) without rebooting. Elevates itself; run it from an ordinary session.
# Usage: windows-update.ps1 <LogPath>
param([Parameter(Mandatory = $true)][string]$Log)

$ErrorActionPreference = "Continue"
$PSNativeCommandUseErrorActionPreference = $false

try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }
# PSGallery refuses anything below TLS 1.2, and Windows PowerShell still
# defaults lower.
[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

function Write-Log([string]$Message) {
    $Message | Out-File -FilePath $Log -Encoding utf8 -Append
}

function Write-Terminator([int]$Code) {
    Write-Log "=== Done $(Get-Date -Format o) exit=$Code ==="
}

$principal = New-Object Security.Principal.WindowsPrincipal(
    [Security.Principal.WindowsIdentity]::GetCurrent())

if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    $host_exe = (Get-Process -Id $PID).Path
    $arguments = '-NoProfile -ExecutionPolicy Bypass -File "' + $PSCommandPath + '" "' + $Log + '"'
    try {
        Start-Process -FilePath $host_exe -Verb RunAs -WindowStyle Hidden `
            -ArgumentList $arguments -ErrorAction Stop | Out-Null
    } catch {
        # ERROR_CANCELLED: the user dismissed the UAC prompt.
        "=== Start $(Get-Date -Format o) ===" | Out-File -FilePath $Log -Encoding utf8
        Write-Log "ELEVATION_REFUSED $($_.Exception.Message)"
        Write-Terminator 1223
        exit 1223
    }
    # The elevated instance owns the log from here; writing to it would race.
    exit 0
}

"=== Start $(Get-Date -Format o) ===" | Out-File -FilePath $Log -Encoding utf8
$code = 0
try {
    $ready = $true
    if (-not (Get-Module -ListAvailable -Name PSWindowsUpdate)) {
        Write-Log "Installing the PSWindowsUpdate module..."
        try {
            Install-PackageProvider -Name NuGet -Force -Confirm:$false `
                -Scope CurrentUser -ErrorAction Stop | Out-Null
            Install-Module -Name PSWindowsUpdate -Force -Confirm:$false `
                -Scope CurrentUser -AllowClobber -ErrorAction Stop
        } catch {
            # Unreachable gallery, not a Windows Update failure. The caller
            # falls back to the Settings app on this marker.
            Write-Log "MODULE_UNAVAILABLE $($_.Exception.Message)"
            $ready = $false
            $code = 2
        }
    }

    if ($ready) {
        Import-Module PSWindowsUpdate -ErrorAction Stop
        # -Verbose *>&1 streams per-update progress into the log as it happens;
        # without it the log stays empty until the very end.
        Get-WindowsUpdate -AcceptAll -Install -IgnoreReboot -Confirm:$false -Verbose *>&1 |
            ForEach-Object { Write-Log "$_" }
        Write-Log "RebootRequired: $(Get-WURebootStatus -Silent)"
    }
} catch {
    Write-Log "ERROR $($_.Exception.Message)"
    $code = 1
} finally {
    Write-Terminator $code
}

exit $code
