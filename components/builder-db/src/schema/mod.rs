// TED: This can be removed when diesel fixes their macros
// https://github.com/rust-lang/rust/issues/50504
#![allow(proc_macro_derive_resolution_fallback)]

pub mod account;
pub mod audit;
pub mod channel;
pub mod integration;
pub mod invitation;
pub mod key;
pub mod license_keys;
pub mod member;
pub mod origin;
pub mod package;
pub mod project;
pub mod project_integration;
pub mod secrets;
pub mod settings;
pub mod sql_types;
