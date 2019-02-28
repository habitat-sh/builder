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
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
#[cfg(not(windows))]
use std::sync::atomic::Ordering;
use std::sync::atomic::{AtomicUsize, ATOMIC_USIZE_INIT};
use std::sync::Mutex;

use crate::hab_core::env::{self, Config};
use crate::hab_core::fs;
use crate::hab_core::package::target::{self, PackageTarget};
use crate::hab_core::url::BLDR_URL_ENVVAR;
use crate::hab_core::ChannelIdent;
use crate::hab_core::AUTH_TOKEN_ENVVAR;

use crate::error::{Error, Result};
use crate::network::NetworkNamespace;
use crate::runner::job_streamer::JobStreamer;
use crate::runner::workspace::Workspace;
use crate::runner::{NONINTERACTIVE_ENVVAR, RUNNER_DEBUG_ENVVAR};

pub static STUDIO_UID: AtomicUsize = ATOMIC_USIZE_INIT;
pub static STUDIO_GID: AtomicUsize = ATOMIC_USIZE_INIT;
pub const DEBUG_ENVVARS: &[&str] = &["RUST_LOG", "DEBUG", "RUST_BACKTRACE"];
pub const WINDOWS_ENVVARS: &[&str] = &["SYSTEMDRIVE", "USERNAME", "COMPUTERNAME", "TEMP"];
pub const STUDIO_USER: &str = "krangschnak";
pub const STUDIO_GROUP: &str = "krangschnak";

lazy_static! {
    /// Absolute path to the Studio program
    static ref STUDIO_PROGRAM: PathBuf = fs::resolve_cmd_in_pkg(
        "hab-studio",
        include_str!(concat!(env!("OUT_DIR"), "/STUDIO_PKG_IDENT")),
    );

    pub static ref STUDIO_HOME: Mutex<PathBuf> = {
        Mutex::new(PathBuf::new())
    };
}

pub struct Studio<'a> {
    workspace: &'a Workspace,
    bldr_url: &'a str,
    auth_token: &'a str,
    airlock_enabled: bool,
    network_namespace: Option<NetworkNamespace>,
    target: PackageTarget,
}

impl<'a> Studio<'a> {
    /// Creates a new Studio runner for a given `Workspace` and Builder URL.
    pub fn new(
        workspace: &'a Workspace,
        bldr_url: &'a str,
        auth_token: &'a str,
        airlock_enabled: bool,
        network_namespace: Option<NetworkNamespace>,
        target: PackageTarget,
    ) -> Self {
        Studio {
            workspace,
            bldr_url,
            auth_token,
            airlock_enabled,
            network_namespace,
            target,
        }
    }

    /// Spawns a Studio build command, pipes output streams to the given `LogPipe` and returns the
    /// process' `ExitStatus`.
    ///
    /// # Errors
    ///
    /// * If the child process can't be spawned
    /// * If the calling thread can't wait on the child process
    /// * If the `LogPipe` fails to pipe output
    pub fn build(&self, streamer: &mut JobStreamer) -> Result<Child> {
        let channel = if self.workspace.job.has_channel() {
            ChannelIdent::from(self.workspace.job.get_channel())
        } else {
            ChannelIdent::stable()
        };

        let mut cmd = self.studio_command()?;
        cmd.current_dir(self.workspace.src());
        if let Some(val) = env::var_os(RUNNER_DEBUG_ENVVAR) {
            debug!(
                "RUNNER_DEBUG_ENVVAR ({}) is set - turning on runner debug",
                RUNNER_DEBUG_ENVVAR
            );
            cmd.env("DEBUG", val);
        }
        cmd.env(
            "PATH",
            env::var("PATH").unwrap_or_else(|_| String::from("")),
        ); // Sets `$PATH`
        cmd.env(NONINTERACTIVE_ENVVAR, "true"); // Disables progress bars
        cmd.env("TERM", "xterm-256color"); // Emits ANSI color codes

        // Tells workers to ignore any locally-installed dependencies,
        // and to always use what's in Builder
        cmd.env("HAB_FEAT_IGNORE_LOCAL", "true");
        // Ideally, we would just pass any `HAB_FEAT_*` flags into the
        // studio directly, since we know they're "ours". Until we do,
        // however, we'll need to prefix it with `HAB_STUDIO_SECRET_`.
        //
        // Follow https://github.com/habitat-sh/habitat/issues/5274
        // for progress on this front.
        cmd.env("HAB_STUDIO_SECRET_HAB_FEAT_IGNORE_LOCAL", "true");

        for secret in self.workspace.job.get_secrets() {
            cmd.env(
                format!(
                    "HAB_STUDIO_SECRET_{}",
                    secret.get_decrypted_secret().get_name()
                ),
                secret.get_decrypted_secret().get_value(),
            );
        }

        if self.target == target::X86_64_LINUX_KERNEL2 {
            cmd.env("HAB_ORIGIN", self.workspace.job.origin());
        }

        if cfg!(windows) {
            for var in WINDOWS_ENVVARS {
                if let Some(val) = env::var_os(var) {
                    debug!("Setting {} to {:?}", var, val);
                    cmd.env(var, val);
                } else {
                    debug!("{} env var not found!", var);
                }
            }
        }

        // propagate debugging environment variables into Airlock and Studio
        for var in DEBUG_ENVVARS {
            if let Ok(val) = env::var(var) {
                cmd.env(var, val);
            }
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // generation of the build command is different for each
        // platform atm, but will eventually be simplified to be
        // a single workflow
        match self.target {
            target::X86_64_LINUX => {
                cmd.arg("-k"); // Origin key
                cmd.arg(self.workspace.job.origin());
                cmd.arg("build");
            }
            target::X86_64_LINUX_KERNEL2 => {
                cmd.arg("studio");
                cmd.arg("build");
                cmd.arg("-D"); // Use Docker studio
            }
            target::X86_64_WINDOWS => {
                cmd.arg("studio");
                cmd.arg("build");
                cmd.arg("-D"); // Use Docker studio
                cmd.arg("-R"); // Work around a bug so studio does not get removed
                               // Remove when we fix this (hab 0.75.0 or later)
                               // TODO (SA): Consider using Docker studio for Linux builds as well
                cmd.arg("-k"); // Origin key
                cmd.arg(self.workspace.job.origin());
            }
            _ => unreachable!("Unexpected platform for build worker"),
        }

        cmd.arg(build_path(self.workspace.job.get_project().get_plan_path()));
        debug!("building studio build command, cmd={:?}", &cmd);
        debug!(
            "setting studio build command env, {}={}",
            ChannelIdent::ENVVAR,
            &channel
        );
        cmd.env(ChannelIdent::ENVVAR, channel.as_str());
        debug!(
            "setting studio build command env, {}={}",
            BLDR_URL_ENVVAR, self.bldr_url
        );
        cmd.env(BLDR_URL_ENVVAR, self.bldr_url);
        cmd.env(AUTH_TOKEN_ENVVAR, self.auth_token);

        debug!("spawning studio build command");
        let mut child = cmd
            .spawn()
            .map_err(|e| Error::StudioBuild(self.workspace.studio().to_path_buf(), e))?;

        streamer.consume_child(&mut child)?;
        Ok(child)
    }

    #[cfg(windows)]
    fn studio_command(&self) -> Result<Command> {
        self.studio_command_not_airlock()
    }

    #[cfg(not(windows))]
    fn studio_command(&self) -> Result<Command> {
        if self.airlock_enabled && (self.target == target::X86_64_LINUX) {
            self.studio_command_airlock()
        } else {
            self.studio_command_not_airlock()
        }
    }

    #[cfg(not(windows))]
    fn studio_command_airlock(&self) -> Result<Command> {
        assert!(self.airlock_enabled);

        let mut cmd = Command::new("airlock");
        cmd.uid(studio_uid());
        cmd.gid(studio_gid());
        cmd.env_clear();
        cmd.env("HOME", &*STUDIO_HOME.lock().unwrap()); // Sets `$HOME` for build user
        cmd.env("USER", STUDIO_USER); // Sets `$USER` for build user
        cmd.arg("run");
        cmd.arg("--fs-root");
        cmd.arg(self.workspace.studio());
        cmd.arg("--no-rm");
        if self.network_namespace.is_some() {
            cmd.arg("--use-userns");
            cmd.arg(self.network_namespace.as_ref().unwrap().userns());
            cmd.arg("--use-netns");
            cmd.arg(self.network_namespace.as_ref().unwrap().netns());
        }
        cmd.arg(&*STUDIO_PROGRAM);

        Ok(cmd)
    }

    #[cfg(not(windows))]
    fn studio_command_not_airlock(&self) -> Result<Command> {
        let mut cmd = if self.target == target::X86_64_LINUX {
            Command::new(&*STUDIO_PROGRAM)
        } else {
            Command::new(&"hab") // Linux2 builder uses the hab cli instead of the
                                 // explict STUDIO_PROGRAM path. This is required for
                                 // use of Docker studio.
                                 // TODO (SA): Consider the same change for Linux
        };
        cmd.env_clear();

        debug!("HAB_CACHE_KEY_PATH: {:?}", key_path());
        cmd.env("NO_ARTIFACT_PATH", "true"); // Disables artifact cache mounting
        cmd.env("HAB_CACHE_KEY_PATH", key_path()); // Sets key cache to build user's home

        info!("Airlock is not enabled, running uncontained Studio");
        Ok(cmd)
    }

    #[cfg(windows)]
    fn studio_command_not_airlock(&self) -> Result<Command> {
        let mut cmd = Command::new(&"hab"); // Windows builder uses the hab cli instead of the
                                            // explict STUDIO_PROGRAM path. This is required for
                                            // use of Docker studio.
                                            // TODO (SA): Consider the same change for Linux

        // Note - Windows builder does not clear the env
        // TODO (SA): Consider the same change for Linux

        debug!("HAB_CACHE_KEY_PATH: {:?}", key_path());
        cmd.env("NO_ARTIFACT_PATH", "true"); // Disables artifact cache mounting
        cmd.env("HAB_CACHE_KEY_PATH", key_path()); // Sets key cache to build user's home

        Ok(cmd)
    }
}

#[cfg(not(windows))]
pub fn studio_gid() -> u32 {
    STUDIO_GID.load(Ordering::Relaxed) as u32
}

#[cfg(not(windows))]
pub fn studio_uid() -> u32 {
    STUDIO_UID.load(Ordering::Relaxed) as u32
}

#[cfg(not(windows))]
pub fn set_studio_gid(gid: u32) {
    STUDIO_GID.store(gid as usize, Ordering::Relaxed);
}

#[cfg(not(windows))]
pub fn set_studio_uid(uid: u32) {
    STUDIO_UID.store(uid as usize, Ordering::Relaxed);
}

pub fn key_path() -> PathBuf {
    (&*STUDIO_HOME)
        .lock()
        .unwrap()
        .join(format!(".{}", fs::CACHE_KEY_PATH))
}

/// Returns a path argument suitable to pass to a Studio build command.
pub fn build_path(plan_path: &str) -> String {
    debug!("Creating build_path from plan_path {}", plan_path);
    let mut parts: Vec<_> = plan_path.split('/').collect();
    if parts.last().map_or("", |p| *p) == "plan.sh" {
        parts.pop();
    }
    if parts.last().map_or("", |p| *p) == "plan.ps1" {
        parts.pop();
    }
    if parts.last().map_or("", |p| *p) == "habitat" {
        parts.pop();
    }

    let ret = if parts.is_empty() {
        String::from(".")
    } else {
        parts.join("/")
    };
    debug!("build_path is {}", ret);
    ret
}

#[cfg(test)]
mod tests {
    use super::build_path;

    #[test]
    fn build_path_with_plan_sh() {
        assert_eq!(".", build_path("plan.sh"));
    }

    #[test]
    fn build_path_with_plan_ps1() {
        assert_eq!(".", build_path("plan.ps1"));
    }

    #[test]
    fn build_path_with_habitat_plan_sh() {
        assert_eq!(".", build_path("habitat/plan.sh"));
    }

    #[test]
    fn build_path_with_habitat_plan_ps1() {
        assert_eq!(".", build_path("habitat/plan.ps1"));
    }

    #[test]
    fn build_path_with_subdir_plan_sh() {
        assert_eq!("haaay", build_path("haaay/plan.sh"));
    }

    #[test]
    fn build_path_with_subdir_plan_ps1() {
        assert_eq!("haaay", build_path("haaay/plan.ps1"));
    }

    #[test]
    fn build_path_with_subdir_habitat_plan_sh() {
        assert_eq!(
            "components/yep",
            build_path("components/yep/habitat/plan.sh")
        );
    }

    #[test]
    fn build_path_with_subdir_habitat_plan_ps1() {
        assert_eq!(
            "components/yep",
            build_path("components/yep/habitat/plan.ps1")
        );
    }
}
