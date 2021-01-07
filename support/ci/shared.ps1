function Get-NightlyToolchain {
  # It turns out that every nightly version of rustfmt has slight tweaks from the previous version.
  # This means that if we're always using the latest version, then we're going to have enormous
  # churn. Even PRs that don't touch rust code will likely fail CI, since master will have been
  # formatted with a different version than is running in CI. Because of this, we're going to pin
  # the version of nightly that's used to run rustfmt and bump it when we do a new release.
  #
  # Note that not every nightly version of rust includes rustfmt. Sometimes changes are made that
  # break the way rustfmt uses rustc. Therefore, before updating the pin below, double check
  # that the nightly version you're going to update it to includes rustfmt. You can do that
  # using https://mexus.github.io/rustup-components-history/x86_64-unknown-linux-gnu.html
  Get-Content "$PSScriptRoot\..\..\RUSTFMT_VERSION"
}

function Install-Rustup($Toolchain) {
    if(Test-Path $env:USERPROFILE\.cargo\bin) {
        $env:path = New-PathString -StartingPath $env:path -Path "$env:USERPROFILE\.cargo\bin"
    }

    if (get-command -Name rustup.exe -ErrorAction SilentlyContinue) {
        Write-Host "rustup is currently installed"
        rustup set default-host x86_64-pc-windows-msvc
        rustup default stable-x86_64-pc-windows-msvc
    } else {
        Write-Host "Installing rustup and $toolchain-x86_64-pc-windows-msvc Rust."
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        invoke-restmethod -usebasicparsing 'https://static.rust-lang.org/rustup/dist/i686-pc-windows-gnu/rustup-init.exe' -outfile 'rustup-init.exe'
        ./rustup-init.exe -y --default-toolchain $toolchain-x86_64-pc-windows-msvc --no-modify-path
        $env:path += ";$env:USERPROFILE\.cargo\bin"
    }
}

function Get-RustfmtToolchain {
    # It turns out that every nightly version of rustfmt has slight tweaks from the previous version.
    # This means that if we're always using the latest version, then we're going to have enormous
    # churn. Even PRs that don't touch rust code will likely fail CI, since master will have been
    # formatted with a different version than is running in CI. Because of this, we're going to pin
    # the version of nightly that's used to run rustfmt and bump it when we do a new release.
    #
    # Note that not every nightly version of rust includes rustfmt. Sometimes changes are made that
    # break the way rustfmt uses rustc. Therefore, before updating the pin below, double check
    # that the nightly version you're going to update it to includes rustfmt. You can do that
    # using https://mexus.github.io/rustup-components-history/x86_64-unknown-linux-gnu.html
    "$(Get-Content $PSScriptRoot\..\..\RUSTFMT_VERSION)-x86_64-pc-windows-msvc"
}

function Get-Toolchain {
    "$(Get-Content $PSScriptRoot\..\..\rust-toolchain)"
}

function Install-RustToolchain($Toolchain) {
    rustup component list --toolchain $toolchain | Out-Null
    if ($LASTEXITCODE -ne 0) {
        Write-Host "Installing rust toolchain $toolchain"
        rustup toolchain install $toolchain
    } else {
        Write-Host "Rust toolchain $toolchain is already installed"
    }
}

function Install-Rustfmt($Toolchain) {
  local toolchain="${1?toolchain argument required}"
  Install-RustToolchain $Toolchain
  rustup component add --toolchain $Toolchain rustfmt
}

function Install-Habitat {
    if (-not (get-command choco -ErrorAction SilentlyContinue)) {
        Write-Host "Installing Chocolatey"
        Invoke-Expression ((New-Object System.Net.WebClient).DownloadString('https://chocolatey.org/install.ps1')) | out-null
    }

    if (!((choco list habitat --local-only) -match '^1 packages installed\.$')) {
        choco install habitat -y
    }
}

function Install-HabPkg([string[]]$idents) {
  $idents | % {
      $id = $_
      $installedPkgs=hab pkg list $id | ? { $_.StartsWith($id)}

      if($installedPkgs){
          Write-host "$id already installed"
      } else {
          hab pkg install $id
      }
  }
}

function New-PathString([string]$StartingPath, [string]$Path) {
    if (-not [string]::IsNullOrEmpty($path)) {
        if (-not [string]::IsNullOrEmpty($StartingPath)) {
            [string[]]$PathCollection = "$path;$StartingPath" -split ';'
            $Path = ($PathCollection |
                Select-Object -Unique |
                Where-Object {-not [string]::IsNullOrEmpty($_.trim())} |
                Where-Object {test-path "$_"}
            ) -join ';'
        }
        $path
    }
    else {
        $StartingPath
    }
}

function Setup-Environment {
    $env:HAB_LICENSE = "accept-no-persist"
    Install-Habitat

    Install-HabPkg @(
        "core/cacerts",
        "core/libarchive",
        "core/protobuf",
        "core/visual-cpp-build-tools-2015",
        "core/xz",
        "core/zeromq",
        "core/zlib",
        "core/perl"
    )

    # Set up some path variables for ease of use later
    $cacertsDir     = & hab pkg path core/cacerts
    $libarchiveDir  = & hab pkg path core/libarchive
    $protobufDir    = & hab pkg path core/protobuf
    $xzDir          = & hab pkg path core/xz
    $zeromqDir      = & hab pkg path core/zeromq
    $zlibDir        = & hab pkg path core/zlib
    $perl           = & hab pkg path core/perl

    # Set some required variables
    $env:LIBARCHIVE_INCLUDE_DIR     = "$libarchiveDir\include"
    $env:LIBARCHIVE_LIB_DIR         = "$libarchiveDir\lib"
    $env:LIBZMQ_PREFIX              = "$zeromqDir"
    $env:SSL_CERT_FILE              = "$cacertsDir\ssl\certs\cacert.pem"
    $env:LD_LIBRARY_PATH            = "$env:LIBZMQ_PREFIX\lib;$zlibDir\lib;$xzDir\lib"
    $env:PATH                       = New-PathString -StartingPath $env:PATH -Path "$protobufDir\bin;$zeromqDir\bin;$libarchiveDir\bin;$zlibDir\bin;$xzDir\bin;$perl\bin"

    $vsDir = & hab pkg path core/visual-cpp-build-tools-2015
    $env:DisableRegistryUse="true"
    $env:UseEnv="true"
    $env:VisualStudioVersion = "14.0"
    $env:WindowsSdkDir_81="$vsDir\Windows Kits\8.1"
    $env:VCTargetsPath="$vsDir\Program Files\MSBuild\Microsoft.Cpp\v4.0\v140"
    $env:VcInstallDir="$vsDir\Program Files\Microsoft Visual Studio 14.0\VC"
    $env:CLTrackerSdkPath="$vsDir\Program Files\MSBuild\14.0\bin\amd64"
    $env:CLTrackerFrameworkPath="$vsDir\Program Files\MSBuild\14.0\bin\amd64"
    $env:LinkTrackerSdkPath="$vsDir\Program Files\MSBuild\14.0\bin\amd64"
    $env:LinkTrackerFrameworkPath="$vsDir\Program Files\MSBuild\14.0\bin\amd64"
    $env:LibTrackerSdkPath="$vsDir\Program Files\MSBuild\14.0\bin\amd64"
    $env:LibTrackerFrameworkPath="$vsDir\Program Files\MSBuild\14.0\bin\amd64"
    $env:RCTrackerSdkPath="$vsDir\Program Files\MSBuild\14.0\bin\amd64"
    $env:RCTrackerFrameworkPath="$vsDir\Program Files\MSBuild\14.0\bin\amd64"
    $env:LIB = "$(Get-Content "$vsDir\LIB_DIRS")"
    $env:INCLUDE = (Get-Content "$vsDir\INCLUDE_DIRS")
    $env:PATH = New-PathString -StartingPath $env:PATH -Path (Get-Content "$vsDir\PATH")
}

# On buildkite, the rust binaries will be directly in C:
if($env:BUILDKITE) {
    # this will avoid a path length limit from the long buildkite working dir path
    $env:CARGO_TARGET_DIR = "c:\target"
}
