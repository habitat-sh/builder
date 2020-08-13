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

use std::{collections::{HashMap,
                        HashSet},
          fmt,
          iter::FromIterator,
          ops::Deref,
          path::PathBuf,
          result,
          str::FromStr,
          string::ToString};

use serde::{de,
            Deserialize,
            Deserializer,
            Serialize,
            Serializer};

use crate::{error::{Error,
                    Result},
            hab_core::{config::ConfigFile,
                       package::target::{self,
                                         PackageTarget}}};

/// Postprocessing config file name
pub const BLDR_CFG: &str = ".bldr.toml";

#[derive(Debug, Deserialize, Serialize)]
pub struct BuildCfg(HashMap<String, ProjectCfg>);

impl BuildCfg {
    pub fn from_slice(bytes: &[u8]) -> Result<Self> {
        let inner = toml::from_slice::<HashMap<String, ProjectCfg>>(bytes)
            .map_err(|e| Error::DecryptError(e.to_string()))?;
        Ok(BuildCfg(inner))
    }

    /// List of all registered projects for this `BuildCfg`.
    pub fn projects(&self) -> Vec<&ProjectCfg> { self.0.values().collect() }

    /// Returns true if the given branch & file path combination should result in a new build
    /// being automatically triggered by a GitHub Push notification.
    pub fn triggered_by<T>(&self, branch: &str, paths: &[T]) -> Vec<&ProjectCfg>
        where T: AsRef<str>
    {
        self.0
            .values()
            .filter(|p| p.triggered_by(branch, paths))
            .collect()
    }
}

impl ConfigFile for BuildCfg {
    type Error = Error;
}

impl Default for BuildCfg {
    fn default() -> Self {
        let mut cfg = HashMap::default();
        cfg.insert("default".into(), ProjectCfg::default());
        BuildCfg(cfg)
    }
}

impl Deref for BuildCfg {
    type Target = HashMap<String, ProjectCfg>;

    fn deref(&self) -> &Self::Target { &self.0 }
}

pub struct Pattern {
    inner:   glob::Pattern,
    options: glob::MatchOptions,
}

impl Pattern {
    fn default_options() -> glob::MatchOptions {
        glob::MatchOptions { case_sensitive:              false,
                             require_literal_separator:   false,
                             require_literal_leading_dot: false, }
    }

    pub fn matches<T>(&self, value: T) -> bool
        where T: AsRef<str>
    {
        self.inner.matches_with(value.as_ref(), self.options)
    }
}

impl<'de> Deserialize<'de> for Pattern {
    fn deserialize<D>(deserializer: D) -> result::Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        Pattern::from_str(&s).map_err(de::Error::custom)
    }
}

impl fmt::Debug for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.inner) }
}

impl FromStr for Pattern {
    type Err = glob::PatternError;

    fn from_str(value: &str) -> result::Result<Self, Self::Err> {
        let inner: glob::Pattern = FromStr::from_str(value)?;
        Ok(Pattern { inner,
                     options: Pattern::default_options() })
    }
}

impl Serialize for Pattern {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_str(&self.inner.to_string())
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProjectCfg {
    /// Relative filepath to the project's Habitat Plan (default: "habitat").
    #[serde(default = "ProjectCfg::default_plan_path")]
    pub plan_path:     PathBuf,
    /// Unix style file globs which are matched against changed files from a GitHub push
    /// notification to determine if an automatic rebuild should occur.
    #[serde(default)]
    pub paths:         Vec<Pattern>,
    /// Package targets to build when changes detected
    #[serde(default = "ProjectCfg::default_build_targets")]
    pub build_targets: HashSet<PackageTarget>,
}

impl ProjectCfg {
    fn default_plan_path() -> PathBuf { PathBuf::from("habitat") }

    fn default_path_pattern() -> Pattern { Pattern::from_str("*").unwrap() }

    fn default_build_targets() -> HashSet<PackageTarget> {
        HashSet::from_iter(vec![target::X86_64_WINDOWS, target::X86_64_LINUX])
    }

    // Enumerate all the possible candidates for plan file locations.
    // For example, given a plan_path of "foo/bar/habitat", the possible
    // valid paths for a plan file would be "foo/bar" and "foo/bar/habitat".
    // The same possible valid paths would be returned if the plan_path was
    // "foo/bar", or "foo/bar/plan.sh", or "foo/bar/plan.ps1", or even
    // "foo/bar/habitat/plan.sh" or "foo/bar/habitat/plan.ps1".
    // This flexibility helps us do a better job finding plans for a
    // given bldr.toml specification.
    pub fn plan_path_candidates(&self) -> Vec<PathBuf> {
        let mut candidates = Vec::new();
        let mut p = self.plan_path.clone();

        if p.ends_with("plan.sh") || p.ends_with("plan.ps1") {
            p.pop();
        }

        if p.ends_with("habitat") {
            p.pop();
        }

        candidates.push(p.clone());
        p.push("habitat");
        candidates.push(p);

        debug!("plan_path_candidates for {:?}: {:?}",
               self.plan_path, candidates);
        candidates
    }

    /// Returns true if the given branch & file path combination should result in a new build
    /// being automatically triggered by a GitHub Push notification
    fn triggered_by<T>(&self, branch: &str, paths: &[T]) -> bool
        where T: AsRef<str>
    {
        if branch != "master" {
            return false;
        }

        // Create the match patterns for all the plan path candidates
        let candidates = self.plan_path_candidates();
        let mut plan_patterns = candidates.iter().map(|p| {
            Pattern::from_str(&p.join("*").to_string_lossy()).unwrap_or_else(|_| {
                                                                 Self::default_path_pattern()
                                                             })
        });

        // Check to see if any of the passed in paths match either the plan
        // patterns or the path patterns
        paths.iter().any(|p| {
                        plan_patterns.any(|i| i.matches(p.as_ref()))
                        || self.paths.iter().any(|i| i.matches(p.as_ref()))
                    })
    }
}

impl Default for ProjectCfg {
    fn default() -> Self {
        ProjectCfg { plan_path:     ProjectCfg::default_plan_path(),
                     paths:         vec![ProjectCfg::default_path_pattern()],
                     build_targets: ProjectCfg::default_build_targets(), }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const CONFIG: &str = r#"
    [hab-sup]
    plan_path = "components/hab-sup"
    paths = [
      "components/net/*"
    ]

    [builder-api]
    plan_path = "components/builder-api/habitat"
    paths = [
      "components/net/*"
    ]
    build_targets = [ "x86_64-windows" ]

    [full-plan-path]
    plan_path = "components/builder-api/habitat/plan.sh"

    [default]
    "#;

    #[test]
    fn triggered_by() {
        let cfg = BuildCfg::from_slice(CONFIG.as_bytes()).unwrap();
        let hab_sup = cfg.get("hab-sup").unwrap();
        let bldr_api = cfg.get("builder-api").unwrap();
        let default = cfg.get("default").unwrap();

        assert!(hab_sup.triggered_by("master", &["components/hab-sup/Cargo.toml"],));
        assert!(hab_sup.triggered_by("master", &["components/hAb-Sup/Cargo.toml"],));
        assert_eq!(hab_sup.triggered_by("dev", &["components/hab-sup/Cargo.toml"]),
                   false);
        assert_eq!(hab_sup.triggered_by("master", &["components"]), false);

        assert!(bldr_api.triggered_by("master", &["components/builder-api/habitat/plan.sh"],));
        assert!(bldr_api.triggered_by("master", &["components/net/Cargo.toml"],));
        assert_eq!(bldr_api.build_targets.len(), 1);
        assert!(bldr_api.build_targets.contains(&target::X86_64_WINDOWS));

        assert!(default.triggered_by("master", &["habitat/plan.sh"]));
        assert!(default.triggered_by("master", &["habitat/hooks/init"]));
        assert_eq!(default.triggered_by("dev", &["habitat/plan.sh"]), false);
        assert_eq!(default.triggered_by("master", &["components"]), true);
    }

    #[test]
    fn triggered_by_default() {
        let cfg = BuildCfg::default();

        assert_eq!(cfg.triggered_by("dev", &["habitat/plan.sh"]).len(), 0);
        assert_eq!(cfg.triggered_by("master", &["habitat/plan.sh"]).len(), 1);
        assert_eq!(cfg.triggered_by("master", &["plan.sh"]).len(), 1);
    }

    #[test]
    fn full_plan_path() {
        let cfg = BuildCfg::from_slice(CONFIG.as_bytes()).unwrap();
        let full_plan_path = cfg.get("full-plan-path").unwrap();
        assert!(full_plan_path.plan_path.ends_with("plan.sh"));
        assert!(full_plan_path.triggered_by("master",
                                            &["components/builder-api/habitat/Cargo.toml"],));
    }
}
