function Install-BuildkiteAgent() {
    # Though the Windows machine we're running on has to have the
    # buildkite-agent installed, by definition, if you need to use the
    # buildkite-agent inside a container running on that host (e.g., to
    # do artifact uploads, or to manipulate pipeline metadata), then
    # you'll need to install it in the container as well.
    Write-Host "--- Installing buildkite agent in container"
    Invoke-Expression ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/buildkite/agent/master/install.ps1')) | Out-Null
}

function Install-LatestHabitat() {
    # Install latest hab from using install.ps1
    $env:HAB_LICENSE = "accept-no-persist"
    Write-Host "--- :habicat: Installing latest hab binary for $Env:HAB_PACKAGE_TARGET using install.ps1"
    Set-ExecutionPolicy Bypass -Scope Process -Force
    Invoke-Expression ((New-Object System.Net.WebClient).DownloadString('https://raw.githubusercontent.com/habitat-sh/habitat/master/components/hab/install.ps1')) | Out-Null
}

function Set-WorkerIdent($PackageIdent) {
    Write-Host "--- Registering $PackageIdent (x86_64-windows)"
    Invoke-Expression "buildkite-agent meta-data set x86_64-windows-builder-worker $PackageIdent"
}
