
#![cfg_attr(
    feature = "cargo-clippy",
    allow(clippy::redundant_field_names, clippy::unreadable_literal)
)]
#![crate_name = "fastcan"]

extern crate encoding;
#[macro_use]
extern crate enum_primitive;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;
extern crate byteorder;

#[cfg(feature = "use-socketcan")]
extern crate socketcan;

pub mod dbc;
pub mod parser;
pub mod mapper;
pub mod tests;