#!/usr/bin/env powershell

#Requires -Version 5

$ErrorActionPreference="stop"

# Import shared functions
. $PSScriptRoot\shared.ps1

# We have to do this because everything that comes from vault is quoted on windows.
$Rawtoken=$Env:PIPELINE_HAB_AUTH_TOKEN
$Env:HAB_AUTH_TOKEN=$Rawtoken.Replace("`"","")

$Env:buildkiteAgentToken = $Env:BUILDKITE_AGENT_ACCESS_TOKEN

$Env:HAB_BLDR_URL=$Env:PIPELINE_HAB_BLDR_URL
$Env:HAB_PACKAGE_TARGET=$Env:BUILD_PKG_TARGET

Install-BuildkiteAgent

Install-LatestHabitat

# Get keys
Write-Host "--- :key: Downloading '$Env:HAB_ORIGIN' public keys from Builder"
Invoke-Expression "hab origin key download $Env:HAB_ORIGIN"
Write-Host "--- :closed_lock_with_key: Downloading latest '$Env:HAB_ORIGIN' secret key from Builder"
Invoke-Expression "hab origin key download $Env:HAB_ORIGIN --auth $Env:HAB_AUTH_TOKEN --secret"
$Env:HAB_CACHE_KEY_PATH = "C:\hab\cache\keys"

# Run a build!
Write-Host "--- :habicat: Building builder-worker"

Invoke-Expression "hab pkg build components\builder-worker"
. results\last_build.ps1

Write-Host "--- :habicat: Uploading $pkg_ident to $env:HAB_BLDR_URL in the 'unstable' channel"
Invoke-Expression "hab pkg upload results\$pkg_artifact --no-build"
Set-WorkerIdent $pkg_ident

Invoke-Expression "buildkite-agent annotate --append --context 'release-manifest' '<br>* ${pkg_ident} (x86_64-windows)'"

exit $LASTEXITCODE
