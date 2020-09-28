use crate::{error::{Error,
                    Result},
            hab_core::{env::{self,
                             Config},
                       fs,
                       package::target::{self,
                                         PackageTarget},
                       url::BLDR_URL_ENVVAR,
                       ChannelIdent,
                       AUTH_TOKEN_ENVVAR},
            runner::{job_streamer::JobStreamer,
                     workspace::Workspace,
                     DEV_MODE,
                     NONINTERACTIVE_ENVVAR,
                     RUNNER_DEBUG_ENVVAR}};
use std::{path::PathBuf,
          process::{Child,
                    Command,
                    Stdio},
          sync::{atomic::AtomicUsize,
                 Mutex}};

pub static STUDIO_UID: AtomicUsize = AtomicUsize::new(0);
pub static STUDIO_GID: AtomicUsize = AtomicUsize::new(0);
pub const DEBUG_ENVVARS: &[&str] = &["RUST_LOG", "DEBUG", "RUST_BACKTRACE"];
pub const WINDOWS_ENVVARS: &[&str] = &["SYSTEMDRIVE", "USERNAME", "COMPUTERNAME", "TEMP"];

lazy_static! {
    /// Absolute path to the Studio program
    static ref STUDIO_PROGRAM: PathBuf = fs::resolve_cmd_in_pkg(
        "hab-studio",
        include_str!(concat!(env!("OUT_DIR"), "/STUDIO_PKG_IDENT")),
    );

    /// Absolute path to the hab cli
    static ref HAB_CLI: PathBuf = fs::resolve_cmd_in_pkg(
        "hab",
        include_str!(concat!(env!("OUT_DIR"), "/HAB_PKG_IDENT")),
    );

    pub static ref STUDIO_HOME: Mutex<PathBuf> = {
        Mutex::new(PathBuf::new())
    };
}

pub struct Studio<'a> {
    workspace:  &'a Workspace,
    bldr_url:   &'a str,
    auth_token: &'a str,
    target:     PackageTarget,
}

impl<'a> Studio<'a> {
    /// Creates a new Studio runner for a given `Workspace` and Builder URL.
    pub fn new(workspace: &'a Workspace,
               bldr_url: &'a str,
               auth_token: &'a str,
               target: PackageTarget)
               -> Self {
        Studio { workspace,
                 bldr_url,
                 auth_token,
                 target }
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
        let dev_mode = if let Some(_val) = env::var_os(DEV_MODE) {
            debug!("RUNNER_DEBUG_ENVVAR ({}) is set - using non-Docker studio",
                   DEV_MODE);
            true
        } else {
            false
        };

        let channel = if self.workspace.job.has_channel() {
            ChannelIdent::from(self.workspace.job.get_channel())
        } else {
            ChannelIdent::stable()
        };

        let mut cmd = self.studio_command()?;
        cmd.current_dir(self.workspace.src());
        if dev_mode && cfg!(not(windows)) {
            cmd.env("HOME", "/hab/svc/builder-worker/data");
        }

        if let Some(val) = env::var_os(RUNNER_DEBUG_ENVVAR) {
            debug!("RUNNER_DEBUG_ENVVAR ({}) is set - turning on runner debug",
                   RUNNER_DEBUG_ENVVAR);
            cmd.env("DEBUG", val);
        }
        cmd.env("PATH",
                env::var("PATH").unwrap_or_else(|_| String::from(""))); // Sets `$PATH`
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

        // TODO JB: remove the HAB_STUDIO_SECRET_HAB_LICENSE line after our (n-1) version exceeds
        // 0.81.0
        cmd.env("HAB_LICENSE", "accept-no-persist");
        cmd.env("HAB_STUDIO_SECRET_HAB_LICENSE", "accept-no-persist");

        cmd.env("HAB_DOCKER_OPTS", "--name builder");

        for secret in self.workspace.job.get_secrets() {
            cmd.env(format!("HAB_STUDIO_SECRET_{}",
                            secret.get_decrypted_secret().get_name()),
                    secret.get_decrypted_secret().get_value());
        }

        cmd.env("HAB_ORIGIN", self.workspace.job.origin());

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


        cmd.arg("studio");
        cmd.arg("build");

        if !dev_mode {
            cmd.arg("-D"); // Use Docker studio
        }

        if self.target == target::X86_64_WINDOWS {
            cmd.arg("-R"); // Work around a bug so studio does not get removed
                           // Remove when we fix this (hab 0.75.0 or later)
            cmd.arg("-k"); // Origin key
            cmd.arg(self.workspace.job.origin());
        }

        cmd.arg(build_path(self.workspace.job.get_project().get_plan_path()));
        debug!("building studio build command, cmd={:?}", &cmd);
        debug!("setting studio build command env, {}={}",
               ChannelIdent::ENVVAR,
               &channel);
        cmd.env(ChannelIdent::ENVVAR, channel.as_str());
        debug!("setting studio build command env, {}={}",
               BLDR_URL_ENVVAR, self.bldr_url);
        cmd.env(BLDR_URL_ENVVAR, self.bldr_url);
        cmd.env(AUTH_TOKEN_ENVVAR, self.auth_token);

        debug!("spawning studio build command");
        let mut child =
            cmd.spawn()
               .map_err(|e| Error::StudioBuild(self.workspace.studio().to_path_buf(), e))?;

        streamer.consume_child(&mut child)?;
        Ok(child)
    }

    fn studio_command(&self) -> Result<Command> {
        let mut cmd = Command::new(&*HAB_CLI);
        if cfg!(not(windows)) {
            cmd.env_clear();
        }

        debug!("HAB_CACHE_KEY_PATH: {:?}", self.workspace.key_path());
        cmd.env("NO_ARTIFACT_PATH", "true"); // Disables artifact cache mounting
        cmd.env("HAB_CACHE_KEY_PATH", self.workspace.key_path()); // Sets key cache to build user's home

        Ok(cmd)
    }
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
        assert_eq!("components/yep",
                   build_path("components/yep/habitat/plan.sh"));
    }

    #[test]
    fn build_path_with_subdir_habitat_plan_ps1() {
        assert_eq!("components/yep",
                   build_path("components/yep/habitat/plan.ps1"));
    }
}
