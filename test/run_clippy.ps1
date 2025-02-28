#Requires -Version 5

param (
    [string]$UnexaminedLintsPath,
    [string]$AllowedLintsPath,
    [string]$LintsToFixPath,
    [string]$DeniedLintsPath
)

$ErrorActionPreference="stop"
. $PSScriptRoot\..\support\ci\shared.ps1

$Env:RUSTFLAGS="-D warnings"

function Convert-ArrayToArgs ($arg, $list) {
    if($list) {
        $list | ForEach-Object { "-$arg $_ ``" } | Out-String
    }
}

$toolchain = Get-Toolchain
Install-Rustup $toolchain
Install-RustToolchain $toolchain

Write-Host "Installing clippy"
rustup component add --toolchain "$toolchain-x86_64-pc-windows-msvc" clippy

Setup-Environment

$clippyArgs += Convert-ArrayToArgs -arg A -list (Get-Content $UnexaminedLintsPath)
$clippyArgs += Convert-ArrayToArgs -arg A -list (Get-Content $AllowedLintsPath)
$clippyArgs += Convert-ArrayToArgs -arg W -list (Get-Content $LintsToFixPath)
$clippyArgs += Convert-ArrayToArgs -arg D -list (Get-Content $DeniedLintsPath)

Write-Host "--- Running clippy!"
Write-Host "Clippy rules: $clippyCommand"
Invoke-Expression $clippyCommand

if ($LASTEXITCODE -ne 0) {exit $LASTEXITCODE}
