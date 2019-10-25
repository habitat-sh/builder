// TED: This can be removed when diesel fixes their macros
// https://github.com/rust-lang/rust/issues/50504
#![allow(proc_macro_derive_resolution_fallback)]

pub mod account;
pub mod channel;
pub mod integration;
pub mod invitations;
pub mod jobs;
pub mod keys;
pub mod origin;
pub mod package;
pub mod pagination;
pub mod project_integration;
pub mod projects;
pub mod secrets;
pub mod settings;

mod db_id_format {
    use serde::{self,
                Deserialize,
                Deserializer,
                Serializer};

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn serialize<S>(id: &i64, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let s = id.to_string();
        serializer.serialize_str(&s)
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<i64>().map_err(serde::de::Error::custom)
    }
}

mod db_optional_id_format {
    use serde::{self,
                Deserialize,
                Deserializer,
                Serializer};
    pub fn serialize<S>(id: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let s = match id {
            Some(s) => s.to_string(),
            None => String::from(""),
        };
        serializer.serialize_str(&s)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        match s.parse::<i64>().map_err(serde::de::Error::custom) {
            Ok(s) => Ok(Some(s)),
            Err(e) => Err(e),
        }
    }
}
