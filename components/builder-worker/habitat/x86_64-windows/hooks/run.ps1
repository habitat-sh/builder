$env:HOME = "{{pkg.svc_data_path}}"
$env:RUST_LOG = "{{cfg.log_level}}"
$env:RUST_BACKTRACE = 1

# Wait for pem file before starting the service
while (!(Test-Path -Path "{{pkg.svc_files_path}}/builder-github-app.pem")) {
    Write-Host "Waiting for builder-github-app.pem"
    Start-Sleep -Seconds 30
}

Write-Host "Starting builder-worker, parent process environment:"
gci env:

bldr-worker start -c "{{pkg.svc_config_path}}/config.toml"
exit $LASTEXITCODE
