// TED: This can be removed when diesel fixes their macros
// https://github.com/rust-lang/rust/issues/50504
#![allow(proc_macro_derive_resolution_fallback)]

pub mod account;
pub mod channel;
pub mod integration;
pub mod invitations;
pub mod origin;
pub mod package;
pub mod project_integration;
pub mod projects;

mod db_id_format {
    use serde::{self, Deserialize, Deserializer, Serializer};
    pub fn serialize<S>(id: &i64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", id.to_string());
        serializer.serialize_str(&s)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        s.parse::<i64>().map_err(serde::de::Error::custom)
    }
}
