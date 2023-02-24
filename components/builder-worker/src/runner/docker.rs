// Copyright (c) 2017 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[cfg(not(windows))]
use std::{path::PathBuf,
          process::{Command,
                    ExitStatus,
                    Stdio},
          str::FromStr};

#[cfg(windows)]
use std::{path::PathBuf,
          process::{Command,
                    ExitStatus,
                    Stdio},
          str::FromStr};

#[cfg(not(windows))]
use crate::hab_core::{env,
                      fs as hfs,
                      fs::FS_ROOT_PATH,
                      package::{ident::PackageIdent,
                                PackageInstall}};

#[cfg(windows)]
use crate::hab_core::{env,
                      fs as hfs,
                      fs::FS_ROOT_PATH,
                      package::{ident::PackageIdent,
                                PackageInstall}};

use crate::error::{Error,
                   Result};

use crate::runner::{job_streamer::JobStreamer,
                    studio::WINDOWS_ENVVARS,
                    workspace::Workspace,
                    NONINTERACTIVE_ENVVAR,
                    RUNNER_DEBUG_ENVVAR};

lazy_static! {
    /// Absolute path to the Docker exporter program
    static ref CONTAINER_EXPORTER_PROGRAM: PathBuf = hfs::resolve_cmd_in_pkg(
        "hab-pkg-export-container",
        include_str!(concat!(env!("OUT_DIR"), "/CONTAINER_EXPORTER_PKG_IDENT")),
    );

    /// Absolute path to the Dockerd program
    static ref DOCKERD_PROGRAM: PathBuf = hfs::resolve_cmd_in_pkg(
        "dockerd",
        include_str!(concat!(env!("OUT_DIR"), "/DOCKER_PKG_IDENT")),
    );
}

pub struct DockerExporterSpec {
    pub username:             String,
    pub password:             String,
    pub registry_type:        String,
    pub registry_url:         Option<String>,
    pub docker_hub_repo_name: String,
    pub latest_tag:           bool,
    pub version_tag:          bool,
    pub version_release_tag:  bool,
    pub custom_tag:           Option<String>,
}

pub struct DockerExporter<'a> {
    spec:       DockerExporterSpec,
    workspace:  &'a Workspace,
    bldr_url:   &'a str,
    auth_token: &'a str,
}

impl<'a> DockerExporter<'a> {
    /// Creates a new Docker exporter for a given `Workspace` and Builder URL.
    pub fn new(spec: DockerExporterSpec,
               workspace: &'a Workspace,
               bldr_url: &'a str,
               auth_token: &'a str)
               -> Self {
        DockerExporter { spec,
                         workspace,
                         bldr_url,
                         auth_token }
    }

    /// Spawns a Docker export command, sends output streams to the given `LogStreamer` and returns
    /// the process' `ExitStatus`.
    ///
    /// # Errors
    ///
    /// * If the child process can't be spawned
    /// * If the calling thread can't wait on the child process
    /// * If the `LogStreamer` fails to stream outputs
    pub fn export(&self, streamer: &mut JobStreamer) -> Result<ExitStatus> {
        // TODO: We should determine what broke this behavior and restore it
        self.run_export(streamer)
    }

    fn run_export(&self, streamer: &mut JobStreamer) -> Result<ExitStatus> {
        debug!("Using pre-configured container exporter program: {:?}",
               &*CONTAINER_EXPORTER_PROGRAM);

        let mut cmd = Command::new(&*CONTAINER_EXPORTER_PROGRAM);

        let exporter_ident = PackageIdent::from_str("core/hab-pkg-export-container")?;
        let pkg_install = PackageInstall::load(&exporter_ident, Some(&*FS_ROOT_PATH))?;

        cmd.current_dir(self.workspace.root());
        cmd.arg("--image-name");
        cmd.arg(&self.spec.docker_hub_repo_name);
        cmd.arg("--base-pkgs-url");
        cmd.arg(self.bldr_url);
        cmd.arg("--url");
        cmd.arg(self.bldr_url);
        cmd.arg("--auth");
        cmd.arg(self.auth_token);
        if self.spec.latest_tag {
            cmd.arg("--tag-latest");
        }
        if self.spec.version_tag {
            cmd.arg("--tag-version");
        }
        if self.spec.version_release_tag {
            cmd.arg("--tag-version-release");
        }
        if let Some(ref custom_tag) = self.spec.custom_tag {
            cmd.arg("--tag-custom");
            cmd.arg(custom_tag);
        }
        cmd.arg("--push-image");
        cmd.arg("--username");
        cmd.arg(&self.spec.username);
        cmd.arg("--password");
        cmd.arg(&self.spec.password);
        cmd.arg("--rm-image");
        if let Some(ref registry_url) = self.spec.registry_url {
            cmd.arg("--registry-url");
            cmd.arg(registry_url);
        }
        cmd.arg("--registry-type");
        cmd.arg(&self.spec.registry_type);

        cmd.arg(self.workspace.last_built()?.path); // Locally built artifact
        debug!(
            "building container export command, cmd={}",
            format!("building container export command, cmd={:?}", &cmd)
                .replace(&self.spec.username, "<username-redacted>")
                .replace(&self.spec.password, "<password-redacted>")
        );

        if cfg!(not(windows)) {
            cmd.env_clear();
            let cmd_env = pkg_install.environment_for_command()?;

            for (key, value) in cmd_env.into_iter() {
                debug!("Setting: {}='{}'", key, value);
                cmd.env(key, value);
            }
        } else {
            for var in WINDOWS_ENVVARS {
                if let Some(val) = env::var_os(var) {
                    debug!("Setting {} to {:?}", var, val);
                    cmd.env(var, val);
                } else {
                    debug!("{} env var not found!", var);
                }
            }
        }
        if env::var_os(RUNNER_DEBUG_ENVVAR).is_some() {
            cmd.env("RUST_LOG", "debug");
        }

        cmd.env(NONINTERACTIVE_ENVVAR, "true"); // Disables progress bars
        cmd.env("TERM", "xterm-256color"); // Emits ANSI color codes

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        debug!("spawning container export command");
        let mut child = cmd.spawn().map_err(Error::Exporter)?;
        streamer.consume_child(&mut child)?;
        let exit_status = child.wait().map_err(Error::Exporter)?;
        debug!("completed container export command, status={:?}",
               exit_status);

        Ok(exit_status)
    }
}
