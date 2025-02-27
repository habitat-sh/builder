// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
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

use std::{fmt,
          result,
          str::FromStr};

use serde::{ser::SerializeStruct,
            Serialize,
            Serializer};

use crate::{error::ProtocolError,
            message::originsrv::OriginPackage};

pub use crate::message::{jobsrv::*,
                         originsrv};

pub const GITHUB_PUSH_NOTIFY_ID: u64 = 23;

impl fmt::Display for JobGroupTrigger {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let value = match *self {
            JobGroupTrigger::Unknown => "Unknown",
            JobGroupTrigger::Webhook => "Webhook",
            JobGroupTrigger::Upload => "Upload",
            JobGroupTrigger::HabClient => "HabClient",
            JobGroupTrigger::BuilderUI => "BuilderUI",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for JobGroupTrigger {
    type Err = ProtocolError;

    fn from_str(value: &str) -> result::Result<Self, Self::Err> {
        match value.to_lowercase().as_ref() {
            "unknown" => Ok(JobGroupTrigger::Unknown),
            "webhook" => Ok(JobGroupTrigger::Webhook),
            "upload" => Ok(JobGroupTrigger::Upload),
            "habclient" => Ok(JobGroupTrigger::HabClient),
            "builderui" => Ok(JobGroupTrigger::BuilderUI),
            _ => Err(ProtocolError::BadJobGroupState(value.to_string())),
        }
    }
}

impl From<JobGraphPackagePreCreate> for OriginPackage {
    fn from(value: JobGraphPackagePreCreate) -> OriginPackage {
        let mut package = OriginPackage::new();

        let name = value.get_ident().to_string();
        let target = value.get_target().to_string();

        let deps = value.get_deps()
                        .iter()
                        .map(|x| originsrv::OriginPackageIdent::from_str(x).unwrap())
                        .collect();

        let build_deps = value.get_build_deps()
                              .iter()
                              .map(|x| originsrv::OriginPackageIdent::from_str(x).unwrap())
                              .collect();

        package.set_ident(originsrv::OriginPackageIdent::from_str(&name).unwrap());
        package.set_target(target);
        package.set_deps(deps);
        package.set_build_deps(build_deps);
        package
    }
}

impl fmt::Display for JobGroupState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let value = match *self {
            JobGroupState::GroupDispatching => "Dispatching",
            JobGroupState::GroupPending => "Pending",
            JobGroupState::GroupComplete => "Complete",
            JobGroupState::GroupFailed => "Failed",
            JobGroupState::GroupQueued => "Queued",
            JobGroupState::GroupCanceled => "Canceled",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for JobGroupState {
    type Err = ProtocolError;

    fn from_str(value: &str) -> result::Result<Self, Self::Err> {
        match value.to_lowercase().as_ref() {
            "dispatching" => Ok(JobGroupState::GroupDispatching),
            "pending" => Ok(JobGroupState::GroupPending),
            "complete" => Ok(JobGroupState::GroupComplete),
            "failed" => Ok(JobGroupState::GroupFailed),
            "queued" => Ok(JobGroupState::GroupQueued),
            "canceled" => Ok(JobGroupState::GroupCanceled),
            _ => Err(ProtocolError::BadJobGroupState(value.to_string())),
        }
    }
}

impl Serialize for JobGroupState {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
        where S: Serializer
    {
        match *self as u64 {
            0 => serializer.serialize_str("Pending"),
            1 => serializer.serialize_str("Dispatching"),
            2 => serializer.serialize_str("Complete"),
            3 => serializer.serialize_str("Failed"),
            4 => serializer.serialize_str("Queued"),
            5 => serializer.serialize_str("Canceled"),
            _ => panic!("Unexpected enum value"),
        }
    }
}

impl fmt::Display for JobGroupProjectState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let value = match *self {
            JobGroupProjectState::NotStarted => "NotStarted",
            JobGroupProjectState::InProgress => "InProgress",
            JobGroupProjectState::Success => "Success",
            JobGroupProjectState::Failure => "Failure",
            JobGroupProjectState::Skipped => "Skipped",
            JobGroupProjectState::Canceled => "Canceled",
            JobGroupProjectState::NotFound => "NotFound",
        };
        write!(f, "{}", value)
    }
}

impl FromStr for JobGroupProjectState {
    type Err = ProtocolError;

    fn from_str(value: &str) -> result::Result<Self, Self::Err> {
        match value.to_lowercase().as_ref() {
            "notstarted" => Ok(JobGroupProjectState::NotStarted),
            "inprogress" => Ok(JobGroupProjectState::InProgress),
            "success" => Ok(JobGroupProjectState::Success),
            "failure" => Ok(JobGroupProjectState::Failure),
            "skipped" => Ok(JobGroupProjectState::Skipped),
            "canceled" => Ok(JobGroupProjectState::Canceled),
            "notfound" => Ok(JobGroupProjectState::NotFound),
            _ => Err(ProtocolError::BadJobGroupProjectState(value.to_string())),
        }
    }
}

impl Serialize for JobGroupProjectState {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
        where S: Serializer
    {
        match *self as u64 {
            0 => serializer.serialize_str("NotStarted"),
            1 => serializer.serialize_str("InProgress"),
            2 => serializer.serialize_str("Success"),
            3 => serializer.serialize_str("Failure"),
            4 => serializer.serialize_str("Skipped"),
            5 => serializer.serialize_str("Canceled"),
            6 => serializer.serialize_str("NotFound"),
            _ => panic!("Unexpected enum value"),
        }
    }
}

impl Serialize for JobGroupProject {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut strukt = serializer.serialize_struct("job_group_project", 4)?;
        strukt.serialize_field("name", &self.get_name())?;
        strukt.serialize_field("ident", &self.get_ident())?;
        strukt.serialize_field("state", &self.get_state())?;
        strukt.serialize_field("job_id", &self.get_job_id().to_string())?;
        strukt.serialize_field("target", &self.get_target())?;
        strukt.end()
    }
}

impl Serialize for JobGroup {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut strukt = serializer.serialize_struct("job_group", 5)?;
        strukt.serialize_field("id", &self.get_id().to_string())?;
        strukt.serialize_field("state", &self.get_state())?;
        strukt.serialize_field("projects", &self.get_projects())?;
        strukt.serialize_field("created_at", &self.get_created_at())?;
        strukt.serialize_field("project_name", &self.get_project_name())?;
        strukt.serialize_field("target", &self.get_target())?;
        strukt.end()
    }
}

impl Serialize for JobGraphPackageReverseDependencies {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut strukt = serializer.serialize_struct("job_graph_package_reverse_dependencies", 3)?;
        strukt.serialize_field("origin", &self.get_origin())?;
        strukt.serialize_field("name", &self.get_name())?;
        strukt.serialize_field("rdeps", &self.get_rdeps())?;
        strukt.end()
    }
}
