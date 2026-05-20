#Requires -Version 5.1
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$Uv = Get-Command uv -CommandType Application -ErrorAction SilentlyContinue
if ($null -eq $Uv) {
    [Console]::Error.WriteLine("error: uv is required: https://docs.astral.sh/uv/")
    exit 127
}

$SkillDir = Split-Path -Parent $PSScriptRoot
$ExpectedFingerprint = (& $Uv.Source run --project $SkillDir --locked itw setup fingerprint)
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

& $Uv.Source tool install $SkillDir --force --reinstall
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

& $Uv.Source run --project $SkillDir --locked itw setup install
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$Itw = Get-Command itw -CommandType Application -ErrorAction SilentlyContinue
if ($null -eq $Itw) {
    [Console]::Error.WriteLine("error: itw was installed but is not on PATH; ensure the uv tool bin directory is on PATH, then rerun setup.")
    exit 1
}

$ActualFingerprint = (& $Itw.Source setup fingerprint)
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}
if ($ActualFingerprint -ne $ExpectedFingerprint) {
    [Console]::Error.WriteLine("error: installed itw fingerprint mismatch")
    [Console]::Error.WriteLine("expected: $ExpectedFingerprint")
    [Console]::Error.WriteLine("actual:   $ActualFingerprint")
    exit 1
}

& $Itw.Source setup status
exit $LASTEXITCODE
