// Copyright (c) 2020-2020 Chef Software Inc. and/or applicable contributors
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

use std::{cmp::{Ordering,
                PartialOrd},
          convert::AsRef,
          fmt,
          result,
          str::FromStr};

use crate::hab_core::{error as herror,
                      package::{ident::{version_sort,
                                        Identifiable},
                                PackageIdent}};

use crate::db::models::package::BuilderPackageIdent;

use internment::Intern;

#[derive(Default, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct PackageIdentIntern {
    origin:  Intern<String>,
    name:    Intern<String>,
    version: Option<Intern<String>>,
    release: Option<Intern<String>>,
}

// This is a hack because I got tired of writing the PackageIdentIntern.from_str().expect()
// for stub code
#[macro_export]
macro_rules! ident_intern {
    ( $( $x:expr ),* ) => {
        {
            $(
                PackageIdentIntern::from_str($x).expect(format!("Unable to make ident from {}", $x).as_str())
            )*
        }
    }
}

#[macro_export]
macro_rules! ident_intern_vec {
    ( $( $x:expr ),* ) => {
        {
            $(
                let v: Vec<PackageIdentIntern> = $x.iter().map(|x| ident_intern!(x)).collect()
            )*
        }
    }
}

impl PackageIdentIntern {
    pub fn new(origin: &str, name: &str, version: Option<&str>, release: Option<&str>) -> Self {
        PackageIdentIntern { origin:  Intern::<String>::new(origin.to_string()),
                             name:    Intern::<String>::new(name.to_string()),
                             version: version.map(|x| Intern::<String>::new(x.into())),
                             release: release.map(|x| Intern::<String>::new(x.into())), }
    }

    pub fn from_ident(ident: &PackageIdent) -> PackageIdentIntern {
        PackageIdentIntern::new(ident.origin(),
                                ident.name(),
                                ident.version(),
                                ident.release())
    }

    pub fn short_ident(&self) -> PackageIdentIntern {
        PackageIdentIntern::new(&self.origin, &self.name, None, None)
    }

    pub fn versioned_ident(&self) -> PackageIdentIntern {
        // TODO Turn this into a result? (hit some problems bringing in our Result class)
        PackageIdentIntern::new(&self.origin, &self.name, Some(&self.version.unwrap()), None)
    }
}

impl Identifiable for PackageIdentIntern {
    fn origin(&self) -> &str { &self.origin }

    fn name(&self) -> &str { &self.name }

    fn version(&self) -> Option<&str> {
        // This is a bit hideous, need to find better way of taking Intern<String> to String to &str
        // self.version.as_ref().map(|x| **x) // std::option::Option<std::string::String>
        self.version.as_ref().map(|x| &***x) // works
    }

    fn release(&self) -> Option<&str> { self.release.as_ref().map(|x| &***x) }
}

impl fmt::Display for PackageIdentIntern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.version.is_some() && self.release.is_some() {
            write!(f,
                   "{}/{}/{}/{}",
                   self.origin,
                   self.name,
                   self.version.as_ref().unwrap(),
                   self.release.as_ref().unwrap())
        } else if self.version.is_some() {
            write!(f,
                   "{}/{}/{}",
                   self.origin,
                   self.name,
                   self.version.as_ref().unwrap())
        } else {
            write!(f, "{}/{}", self.origin, self.name)
        }
    }
}

impl AsRef<PackageIdentIntern> for PackageIdentIntern {
    fn as_ref(&self) -> &PackageIdentIntern { self }
}

impl FromStr for PackageIdentIntern {
    type Err = herror::Error;

    fn from_str(value: &str) -> result::Result<Self, Self::Err> {
        let ident = PackageIdent::from_str(value)?;
        Ok(PackageIdentIntern::from_ident(&ident))
    }
}

// TODO: Investigate this pattern to see if it applies to our From implementations below
// impl<T: Borrow<TypeA>> From<T> for TypeB {
//     fn from(a: T) -> Self {
//         Self
//     }
// }
// Possible pattern:
// impl<T: Borrow<BuilderPackageIdent>> From<T> for PackageIdentIntern {
//    fn from(ident: T) -> Self {
//        PackageIdentIntern::from_ident(ident.as_ref().0)
//    }
//}

// impl From<T> for PackageIdentIntern
// where
//     T: Identifiable,
// {
//     fn from(ident: T) -> Self {
//         PackageIdentIntern::new(
//             ident.origin(),
//             ident.name(),
//             ident.version(),
//             ident.release(),
//         )
//     }
// }

impl From<&PackageIdent> for PackageIdentIntern {
    fn from(ident: &PackageIdent) -> Self { PackageIdentIntern::from_ident(&ident) }
}

impl From<PackageIdent> for PackageIdentIntern {
    fn from(ident: PackageIdent) -> Self {
        PackageIdentIntern::new(ident.origin(),
                                ident.name(),
                                ident.version(),
                                ident.release())
    }
}

impl From<&BuilderPackageIdent> for PackageIdentIntern {
    fn from(ident: &BuilderPackageIdent) -> Self { PackageIdentIntern::from_ident(&ident.0) }
}
impl From<BuilderPackageIdent> for PackageIdentIntern {
    fn from(ident: BuilderPackageIdent) -> Self { PackageIdentIntern::from_ident(&ident.0) }
}

impl Into<PackageIdent> for PackageIdentIntern {
    fn into(self) -> PackageIdent {
        PackageIdent::new(self.origin(), self.name(), self.version(), self.release())
    }
}

// These are basically copypasta, too bad the base impl uses direct access to fields
impl PartialOrd for PackageIdentIntern {
    /// Packages can be compared according to the following:
    ///
    /// * origin is ignored in the comparison - my redis and your redis compare the same.
    /// * If the names are not equal, they cannot be compared.
    /// * If the versions are greater/lesser, return that as the ordering.
    /// * If the versions are equal, return the greater/lesser for the release.
    fn partial_cmp(&self, other: &PackageIdentIntern) -> Option<Ordering> {
        if self.name != other.name {
            return None;
        }
        if self.version.is_none() && other.version.is_none() {
            return None;
        }
        if self.version.is_none() && other.version.is_some() {
            return Some(Ordering::Less);
        }
        if self.version.is_some() && other.version.is_none() {
            return Some(Ordering::Greater);
        }
        if self.release.is_none() && other.release.is_none() {
            return None;
        }
        if self.release.is_none() && other.release.is_some() {
            return Some(Ordering::Less);
        }
        if self.release.is_some() && other.release.is_none() {
            return Some(Ordering::Greater);
        }
        match version_sort(self.version.as_ref().unwrap(),
                           other.version.as_ref().unwrap())
        {
            ord @ Ok(Ordering::Greater) | ord @ Ok(Ordering::Less) => ord.ok(),
            Ok(Ordering::Equal) => Some(self.release.cmp(&other.release)),
            Err(_) => {
                // TODO: Can we do better than this? As long as we allow
                // non-numeric versions to co-exist with numeric ones, we
                // always have potential for incorrect ordering no matter
                // what we choose - eg, "master" vs. "0.x.x" (real examples)
                debug!("Comparing non-numeric versions: {} {}",
                       self.version.as_ref().unwrap(),
                       other.version.as_ref().unwrap());
                match self.version
                          .as_ref()
                          .unwrap()
                          .cmp(other.version.as_ref().unwrap())
                {
                    ord @ Ordering::Greater | ord @ Ordering::Less => Some(ord),
                    Ordering::Equal => Some(self.release.cmp(&other.release)),
                }
            }
        }
    }
}

impl Ord for PackageIdentIntern {
    /// Packages can be compared according to the following:
    ///
    /// * origin is ignored in the comparison - my redis and your redis compare the same.
    /// * If the names are not equal, they cannot be compared.
    /// * If the versions are greater/lesser, return that as the ordering.
    /// * If the versions are equal, return the greater/lesser for the release.
    fn cmp(&self, other: &PackageIdentIntern) -> Ordering {
        if self.name != other.name {
            return self.name.cmp(&other.name);
        }
        match version_sort(self.version.as_ref().unwrap(),
                           other.version.as_ref().unwrap())
        {
            Ok(Ordering::Equal) => self.release.cmp(&other.release),
            Ok(ordering) => ordering,
            Err(_) => Ordering::Less,
        }
    }
}

pub fn display_ordering_cmp<T>(a: &T, b: &T) -> Ordering
    where T: Identifiable
{
    let cmp = a.origin().cmp(b.origin());
    if cmp != Ordering::Equal {
        return cmp;
    }

    let cmp = a.name().cmp(b.name());
    if cmp != Ordering::Equal {
        return cmp;
    }

    let cmp = match (a.version(), b.version()) {
        (None, None) => return Ordering::Equal,
        (None, Some(_)) => return Ordering::Less,
        (Some(_), None) => return Ordering::Greater,
        (Some(a_v), Some(b_v)) => {
            // We could panic here, but since this is intended for display formatting, we just make
            // a choice.
            //
            version_sort(a_v, b_v).unwrap_or(Ordering::Equal)
        }
    };
    if cmp != Ordering::Equal {
        return cmp;
    }

    match (a.release(), b.release()) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (Some(a_r), Some(b_r)) => a_r.cmp(b_r),
    }
}
