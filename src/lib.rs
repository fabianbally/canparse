#![cfg_attr(
    feature = "cargo-clippy",
    allow(clippy::redundant_field_names, clippy::unreadable_literal)
)]
#![crate_name = "fastcan"]
#![warn(missing_docs)]

//!
//! # fastcan-rs
//!
//! The fastcan-rs library is a fork of the canparse library.
//! While the canparse library implements the J1393, the fastcan-rs
//! library follows a more generic approach and just implements basic CAN encoding and decoding.
//!
//! With the fastcan-rs library, you can load DBC files dynamically and encode as well as decode CAN messages.
//!
//! # Examples
//!
//! ```rust
//! use fastcan::{dbc::DbcSignalDefinition,
//!     dbc::{DbcFrame, DbcSignal,
//!     DbcLibrary, DbcVersion, Entry},
//!     mapper::{DecodeMessage, EncodeMessage},
//! };
//!
//! use std::collections::HashMap;
//!
//! let dbc = DbcLibrary::from_dbc_file("./tests/data/sample.dbc").unwrap();
//!
//! let mut signal_map: HashMap<String, f64> = HashMap::new();
//! signal_map.insert("Engine_Speed".to_string(), 2728.5);
//!
//! let frame = dbc.get_frame(2364539904).unwrap();
//!
//! let ret: Vec<u8> = frame.encode_message(&signal_map).unwrap();
//!
//! let signal = frame.get_signal("Engine_Speed").unwrap();
//!
//! let data = signal.decode_message(ret);
//! ```

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
pub mod mapper;

mod tests;
