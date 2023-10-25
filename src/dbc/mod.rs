//! CANdb definition parsing

#![allow(non_upper_case_globals)]

use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

mod library;
mod parser;

pub use self::library::{DbcFrame, DbcLibrary, DbcSignal};

#[derive(Debug, Clone, Eq, PartialEq)]
#[doc(hidden)]
pub struct DbcVersion(pub String);

#[derive(Debug, Clone, PartialEq)]
#[doc(hidden)]
pub struct BusConfiguration(pub f32);

#[derive(Debug, Clone, Eq, PartialEq)]
/// Container for CAN frame definition from DBC
pub struct DbcFrameDefinition {
    /// Arbitration ID
    pub id: u32,
    /// CAN frame name
    pub name: String,
    /// Length of frame in bytes
    pub message_len: u32,
    /// Node that sends the frame
    pub sending_node: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[doc(hidden)]
pub struct DbcMessageDescription {
    pub id: u32,
    pub description: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[doc(hidden)]
pub struct DbcMessageAttribute {
    pub name: String,
    pub id: u32,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
/// Container for CAN signal definition from DBC
pub struct DbcSignalDefinition {
    /// Signal name
    pub name: String,
    /// Bit position in frame where the signal starts
    pub start_bit: usize,
    /// Length of the signal in bits
    pub bit_len: usize,
    /// Flag for if the signal is little endian
    pub little_endian: bool,
    /// Flag for if the signal is signed
    pub signed: bool,
    /// Factor that has to be applied to retrieve the physical value of the signal
    pub scale: f32,
    /// Offset that has to be applied to retrieve the physical value of the signal
    pub offset: f32,
    /// Minimum value of the signal
    pub min_value: f32,
    /// Maximum value of the signal
    pub max_value: f32,
    /// Unit of the physical value of the signal
    pub units: String,
    /// Nodes that receive the signal, seperated by commas
    pub receiving_node: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[doc(hidden)]
pub struct DbcSignalDescription {
    pub id: u32,
    pub signal_name: String,
    pub description: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
#[doc(hidden)]
pub struct DbcSignalAttribute {
    pub name: String,
    pub id: u32,
    pub signal_name: String,
    pub value: String,
}

/// Composed DBC entry.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    /// `VERSION`
    Version(DbcVersion),

    /// `BS_: <Speed>`
    BusConfiguration(BusConfiguration),

    // TODO: ??
    // CanNodes,
    // `CM_ BU_ [can id] [signal name] "[description]"`
    // CanNodesDescription,
    // CanNodesAttribute,
    /// `BO_ [can id] [message name]: [message length] [sending node]`
    MessageDefinition(DbcFrameDefinition),
    /// `CM_ BO_ [can id] [signal name] "[description]"`
    MessageDescription(DbcMessageDescription),
    /// `BA_ "[attribute name]" BO_ [node|can id] [signal name] [attribute value];`
    MessageAttribute(DbcMessageAttribute),

    /// `SG_ [signal name] [...] : [start bit]|[length]@[endian][sign] [[min]|[max]] "[unit]" [receiving nodes]`
    SignalDefinition(DbcSignalDefinition),
    /// `CM_ SG_ [can id] [signal name] "[description]"`
    SignalDescription(DbcSignalDescription),
    /// `BA_ "[attribute name]" SG_ [node|can id] [signal name] [attribute value];`
    SignalAttribute(DbcSignalAttribute),

    // `CM_ [BU_|BO_|SG_] [can id] [signal name] "[description]"`
    // Description, -- flatten subtypes instead

    // `BA_DEF_ ...`
    // AttributeDefinition,

    // `BA_DEF_DEF_ ...`
    // AttributeDefault,

    // `BA_ "[attribute name]" [BU_|BO_|SG_] [node|can id] [signal name] [attribute value];`
    // Attribute
    #[doc(hidden)]
    Unknown(String),
}

impl Entry {
    /// Returns an opaque `EntryType` for an `Entry` structure variant.
    // TODO: Finalize naming convention and expose
    pub(super) fn get_type(&self) -> EntryType {
        match self {
            Entry::Version(_) => EntryType::Version,
            Entry::BusConfiguration(_) => EntryType::BusConfiguration,
            Entry::MessageDefinition(_) => EntryType::MessageDefinition,
            Entry::MessageDescription(_) => EntryType::MessageDescription,
            Entry::MessageAttribute(_) => EntryType::MessageAttribute,
            Entry::SignalDefinition(_) => EntryType::SignalDefinition,
            Entry::SignalDescription(_) => EntryType::SignalDescription,
            Entry::SignalAttribute(_) => EntryType::SignalAttribute,
            Entry::Unknown(_) => EntryType::Unknown,
        }
    }
}

impl Display for Entry {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.get_type().fmt(f)
    }
}

enum_from_primitive! {
/// Internal type for DBC `Entry` line.
#[doc(hidden)]
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EntryType {
    Version = 0,

    BusConfiguration,

    // CanNodes,
    // CanNodesDescription,
    // CanNodesAttribute

    MessageDefinition,
    MessageDescription,
    MessageAttribute,
//    MessageAttributeDefinition,

    SignalDefinition,
    SignalDescription,
    SignalAttribute,
    SignalLongName,
//    SignalAttributeDefinition,

    // AttributeDefinition,
    // AttributeDefault,
    // Attribute

    Unknown,
}
}

impl Display for EntryType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let entry_str = match *self {
            EntryType::Version => "Version",
            EntryType::BusConfiguration => "BusConfiguration",
            EntryType::MessageDefinition => "MessageDefinition",
            EntryType::MessageDescription => "MessageDescription",
            EntryType::MessageAttribute => "MessageAttribute",
            EntryType::SignalDefinition => "SignalDefinition",
            EntryType::SignalDescription => "SignalDescription",
            EntryType::SignalAttribute => "SignalAttribute",

            EntryType::Unknown => "Unknown",
            EntryType::SignalLongName => "SignalLongName",
        };
        write!(f, "{}", entry_str)
    }
}

/// Error returned on failure to parse DBC `Entry`.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ParseEntryError {
    kind: EntryErrorKind,
}

impl ParseEntryError {
    #[doc(hidden)]
    pub fn __description(&self) -> &str {
        self.kind.__description()
    }

    #[doc(hidden)]
    pub fn __cause(&self) -> Option<&dyn Error> {
        self.kind.__cause()
    }
}

impl Display for ParseEntryError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", self.__description())
    }
}

impl Error for ParseEntryError {
    fn description(&self) -> &str {
        self.__description()
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.__cause()
    }
}

/// Internal type DBC `Entry` parsing error.
#[derive(Debug, Clone, Eq, PartialEq)]
enum EntryErrorKind {
    /// Could not find a regex match for input
    RegexNoMatch,
    /// Integer could not be converted into valid `EntryType`
    #[allow(dead_code)]
    UnknownEntryType(i32),
}

impl EntryErrorKind {
    #[doc(hidden)]
    pub fn __description(&self) -> &str {
        match *self {
            EntryErrorKind::RegexNoMatch => "could not find a regex match for input",
            EntryErrorKind::UnknownEntryType(_) => {
                "integer could not be converted into valid EntryType"
            }
        }
    }
    #[doc(hidden)]
    pub fn __cause(&self) -> Option<&dyn Error> {
        match *self {
            EntryErrorKind::RegexNoMatch => None,
            EntryErrorKind::UnknownEntryType(_) => None,
        }
    }
}

impl Display for EntryErrorKind {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let s = self.__description();
        write!(f, "{}", s)
    }
}

impl From<EntryErrorKind> for ParseEntryError {
    fn from(kind: EntryErrorKind) -> Self {
        ParseEntryError { kind }
    }
}

impl FromStr for Entry {
    type Err = ParseEntryError;

    fn from_str(line: &str) -> Result<Self, Self::Err> {
        parser::parse_dbc(line).map_or_else(
            || {
                Err(ParseEntryError {
                    kind: EntryErrorKind::RegexNoMatch,
                })
            },
            Ok,
        )
    }
}

/// Probably some spec to determine a type when generating structs
/// Here an enum will be dispatched instead (e.g., VAL_)
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ValueDefinition {
    values: Vec<String>,
}

#[doc(hidden)]
/// Types a attribute can be
pub enum AttributeType {
    /// Integer type with min/max values
    Int { min: i32, max: i32 },
    /// Float type with min/max values
    Float { min: f32, max: f32 },
    /// String type
    String,
    /// Enum type, represented as a vector of `String`s
    Enum(Vec<String>),
}
