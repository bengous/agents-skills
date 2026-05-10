#Requires -Version 5.1
$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

$Uv = Get-Command uv -CommandType Application -ErrorAction SilentlyContinue
if ($null -eq $Uv) {
    [Console]::Error.WriteLine("error: uv is required: https://docs.astral.sh/uv/")
    exit 127
}

$SkillDir = Split-Path -Parent $PSScriptRoot
& $Uv.Source run --project $SkillDir --locked itw @args
exit $LASTEXITCODE
