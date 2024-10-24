#![forbid(unsafe_code)]
#![deny(
    rust_2018_idioms,
    trivial_casts,
    unused_lifetimes,
    unused_qualifications
)]

#[cfg(test)]
#[macro_use]
extern crate more_asserts;

pub mod config;
pub mod error;
pub mod query;

pub const DEFAULT_CONFIG_PATH: &str = "chains.toml";
