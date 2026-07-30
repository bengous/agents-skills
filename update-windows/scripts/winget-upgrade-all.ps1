# Upgrade every winget package in one pass, under a single UAC elevation.
# Elevates itself; run it from an ordinary session.
# Usage: winget-upgrade-all.ps1 <LogPath>
param([Parameter(Mandatory = $true)][string]$Log)

# Windows PowerShell turns native stderr into ErrorRecords under `2>&1`, and
# PowerShell 7.4+ turns a non-zero native exit code into a terminating error.
# Either one would abort the batch on the first package that fails.
$ErrorActionPreference = "Continue"
$PSNativeCommandUseErrorActionPreference = $false

# Decode winget output as UTF-8 so the log stays greppable.
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }

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
    # --disable-interactivity: any prompt inside the hidden window hangs forever.
    winget upgrade --all --include-unknown --silent --disable-interactivity `
        --accept-package-agreements --accept-source-agreements 2>&1 |
        ForEach-Object { Write-Log "$_" }
    $code = $LASTEXITCODE
} catch {
    Write-Log "ERROR $($_.Exception.Message)"
    $code = 1
} finally {
    Write-Terminator $code
}

exit $code
