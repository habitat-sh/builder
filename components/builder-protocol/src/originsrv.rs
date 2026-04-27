// Copyright:: Copyright (c) 2015-2016 Chef Software, Inc.
//
// The terms of the Evaluation Agreement (Bldr) between Chef Software Inc. and the party accessing
// this file ("Licensee") apply to Licensee's use of the Software until such time that the Software
// is made available under an open source license such as the Apache 2.0 License.

use std::{fmt,
          result,
          str::FromStr};

use serde::{ser::SerializeStruct,
            Serialize,
            Serializer};

use crate::hab_core::{self,
                      package::{self,
                                Identifiable}};

pub use crate::message::originsrv::*;

#[derive(Debug)]
pub enum Error {
    BadOriginPackageVisibility,
    BadOAuthProvider,
}

pub trait Pageable {
    fn get_range(&self) -> [u64; 2];

    fn limit(&self) -> i64 { (self.get_range()[1] - self.get_range()[0] + 1) as i64 }
}

impl Serialize for OriginKeyIdent {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut strukt = serializer.serialize_struct("origin_key", 3)?;
        strukt.serialize_field("origin", self.origin())?;
        strukt.serialize_field("revision", self.revision())?;
        strukt.serialize_field("location", self.location())?;
        strukt.end()
    }
}

impl fmt::Display for OriginPackageIdent {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if !self.version().is_empty() && !self.release().is_empty() {
            write!(f,
                   "{}/{}/{}/{}",
                   self.origin(),
                   self.name(),
                   self.version(),
                   self.release())
        } else if !self.version().is_empty() {
            write!(f,
                   "{}/{}/{}",
                   self.origin(),
                   self.name(),
                   self.version())
        } else {
            write!(f, "{}/{}", self.origin(), self.name())
        }
    }
}

impl From<hab_core::package::PackageIdent> for OriginPackageIdent {
    fn from(value: hab_core::package::PackageIdent) -> OriginPackageIdent {
        let mut ident = OriginPackageIdent::new();
        ident.set_origin(value.origin);
        ident.set_name(value.name);
        if let Some(ver) = value.version {
            if !ver.is_empty() {
                ident.set_version(ver);
            }
        }
        if let Some(rel) = value.release {
            if !rel.is_empty() {
                ident.set_release(rel);
            }
        }
        ident
    }
}

impl From<OriginPackageIdent> for package::PackageIdent {
    fn from(value: OriginPackageIdent) -> package::PackageIdent {
        let mut ident =
            package::PackageIdent::new(value.origin(), value.name(), None, None);
        if !value.version().is_empty() {
            ident.version = Some(value.version().into());
        }
        if !value.release().is_empty() {
            ident.release = Some(value.release().into());
        }
        ident
    }
}

impl FromStr for OriginPackageIdent {
    type Err = hab_core::Error;

    fn from_str(value: &str) -> result::Result<Self, Self::Err> {
        let mut parts = value.split('/');
        let mut ident = OriginPackageIdent::new();
        if let Some(part) = parts.next() {
            if !part.is_empty() {
                ident.set_origin(part.to_string());
            }
        }
        if let Some(part) = parts.next() {
            if !part.is_empty() {
                ident.set_name(part.to_string());
            }
        }
        if let Some(part) = parts.next() {
            if !part.is_empty() {
                ident.set_version(part.to_string());
            }
        }
        if let Some(part) = parts.next() {
            if !part.is_empty() {
                ident.set_release(part.to_string());
            }
        }
        Ok(ident)
    }
}

impl Identifiable for OriginPackageIdent {
    fn origin(&self) -> &str { self.origin() }

    fn name(&self) -> &str { self.name() }

    fn version(&self) -> Option<&str> {
        let ver = self.version();
        if ver.is_empty() {
            None
        } else {
            Some(ver)
        }
    }

    fn release(&self) -> Option<&str> {
        let rel = self.release();
        if rel.is_empty() {
            None
        } else {
            Some(rel)
        }
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

impl From<AccessToken> for Session {
    fn from(value: AccessToken) -> Session {
        let mut session = Session::new();
        session.set_id(value.account_id());
        session.set_flags(value.flags());
        session
    }
}

impl Serialize for Session {
    fn serialize<S>(&self, serializer: S) -> result::Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut strukt = serializer.serialize_struct("session", 6)?;
        strukt.serialize_field("id", &self.id().to_string())?;
        strukt.serialize_field("name", self.name())?;
        strukt.serialize_field("email", self.email())?;
        strukt.serialize_field("token", self.token())?;
        strukt.serialize_field("flags", &self.flags())?;
        strukt.serialize_field("oauth_token", self.oauth_token())?;
        strukt.end()
    }
}
