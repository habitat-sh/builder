// Copyright:: Copyright (c) 2015-2016 Chef Software, Inc.
//
// The terms of the Evaluation Agreement (Bldr) between Chef Software Inc. and the party accessing
// this file ("Licensee") apply to Licensee's use of the Software until such time that the Software
// is made available under an open source license such as the Apache 2.0 License.

use std::fmt;
use std::result;
use std::str::FromStr;

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

use crate::hab_core;
use crate::hab_core::package::{self, Identifiable};

pub use crate::message::originsrv::*;

#[derive(Debug)]
pub enum Error {
    BadOriginPackageVisibility,
    BadOAuthProvider,
}

pub trait Pageable {
    fn get_range(&self) -> [u64; 2];

    fn limit(&self) -> i64 {
        (self.get_range()[1] - self.get_range()[0] + 1) as i64
    }
}

impl Serialize for OriginKeyIdent {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut strukt = serializer.serialize_struct("origin_key", 3)?;
        strukt.serialize_field("origin", self.get_origin())?;
        strukt.serialize_field("revision", self.get_revision())?;
        strukt.serialize_field("location", self.get_location())?;
        strukt.end()
    }
}

impl fmt::Display for OriginPackageIdent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.get_version().is_empty() && !self.get_release().is_empty() {
            write!(
                f,
                "{}/{}/{}/{}",
                self.get_origin(),
                self.get_name(),
                self.get_version(),
                self.get_release()
            )
        } else if !self.get_version().is_empty() {
            write!(
                f,
                "{}/{}/{}",
                self.get_origin(),
                self.get_name(),
                self.get_version()
            )
        } else {
            write!(f, "{}/{}", self.get_origin(), self.get_name())
        }
    }
}

impl From<hab_core::package::PackageIdent> for OriginPackageIdent {
    fn from(value: hab_core::package::PackageIdent) -> OriginPackageIdent {
        let mut ident = OriginPackageIdent::new();
        ident.set_origin(value.origin);
        ident.set_name(value.name);
        if let Some(ver) = value.version {
            if ver != "" {
                ident.set_version(ver);
            }
        }
        if let Some(rel) = value.release {
            if rel != "" {
                ident.set_release(rel);
            }
        }
        ident
    }
}

impl<'a> From<&'a OriginPackageIdent> for package::PackageIdent {
    fn from(value: &'a OriginPackageIdent) -> package::PackageIdent {
        let mut ident =
            package::PackageIdent::new(value.get_origin(), value.get_name(), None, None);
        if !value.get_version().is_empty() {
            ident.version = Some(value.get_version().into());
        }
        if !value.get_release().is_empty() {
            ident.release = Some(value.get_release().into());
        }
        ident
    }
}

impl FromStr for OriginPackageIdent {
    type Err = hab_core::Error;

    fn from_str(value: &str) -> result::Result<Self, Self::Err> {
        let mut parts = value.split("/");
        let mut ident = OriginPackageIdent::new();
        if let Some(part) = parts.next() {
            if part.len() > 0 {
                ident.set_origin(part.to_string());
            }
        }
        if let Some(part) = parts.next() {
            if part.len() > 0 {
                ident.set_name(part.to_string());
            }
        }
        if let Some(part) = parts.next() {
            if part.len() > 0 {
                ident.set_version(part.to_string());
            }
        }
        if let Some(part) = parts.next() {
            if part.len() > 0 {
                ident.set_release(part.to_string());
            }
        }
        Ok(ident)
    }
}

impl Identifiable for OriginPackageIdent {
    fn origin(&self) -> &str {
        self.get_origin()
    }

    fn name(&self) -> &str {
        self.get_name()
    }

    fn version(&self) -> Option<&str> {
        let ver = self.get_version();
        if ver.is_empty() {
            None
        } else {
            Some(ver)
        }
    }

    fn release(&self) -> Option<&str> {
        let rel = self.get_release();
        if rel.is_empty() {
            None
        } else {
            Some(rel)
        }
    }
}

impl Into<package::PackageIdent> for OriginPackageIdent {
    fn into(self) -> package::PackageIdent {
        let mut ident = package::PackageIdent::new(self.get_origin(), self.get_name(), None, None);
        if !self.get_version().is_empty() {
            ident.version = Some(self.get_version().into());
        }
        if !self.get_release().is_empty() {
            ident.release = Some(self.get_release().into());
        }
        ident
    }
}

// Sessions

impl FromStr for OAuthProvider {
    type Err = Error;

    fn from_str(value: &str) -> result::Result<Self, Self::Err> {
        match value.to_lowercase().as_ref() {
            "active-directory" => Ok(OAuthProvider::ActiveDirectory),
            "azure-ad" => Ok(OAuthProvider::AzureAD),
            "github" => Ok(OAuthProvider::GitHub),
            "gitlab" => Ok(OAuthProvider::GitLab),
            "bitbucket" => Ok(OAuthProvider::Bitbucket),
            "okta" => Ok(OAuthProvider::Okta),
            "chef-automate" => Ok(OAuthProvider::ChefAutomate),
            "none" => Ok(OAuthProvider::None),
            "" => Ok(OAuthProvider::None),
            _ => Err(Error::BadOAuthProvider),
        }
    }
}

impl Into<Session> for AccessToken {
    fn into(self) -> Session {
        let mut session = Session::new();
        session.set_id(self.get_account_id());
        session.set_flags(self.get_flags());
        session
    }
}

impl Serialize for Session {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut strukt = serializer.serialize_struct("session", 6)?;
        strukt.serialize_field("id", &self.get_id().to_string())?;
        strukt.serialize_field("name", self.get_name())?;
        strukt.serialize_field("email", self.get_email())?;
        strukt.serialize_field("token", self.get_token())?;
        strukt.serialize_field("flags", &self.get_flags())?;
        strukt.serialize_field("oauth_token", self.get_oauth_token())?;
        strukt.end()
    }
}
