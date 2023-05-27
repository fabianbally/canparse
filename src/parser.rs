//! Signal processing using PGN/SPN definitions.

#![allow(clippy::trivially_copy_pass_by_ref, clippy::too_many_arguments)]

use crate::dbc::{parser as nomparse, *};
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use encoding::all::ISO_8859_1;
use encoding::{DecoderTrap, Encoding};
use nom;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::marker::Sized;
use std::path::Path;
use std::str::FromStr;

#[cfg(feature = "use-socketcan")]
use socketcan::CANFrame;

/// Trait for converting `Entry` values into a library's own entries.
pub trait FromDbc {
    type Err;

    /// Converts an `Entity` value from scratch.
    fn from_entry(entry: Entry) -> Result<Self, Self::Err>
    where
        Self: Sized;

    /// Merges the given `Entity` with a `mut` version of the library's entity.  Useful for when
    /// multiple `Entry` types contribute to various attributes within the same destination.
    fn merge_entry(&mut self, entry: Entry) -> Result<(), Self::Err>;
}

/// A library used to translate CAN signals into desired values.
#[derive(Debug, PartialEq, Clone)]
pub struct DbcLibrary {
    last_id: u32,
    frames: HashMap<u32, FrameDefinition>,
}

impl DbcLibrary {
    /// Creates a new `DbcLibrary` instance given an existing lookup table.
    pub fn new(frames: HashMap<u32, FrameDefinition>) -> Self {
        DbcLibrary { last_id: 0, frames }
    }

    /// Convenience function for loading an entire DBC file into a returned `DbcLibrary`.  This
    /// function ignores unparseable lines as well as `Entry` variants which don't apply to
    /// `DbcLibrary` (such as `Entry::Version`).  Fails on `io::Error`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use fastcan::dbc::DbcLibrary;
    ///
    /// let lib: DbcLibrary = DbcLibrary::from_dbc_file("./tests/data/sample.dbc").unwrap();
    ///
    /// ```
    pub fn from_dbc_file<P>(path: P) -> io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Self::from_encoded_dbc_file(path, ISO_8859_1)
    }

    /// Convenience function for loading an entire DBC file into a returned `DbcLibrary`, using
    /// a specified `Encoding` codec. This function ignores unparseable lines as well as `Entry`
    /// variants which don't apply to `DbcLibrary` (such as `Entry::Version`).
    ///
    /// This function is currently considered unstable and subject to change or removal.
    ///
    /// # Example
    ///
    /// ```rust
    /// extern crate fastcan;
    /// extern crate encoding;
    ///
    /// use fastcan::dbc::DbcLibrary;
    /// use encoding::Encoding;
    /// use encoding::all::ISO_8859_1;
    ///
    /// let lib: DbcLibrary = DbcLibrary::from_encoded_dbc_file("./tests/data/sample.dbc",
    ///                                                         ISO_8859_1).unwrap();
    ///
    /// ```
    #[doc(hidden)]
    pub fn from_encoded_dbc_file<P, E>(path: P, encoding: &E) -> io::Result<Self>
    where
        P: AsRef<Path>,
        E: Encoding,
    {
        let mut lib = DbcLibrary::default();

        let data = File::open(path)
            .and_then(|mut f| {
                let mut contents: Vec<u8> = Vec::new();
                f.read_to_end(&mut contents).map(|_bytes_read| contents)
            })
            .and_then(|contents| {
                encoding
                    .decode(contents.as_slice(), DecoderTrap::Replace)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e))
            })?;

        let mut i = data.as_str();
        while !i.is_empty() {
            match nomparse::entry(i) {
                Ok((new_i, entry)) => {
                    if let Err(_e) = lib.add_entry(entry) {
                        // TODO: Handle add_entry error
                    }
                    i = new_i;
                }
                // FIXME: handling `IResult::Err`s could be better
                Err(nom::Err::Incomplete(_)) => {
                    break;
                }
                Err(_) => {
                    i = &i[1..];
                }
            }
        }

        Ok(lib)
    }

    /// Converts/combines DBC `Entry` values into entries within `DbcLibrary`.  Different `Entry`
    /// variants can modify the same internal entry, hence the need for mutability.  This function
    /// is meant to be called when parsing lines in a `dbc` file.
    ///
    /// # Example
    ///
    /// ```rust
    /// use std::collections::HashMap;
    /// use std::io::BufRead;
    /// use std::str::FromStr;
    /// use fastcan::dbc::Entry;
    /// use fastcan::dbc::DbcLibrary;
    ///
    /// let mut lib = DbcLibrary::new( HashMap::default() );
    ///
    /// // File is ISO-8859-1 and needs to be converted before iterating
    /// // with `lines`. `DbcLibrary::from_dbc_file` does this for you.
    /// let data: String = include_bytes!("../tests/data/sample.dbc")
    ///     .iter().map(|b| *b as char).collect();
    ///
    /// for line in data.lines() {
    ///     if let Some(entry) = Entry::from_str(line).ok() {
    ///         lib.add_entry(entry).ok();
    ///     }
    /// }
    /// ```
    pub fn add_entry(&mut self, entry: Entry) -> Result<(), String> {
        use std::collections::hash_map::Entry as HashMapEntry;

        let _id: u32 = *match entry {
            Entry::MessageDefinition(DbcMessageDefinition { ref id, .. }) => id,
            Entry::MessageDescription(DbcMessageDescription { ref id, .. }) => id,
            Entry::MessageAttribute(DbcMessageAttribute { ref id, .. }) => id,
            Entry::SignalDefinition(..) => {
                // no id, and by definition must follow MessageDefinition
                if self.last_id == 0 {
                    return Err("Tried to add SignalDefinition without last ID.".to_string());
                }
                &self.last_id
            }
            Entry::SignalDescription(DbcSignalDescription { ref id, .. }) => id,
            Entry::SignalAttribute(DbcSignalAttribute { ref id, .. }) => id,
            _ => {
                return Err(format!("Unsupported entry: {}.", entry));
            }
        };

        self.last_id = _id;
        match self.frames.entry(_id) {
            HashMapEntry::Occupied(mut existing) => {
                existing.get_mut().merge_entry(entry).unwrap();
            }
            HashMapEntry::Vacant(vacant) => {
                vacant.insert(FrameDefinition::from_entry(entry).unwrap());
            }
        }

        Ok(())
    }

    /// Returns a `DbcDefinition` entry reference, if it exists.
    pub fn get_frame(&self, frame_id: u32) -> Option<&FrameDefinition> {
        self.frames.get(&frame_id)
    }

    /// Returns a `SpnDefinition` entry reference, if it exists.
    pub fn get_signal(&self, name: &str) -> Option<&SignalDefinition> {
        self.frames
            .iter()
            .find_map(|frame| frame.1.signals.get(name))
    }

    pub fn get_frame_ids(&self) -> Vec<u32> {
        return self.frames.keys().cloned().collect();
    }
}

impl Default for DbcLibrary {
    fn default() -> Self {
        DbcLibrary::new(HashMap::default())
    }
}

/// Parameter Group Number definition
#[derive(Debug, PartialEq, Clone)]
pub struct FrameDefinition {
    id: u32,
    name_abbrev: String,
    description: String,
    length: u32,
    signals: HashMap<String, SignalDefinition>,
}

impl FrameDefinition {
    pub fn new(
        id: u32,
        name_abbrev: String,
        description: String,
        length: u32,
        signals: HashMap<String, SignalDefinition>,
    ) -> Self {
        FrameDefinition {
            id,
            name_abbrev,
            description,
            length,
            signals,
        }
    }

    pub fn get_signals(&self) -> Vec<SignalDefinition> {
        return self.signals.values().cloned().collect();
    }

    pub fn get_name(&self) -> &str {
        &self.name_abbrev
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }

    pub fn get_length(&self) -> u32 {
        self.length
    }
}
// TODO: DbcDefinition Builder pattern

/// Error returned on failure to parse `*Definition` type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDefinitionError {
    kind: DefinitionErrorKind,
}

impl ParseDefinitionError {
    #[doc(hidden)]
    pub fn __description(&self) -> &str {
        self.kind.__description()
    }

    #[doc(hidden)]
    pub fn __cause(&self) -> Option<&dyn Error> {
        self.kind.__cause()
    }
}

impl Display for ParseDefinitionError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.__description())
    }
}

impl Error for ParseDefinitionError {
    fn description(&self) -> &str {
        self.__description()
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.__cause()
    }
}

/// Internal type for `*Definition` parsing errors.
#[derive(Debug, Clone, PartialEq, Eq)]
enum DefinitionErrorKind {
    /// Internal `Entry` parsing error
    Entry(super::dbc::ParseEntryError),
    /// `Entry` type not applicable in constructing Definition
    UnusedEntry(super::dbc::EntryType),
}

impl DefinitionErrorKind {
    #[doc(hidden)]
    pub fn __description(&self) -> &str {
        match self {
            DefinitionErrorKind::Entry(_) => "internal Entry parsing error",
            DefinitionErrorKind::UnusedEntry(_) => {
                "Entry type not applicable in constructing Definition"
            }
        }
    }

    #[doc(hidden)]
    pub fn __cause(&self) -> Option<&dyn Error> {
        match self {
            DefinitionErrorKind::Entry(e) => Some(e),
            DefinitionErrorKind::UnusedEntry(_e) => None,
        }
    }
}

impl Display for DefinitionErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = self.__description();
        write!(f, "{}", s)
    }
}

impl From<DefinitionErrorKind> for ParseDefinitionError {
    fn from(kind: DefinitionErrorKind) -> Self {
        ParseDefinitionError { kind }
    }
}

impl FromStr for FrameDefinition {
    type Err = ParseDefinitionError;

    /// `&str` -> `DbcDefinition` via `dbc::Entry` (though probably won't be used).
    fn from_str(line: &str) -> Result<Self, Self::Err>
    where
        Self: Sized + FromDbc,
    {
        Entry::from_str(line)
            .map_err(|e| DefinitionErrorKind::Entry(e).into())
            .and_then(Self::from_entry)
    }
}

impl FromDbc for FrameDefinition {
    type Err = ParseDefinitionError;

    fn from_entry(entry: Entry) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        match entry {
            Entry::MessageDefinition(DbcMessageDefinition { id, name, .. }) => Ok(
                FrameDefinition::new(id, name, "".to_string(), 0, HashMap::new()),
            ),
            Entry::MessageDescription(DbcMessageDescription {
                id, description, ..
            }) => Ok(FrameDefinition::new(
                id,
                "".to_string(),
                description,
                0,
                HashMap::new(),
            )),
            Entry::MessageAttribute(DbcMessageAttribute { id, .. }) => Ok(FrameDefinition::new(
                id,
                "".to_string(),
                "".to_string(),
                0,
                HashMap::new(),
            )),
            _ => Err(DefinitionErrorKind::UnusedEntry(entry.get_type()).into()),
        }
    }

    fn merge_entry(&mut self, entry: Entry) -> Result<(), Self::Err> {
        match entry {
            Entry::MessageDefinition(DbcMessageDefinition {
                id, message_len, ..
            }) => {
                self.id = id;
                self.length = message_len;
                Ok(())
            }
            Entry::MessageDescription(DbcMessageDescription {
                id, description, ..
            }) => {
                self.id = id;
                self.description = description;
                Ok(())
            }
            Entry::MessageAttribute(DbcMessageAttribute { id, .. }) => {
                self.id = id;
                Ok(())
            }
            Entry::SignalDefinition(wrapped) => {
                if self.signals.contains_key(&wrapped.name) {
                    (*self.signals.get_mut(&wrapped.name).unwrap())
                        .merge_entry(Entry::SignalDefinition(wrapped))
                        .unwrap();
                } else {
                    self.signals.insert(
                        wrapped.name.clone(),
                        SignalDefinition::from_entry(Entry::SignalDefinition(wrapped)).unwrap(),
                    );
                }
                Ok(())
            }
            Entry::SignalDescription(wrapped) => {
                if self.signals.contains_key(&wrapped.signal_name) {
                    (*self.signals.get_mut(&wrapped.signal_name).unwrap())
                        .merge_entry(Entry::SignalDescription(wrapped))
                        .unwrap();
                } else {
                    self.signals.insert(
                        wrapped.signal_name.clone(),
                        SignalDefinition::from_entry(Entry::SignalDescription(wrapped)).unwrap(),
                    );
                }
                Ok(())
            }
            Entry::SignalAttribute(wrapped) => {
                if wrapped.name != "SPN" {
                    // Skip non-SPN attributes
                    return Ok(());
                }
                if self.signals.contains_key(&wrapped.signal_name) {
                    (*self.signals.get_mut(&wrapped.signal_name).unwrap())
                        .merge_entry(Entry::SignalAttribute(wrapped))
                        .unwrap();
                } else {
                    self.signals.insert(
                        wrapped.signal_name.clone(),
                        SignalDefinition::from_entry(Entry::SignalAttribute(wrapped)).unwrap(),
                    );
                }
                Ok(())
            }
            _ => Err(DefinitionErrorKind::UnusedEntry(entry.get_type()).into()),
        }
    }
}

/// Suspect Parameter Number definition
#[derive(Debug, PartialEq, Clone)]
pub struct SignalDefinition {
    name: String,
    number: usize,
    id: u32,
    description: String,
    start_bit: usize,
    bit_len: usize,
    little_endian: bool,
    signed: bool,
    scale: f32,
    offset: f32,
    min_value: f32,
    max_value: f32,
    units: String,
}

/// Internal function for parsing CAN message arrays given the definition parameters.  This is where
/// the real calculations happen.
fn parse_array(
    bit_len: usize,
    start_bit: usize,
    little_endian: bool,
    scale: f32,
    offset: f32,
    msg: &[u8; 8],
) -> Option<f32> {
    let msg64: u64 = if little_endian {
        LittleEndian::read_u64(msg)
    } else {
        BigEndian::read_u64(msg)
    };

    let bit_mask: u64 = 2u64.pow(bit_len as u32) - 1;

    Some((((msg64 >> start_bit) & bit_mask) as f32) * scale + offset)
}

/// Internal function for parsing CAN message slices given the definition parameters.  This is where
/// the real calculations happen.
fn decode_message(
    bit_len: usize,
    start_bit: usize,
    little_endian: bool,
    scale: f32,
    offset: f32,
    msg: &[u8],
) -> Option<f32> {
    let mut msg = msg.to_owned();

    if msg.is_empty() {
        return None;
    }

    if msg.len() < 8 {
        msg.resize(8, 0x00);
    }

    let msg64: u64 = if little_endian {
        LittleEndian::read_u64(&msg)
    } else {
        BigEndian::read_u64(&msg)
    };

    let bit_mask: u64 = 2u64.pow(bit_len as u32) - 1;

    Some((((msg64 >> start_bit) & bit_mask) as f32) * scale + offset)
}

fn encode_signal(
    bit_len: usize,
    start_bit: usize,
    little_endian: bool,
    scale: f32,
    offset: f32,
    signal: f64,
) -> Result<[u8; 8], String> {
    let data = (signal - (offset as f64)) / (scale as f64);

    if data.log2() > bit_len as f64 {
        return Err(format!("Signal does not fit into {}", data));
    }

    let byte_data = (data as u64) << start_bit;

    let result: [u8; 8] = match little_endian {
        true => byte_data.to_le_bytes(),
        false => byte_data.to_be_bytes(),
    };

    Ok(result)
}

/// The collection of functions for parsing CAN messages `N` into their defined signal values.
pub trait DecodeMessage<N> {
    /// Parses CAN message type `N` into generic `f32` signal value on success, or `None`
    /// on failure.
    fn decode_message(&self, msg: N) -> Option<f32>;

    /// Returns a closure which parses CAN message type `N` into generic `f32` signal value on
    /// success, or `None` on failure.
    fn parser(&self) -> Box<dyn Fn(N) -> Option<f32>>;
}

pub trait EncodeMessage<N> {
    fn encode_message(&self, signal_map: &HashMap<String, f64>) -> Result<N, String>;
}

impl SignalDefinition {
    /// Return new `SignalDefinition` given the definition parameters.
    pub fn new(
        name: String,
        number: usize,
        id: u32,
        description: String,
        start_bit: usize,
        bit_len: usize,
        little_endian: bool,
        signed: bool,
        scale: f32,
        offset: f32,
        min_value: f32,
        max_value: f32,
        units: String,
    ) -> Self {
        SignalDefinition {
            name,
            number,
            id,
            description,
            start_bit,
            bit_len,
            little_endian,
            signed,
            scale,
            offset,
            min_value,
            max_value,
            units,
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_id(&self) -> u32 {
        self.id
    }

    pub fn get_description(&self) -> &str {
        &self.description
    }
}

impl<'a> DecodeMessage<&'a [u8; 8]> for SignalDefinition {
    fn decode_message(&self, msg: &[u8; 8]) -> Option<f32> {
        parse_array(
            self.bit_len,
            self.start_bit,
            self.little_endian,
            self.scale,
            self.offset,
            msg,
        )
    }

    fn parser(&self) -> Box<dyn Fn(&[u8; 8]) -> Option<f32>> {
        let bit_len = self.bit_len;
        let start_bit = self.start_bit;
        let scale = self.scale;
        let offset = self.offset;
        let little_endian = self.little_endian;

        let fun =
            move |msg: &[u8; 8]| parse_array(bit_len, start_bit, little_endian, scale, offset, msg);

        Box::new(fun)
    }
}

impl DecodeMessage<Vec<u8>> for SignalDefinition {
    fn decode_message(&self, msg: Vec<u8>) -> Option<f32> {
        decode_message(
            self.bit_len,
            self.start_bit,
            self.little_endian,
            self.scale,
            self.offset,
            &msg,
        )
    }

    fn parser(&self) -> Box<dyn Fn(Vec<u8>) -> Option<f32>> {
        let bit_len = self.bit_len;
        let start_bit = self.start_bit;
        let scale = self.scale;
        let offset = self.offset;
        let little_endian = self.little_endian;

        let fun = move |msg: Vec<u8>| {
            decode_message(bit_len, start_bit, little_endian, scale, offset, &msg)
        };

        Box::new(fun)
    }
}

impl EncodeMessage<Vec<u8>> for FrameDefinition {
    fn encode_message(&self, signal_map: &HashMap<String, f64>) -> Result<Vec<u8>, String> {
        let signals = self.get_signals();

        let mut result: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

        for signal in signals {
            if !signal_map.contains_key(&signal.name) {
                return Err(format!("Missing signal data: {}", signal.name));
            }

            let byte_data = encode_signal(
                signal.bit_len,
                signal.start_bit,
                signal.little_endian,
                signal.scale,
                signal.offset,
                *signal_map.get(&signal.name).unwrap(),
            );

            let byte_data = match byte_data {
                Ok(b) => b,
                Err(err) => return Err(format!("Error encoding signal: {}", err)),
            };

            for i in 0..7 {
                result[i] |= byte_data[i];
            }
        }

        Ok(result.to_vec())
    }
}

impl EncodeMessage<[u8; 8]> for FrameDefinition {
    fn encode_message(&self, signal_map: &HashMap<String, f64>) -> Result<[u8; 8], String> {
        let signals = self.get_signals();

        let mut result: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

        for signal in signals {
            if !signal_map.contains_key(&signal.name) {
                return Err(format!("Missing signal data: {}", signal.name));
            }

            let byte_data = encode_signal(
                signal.bit_len,
                signal.start_bit,
                signal.little_endian,
                signal.scale,
                signal.offset,
                *signal_map.get(&signal.name).unwrap(),
            );

            let byte_data = match byte_data {
                Ok(b) => b,
                Err(err) => return Err(format!("Error encoding signal: {}", err)),
            };

            for i in 0..7 {
                result[i] |= byte_data[i];
            }
        }

        Ok(result)
    }
}

#[cfg(feature = "use-socketcan")]
impl<'a> DecodeMessage<&'a CANFrame> for SignalDefinition {
    fn decode_message(&self, frame: &CANFrame) -> Option<f32> {
        let msg = &frame.data().to_vec();
        decode_message(
            self.bit_len,
            self.start_bit,
            self.little_endian,
            self.scale,
            self.offset,
            msg,
        )
    }

    fn parser(&self) -> Box<dyn Fn(&CANFrame) -> Option<f32>> {
        let bit_len = self.bit_len;
        let start_bit = self.start_bit;
        let scale = self.scale;
        let offset = self.offset;
        let little_endian = self.little_endian;

        let fun = move |frame: &CANFrame| {
            let msg = &frame.data();
            decode_message(bit_len, start_bit, little_endian, scale, offset, msg)
        };

        Box::new(fun)
    }
}

impl FromStr for SignalDefinition {
    type Err = ParseDefinitionError;

    /// `&str` -> `SpnDefinition` via `dbc::Entry` (though probably won't be used).
    fn from_str(line: &str) -> Result<Self, Self::Err>
    where
        Self: Sized + FromDbc,
    {
        Entry::from_str(line)
            .map_err(|e| DefinitionErrorKind::Entry(e).into())
            .and_then(Self::from_entry)
    }
}

impl FromDbc for SignalDefinition {
    type Err = ParseDefinitionError;

    fn from_entry(entry: Entry) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        match entry {
            Entry::SignalDefinition(signal_definition) => Ok(signal_definition.into()),
            Entry::SignalDescription(signal_description) => Ok(signal_description.into()),
            Entry::SignalAttribute(signal_attribute) => Ok(signal_attribute.into()),
            _ => Err(DefinitionErrorKind::UnusedEntry(entry.get_type()).into()),
        }
    }

    fn merge_entry(&mut self, entry: Entry) -> Result<(), Self::Err> {
        match entry {
            Entry::SignalDefinition(DbcSignalDefinition {
                name,
                start_bit,
                bit_len,
                little_endian,
                signed,
                scale,
                offset,
                min_value,
                units,
                ..
            }) => {
                self.name = name;
                self.start_bit = start_bit;
                self.bit_len = bit_len;
                self.little_endian = little_endian;
                self.signed = signed;
                self.scale = scale;
                self.offset = offset;
                self.min_value = min_value;
                self.units = units;
                Ok(())
            }
            Entry::SignalDescription(DbcSignalDescription {
                id,
                signal_name,
                description,
            }) => {
                self.name = signal_name;
                self.id = id;
                self.description = description;
                Ok(())
            }
            Entry::SignalAttribute(DbcSignalAttribute {
                id,
                signal_name,
                value,
                ..
            }) => {
                self.name = signal_name;
                self.id = id;
                self.number = value.parse().unwrap();
                Ok(())
            }
            _ => Err(DefinitionErrorKind::UnusedEntry(entry.get_type()).into()),
        }
    }
}

impl From<DbcSignalDefinition> for SignalDefinition {
    fn from(
        DbcSignalDefinition {
            name,
            start_bit,
            bit_len,
            little_endian,
            signed,
            scale,
            offset,
            min_value,
            max_value,
            units,
            ..
        }: DbcSignalDefinition,
    ) -> Self {
        SignalDefinition::new(
            name,
            0,
            0, // TODO: Some()?
            "".to_string(),
            start_bit,
            bit_len,
            little_endian,
            signed,
            scale,
            offset,
            min_value,
            max_value,
            units,
        )
    }
}
impl From<DbcSignalDescription> for SignalDefinition {
    fn from(
        DbcSignalDescription {
            id,
            signal_name,
            description,
        }: DbcSignalDescription,
    ) -> Self {
        SignalDefinition::new(
            signal_name,
            0,
            id,
            description,
            0,
            0,
            true,
            false,
            0.0,
            0.0,
            0.0,
            0.0,
            "".to_string(),
        )
    }
}
impl From<DbcSignalAttribute> for SignalDefinition {
    fn from(
        DbcSignalAttribute {
            id,
            signal_name,
            value,
            ..
        }: DbcSignalAttribute,
    ) -> Self {
        SignalDefinition::new(
            signal_name,
            value.parse().unwrap(),
            id,
            "".to_string(),
            0,
            0,
            true,
            false,
            0.0,
            0.0,
            0.0,
            0.0,
            "".to_string(),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::*;
    use approx::assert_relative_eq;

    lazy_static! {
        static ref DBC_EMPTY: DbcLibrary = DbcLibrary::default();
        static ref DBC_ONE: DbcLibrary = DbcLibrary::from_dbc_file("./tests/data/sample.dbc")
            .expect("Failed to create PgnLibrary from file");
        static ref SIGNAL_DEF: SignalDefinition = SignalDefinition::new(
            "Engine_Speed".to_string(),
            190,
            2364539904,
            "A description for Engine speed.".to_string(),
            24,
            16,
            true,
            false,
            0.125,
            0.0,
            0.0,
            8031.88,
            "rpm".to_string()
        );
        static ref SIGNAL_DEF_BE: SignalDefinition = {
            let mut _spndef = SIGNAL_DEF.clone();
            _spndef.little_endian = false;
            _spndef
        };
        static ref SIGNAL_DEF_ALT: SignalDefinition = {
            let mut sig_alt_def = SIGNAL_DEF.clone();
            sig_alt_def.offset = 10.0;
            sig_alt_def.little_endian = false;

            sig_alt_def.start_bit = 41;

            sig_alt_def
        };
        static ref FRAME_DEF: FrameDefinition = {
            let mut signal_map: HashMap<String, SignalDefinition> = HashMap::new();

            signal_map.insert("Engine_Speed".to_string(), SIGNAL_DEF.clone());
            signal_map.insert("Engine_Speed2".to_string(), SIGNAL_DEF_ALT.clone());

            FrameDefinition {
                id: 2364539904,
                name_abbrev: "test".to_string(),
                description: "test".to_string(),
                length: 6,
                signals: signal_map,
            }
        };
        static ref MSG: Vec<u8> = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88].to_vec();
        static ref MSG_BE: Vec<u8> = [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11].to_vec();
    }

    #[test]
    fn default_pgnlibrary() {
        assert_eq!(DBC_EMPTY.frames.len(), 0);
    }

    #[test]
    fn get_signal_definition() {
        assert_eq!(
            *DBC_ONE
                .get_frame(2364539904)
                .expect("failed to get PgnDefinition from PgnLibrary")
                .signals
                .get("Engine_Speed")
                .expect("failed to get SpnDefinition from PgnDefinition"),
            *SIGNAL_DEF
        );
    }

    #[test]
    fn unsupported_entry() {
        let mut pgnlib: DbcLibrary = DbcLibrary::default();
        let unsupported = Entry::Version(DbcVersion("Don't care about version entry".to_string()));
        let res = pgnlib.add_entry(unsupported);

        assert!(res.is_err(), "Unsupported entry: Version");
    }

    #[test]
    fn test_parse_array() {
        assert_relative_eq!(SIGNAL_DEF.decode_message(MSG.clone()).unwrap(), 2728.5f32);
        assert_relative_eq!(
            SIGNAL_DEF_BE.decode_message(MSG_BE.clone()).unwrap(),
            2728.5
        );
    }

    #[test]
    fn test_parse_message() {
        assert_relative_eq!(
            SIGNAL_DEF
                .decode_message(MSG.clone()[..7].to_vec())
                .unwrap(),
            2728.5
        );
        assert_relative_eq!(
            SIGNAL_DEF_BE
                .decode_message(MSG_BE.clone()[..7].to_vec())
                .unwrap(),
            2728.5
        );
        assert!(SIGNAL_DEF
            .decode_message(MSG.clone()[..0].to_vec())
            .is_none());
        assert!(SIGNAL_DEF_BE
            .decode_message(MSG_BE.clone()[..0].to_vec())
            .is_none());
    }

    #[test]
    fn test_encode_message() {
        let mut signal_map: HashMap<String, f64> = HashMap::new();
        signal_map.insert("Engine_Speed".to_string(), 2728.5);
        signal_map.insert("Engine_Speed2".to_string(), 2728.5);

        let ret = FRAME_DEF.encode_message(&signal_map);

        assert!(ret.is_ok());

        let ret: Vec<u8> = ret.unwrap();

        assert!(ret[3] == 0x44 && ret[4] == 0x55);

        let sig = SIGNAL_DEF.decode_message(ret.clone());

        assert!(sig.is_some());

        assert_eq!(sig.unwrap(), 2728.5);

        let sig = SIGNAL_DEF_ALT.decode_message(ret);

        assert!(sig.is_some());

        assert_eq!(sig.unwrap(), 2728.5);
    }

    #[test]
    fn parse_message_closure() {
        assert_relative_eq!(
            SIGNAL_DEF.parser()(MSG.clone()[..].to_vec()).unwrap(),
            2728.5
        );
        assert_relative_eq!(
            SIGNAL_DEF_BE.parser()(MSG_BE.clone()[..].to_vec()).unwrap(),
            2728.5
        );
    }

    #[cfg(feature = "use-socketcan")]
    mod socketcan {
        extern crate socketcan;

        use super::*;

        use socketcan::CANFrame;

        lazy_static! {
            static ref FRAME: CANFrame = CANFrame::new(0, &MSG[..], false, false).unwrap();
            static ref FRAME_BE: CANFrame = CANFrame::new(0, &MSG_BE[..], false, false).unwrap();
        }

        #[test]
        fn parse_canframe_closure() {
            assert_relative_eq!(SIGNAL_DEF.parser()(&FRAME as &CANFrame).unwrap(), 2728.5);
            assert_relative_eq!(
                SIGNAL_DEF_BE.parser()(&FRAME_BE as &CANFrame).unwrap(),
                2728.5
            );
        }

        #[test]
        fn test_parse_canframe() {
            assert_relative_eq!(
                SIGNAL_DEF.decode_message(&FRAME as &CANFrame).unwrap(),
                2728.5
            );
            assert_relative_eq!(
                SIGNAL_DEF_BE
                    .decode_message(&FRAME_BE as &CANFrame)
                    .unwrap(),
                2728.5
            );
        }
    }
}
