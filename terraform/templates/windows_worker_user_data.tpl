<script>
  winrm quickconfig -q & winrm set winrm/config @{MaxTimeoutms="1800000"} & winrm set winrm/config/service @{AllowUnencrypted="true"} & winrm set winrm/config/service/auth @{Basic="true"}
</script>
<powershell>
  netsh advfirewall firewall add rule name="WinRM in" protocol=TCP dir=in profile=any localport=5985 remoteip=any localip=any action=allow
  New-NetFirewallRule -DisplayName "Habitat TCP" -Direction Inbound -Action Allow -Protocol TCP -LocalPort 9631,9638
  New-NetFirewallRule -DisplayName "Habitat UDP" -Direction Inbound -Action Allow -Protocol UDP -LocalPort 9638

  # Firewall rule to block all tcp/udp traffic from studios
  # into the worker network except dns queries (UDP 53)
  $nat_cidr = (get-netnat).InternalIPInterfaceAddressPrefix
  $nat_gw = $nat_cidr.Split("/")[0]
  $eth = Get-NetIPConfiguration -InterfaceAlias "Ethernet"
  $ip = $eth.IPv4Address
  $length = $ip[0].PrefixLength
  $gw = $eth.IPv4DefaultGateway.NextHop
  $aws_cidr = "$gw/$length".Replace(".1/",".0/")
  New-NetFirewallRule -DisplayName docker_nat_block_tcp -Enabled True -Profile Any -Direction Outbound -Action Block -LocalAddress $nat_cidr -RemoteAddress @($aws_cidr, $nat_gw) -Protocol TCP
  New-NetFirewallRule -DisplayName docker_nat_block_udp -Enabled True -Profile Any -Direction Outbound -Action Block -LocalAddress $nat_cidr -RemoteAddress @($aws_cidr, $nat_gw) -Protocol UDP -RemotePort @("0-52","54-65535")

  # Set Administrator password
  $admin = [adsi]("WinNT://./administrator, user")
  $admin.psbase.invoke("SetPassword", "${password}")

  # Install Chocolatey (for ease of installing debugging packages if needed)
  Set-ExecutionPolicy Bypass -Scope Process -Force; iex ((New-Object System.Net.WebClient).DownloadString('https://chocolatey.org/install.ps1'))

  # Install habitat
  $env:Path = "C:\hab\bin",$env:Path -join ";"
  [System.Environment]::SetEnvironmentVariable('Path', $env:Path, [System.EnvironmentVariableTarget]::Machine)
  [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
  iwr https://packages.chef.io/files/stable/habitat/latest/hab-x86_64-windows.zip -Outfile c:\habitat.zip
  Expand-Archive c:/habitat.zip c:/
  mv c:/hab-* c:/habitat
  Remove-Item c:/habitat.zip

  # Install hab as a Windows service
  $env:HAB_LICENSE='accept';
  SETX HAB_LICENSE accept /m;
  c:\habitat\hab.exe pkg install core/hab --binlink --force
  Remove-Item c:\habitat -Recurse -Force
  hab pkg install core/windows-service
  hab pkg exec core/windows-service install

  # Add config to HabService.dll.config
  $svcPath = Join-Path $env:SystemDrive "hab\svc\windows-service"
  [xml]$configXml = Get-Content (Join-Path $svcPath HabService.dll.config)

  # Update the arguments for the Supervisor
  $launcherArgs = $configxml | select-xml -xpath "//appSettings/add[@key='launcherArgs']"
  $launcherArgs.Node.value = "${flags}"

%{ for feature in enabled_features ~}
  $child = $configXml.CreateElement("add")
  $child.SetAttribute("key", "ENV_HAB_FEAT_${upper(feature)}")
  $child.SetAttribute("value", "true")
  $configXml.configuration.appSettings.AppendChild($child)

%{ endfor ~}
  $configXml.Save((Join-Path $svcPath HabService.dll.config))

  # Start service
  Start-Service -Name "Habitat"

  # Allow sup to update if needed
  Start-Sleep -Seconds 180

  # Without this logging doesn't seem to happen
  Restart-Service -Name "Habitat" -Force

  Write-Debug "Starting habitat/builder-worker"

  # Load builder-worker
  mkdir c:\hab\svc\builder-worker
  Set-Content -Path "c:\hab\svc\builder-worker\user.toml" -Value 'target = "x86_64-windows"'
  hab svc load habitat/builder-worker --group ${environment} --bind jobsrv:builder-jobsrv.${environment} --bind depot:builder-api-proxy.${environment} --strategy at-once --url ${bldr_url} --channel ${worker_release_channel}
</powershell>
