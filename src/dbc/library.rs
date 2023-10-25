use crate::dbc;
use std::collections::HashMap;

/// Trait for converting `Entry` values into a library's own entries.
pub trait FromDbc {
    /// Error Type for parsing errors
    type Err;

    /// Converts an `Entity` value from scratch.
    fn from_entry(entry: dbc::Entry) -> Result<Self, Self::Err>
    where
        Self: Sized;

    /// Merges the given `Entity` with a `mut` version of the library's entity.  Useful for when
    /// multiple `Entry` types contribute to various attributes within the same destination.
    fn merge_entry(&mut self, entry: dbc::Entry) -> Result<(), Self::Err>;
}

type SignalAttribute = String;
#[derive(Clone, Debug, Default, PartialEq)]
/// Container datatype for holding informations concerning a signal of a CAN frame
pub struct DbcSignal {
    /// e.g., {"SPN", "190"}
    /// BA_ "SPN" SG_ 2364540158 EngSpeed 190;
    /// BA_ "SigType" SG_ 2364540158 EngSpeed 1;
    attributes: HashMap<String, SignalAttribute>,
    /// e.g., "Actual engine speed which is calculated over a minimum
    /// crankshaft angle of 720 degrees divided by the number of cylinders."
    description: Option<String>,
    /// e.g, SG_ EngSpeed : 24|16@1+ (0.125,0) [0|8031.875] "rpm" Vector__XXX
    definition: Option<dbc::DbcSignalDefinition>, // FIXME: hate that this has to be Option

    /// Only applicable for enum types
    /// e.g., VAL_ 2364540158 ActlEngPrcntTrqueHighResolution 8 "1111NotAvailable" 7 "0875" 1 "0125" 0 "0000" ;
    value_definition: Option<dbc::ValueDefinition>,
}

impl DbcSignal {
    ///
    /// Returns new DbcSignal
    ///
    /// # Example
    /// ```rust
    /// use fastcan::dbc::*;
    /// use std::collections::HashMap;
    ///
    /// let signal = DbcSignal::new(None, Some("A description".to_string()), HashMap::new(), None);
    /// ```
    pub fn new(
        definition: Option<DbcSignalDefinition>,
        description: Option<String>,
        attributes: HashMap<String, SignalAttribute>,
        value_definition: Option<ValueDefinition>,
    ) -> DbcSignal {
        Self {
            definition,
            description,
            attributes,
            value_definition,
        }
    }

    /// Returns the definition of the signal
    pub fn get_definition(&self) -> &DbcSignalDefinition {
        self.definition.as_ref().unwrap() // if this fails, there is a bug either in the error management of the library or in the lib itself
    }

    /// Queries the signal for an attribute with a given identifier
    pub fn get_attribute(&self, identifier: &str) -> Option<&String> {
        self.attributes.get(identifier)
    }

    /// Returns the long name of the signal
    pub fn long_name(&self) -> &String {
        match self.attributes.get("SystemSignalLongSymbol") {
            Some(name) => name,
            None => &(self.definition.as_ref().unwrap().name),
        }
    }
}

type MessageAttribute = String;

#[derive(Clone, Debug, Default)]
///
/// Container datatype for holding all informations about a CAN frame from a DBC file
pub struct DbcFrame {
    name: String,
    id: u32,
    message_len: u32,
    sending_node: String,

    /// e.g., BA_ "VFrameFormat" BO_ 2364540158 3;
    attributes: HashMap<String, MessageAttribute>,
    /// e.g., CM_ BO_ 2364540158 "Electronic Engine Controller 1";
    description: Option<String>,
    signals: HashMap<String, DbcSignal>,
}

impl DbcFrame {
    /// Returns new DBCFrame
    pub fn new(
        name: String,
        id: u32,
        message_len: u32,
        sending_node: String,
        attributes: HashMap<String, MessageAttribute>,
        description: Option<String>,
        signals: HashMap<String, DbcSignal>,
    ) -> Self {
        Self {
            name,
            id,
            message_len,
            sending_node,
            attributes,
            description,
            signals,
        }
    }

    /// Returns Vec of borrowed signal objects
    pub fn get_signals(&self) -> Vec<&DbcSignal> {
        self.signals.values().collect()
    }

    /// Query signal with signal name
    pub fn get_signal(&self, name: &str) -> Option<&DbcSignal> {
        self.signals.get(name)
    }

    /// Returns name of frame
    pub fn get_name(&self) -> &String {
        &self.name
    }

    /// Returns arbitration ID of CAN frame
    pub fn get_id(&self) -> u32 {
        self.id
    }

    /// Query frame attribute with an identifier
    pub fn get_attribute(&self, identifier: &str) -> &String {
        self.attributes.get(identifier).unwrap()
    }
}

impl FromDbc for DbcFrame {
    type Err = ();

    fn from_entry(entry: dbc::Entry) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        match entry {
            Entry::MessageDefinition(dbc::DbcFrameDefinition {
                id: _id,
                name,
                message_len,
                sending_node,
            }) => Ok(DbcFrame {
                name,
                message_len,
                sending_node,
                ..Default::default()
            }),
            Entry::MessageDescription(dbc::DbcMessageDescription {
                id: _id,
                description,
            }) => Ok(DbcFrame {
                description: Some(description),
                ..Default::default()
            }),
            Entry::MessageAttribute(dbc::DbcMessageAttribute {
                name,
                id: _id,
                value,
            }) => {
                let mut attributes = HashMap::new();
                attributes.insert(name, value);

                Ok(DbcFrame {
                    attributes,
                    ..Default::default()
                })
            }
            // TODO: Need to propogate Signal FromDbc in here..maybe, or just search in DbcLibrary
            _ => Err(()),
        }
    }

    fn merge_entry(&mut self, entry: dbc::Entry) -> Result<(), Self::Err> {
        match entry {
            Entry::MessageDefinition(dbc::DbcFrameDefinition {
                id: _id,
                name,
                message_len,
                sending_node,
            }) => {
                self.name = name;
                self.message_len = message_len;
                self.sending_node = sending_node;
                Ok(())
            }
            Entry::MessageDescription(dbc::DbcMessageDescription {
                id: _id,
                description,
            }) => {
                self.description = Some(description);
                Ok(())
            }
            Entry::MessageAttribute(dbc::DbcMessageAttribute {
                name,
                id: _id,
                value,
            }) => {
                if let Some(_previous_value) = self.attributes.insert(name, value) {
                    // TODO: Warn that we somehow already had an existing entry
                }
                Ok(())
            }
            Entry::SignalDefinition(inner) => {
                if self.signals.contains_key(&inner.name) {
                    (*self
                        .signals
                        .get_mut(&inner.name)
                        .expect("Already checked for Signal key"))
                    .merge_entry(Entry::SignalDefinition(inner))
                } else {
                    let name = inner.name.clone();
                    let signal = DbcSignal::from_entry(Entry::SignalDefinition(inner))?;
                    self.signals.insert(name, signal);
                    Ok(())
                }
            }
            Entry::SignalDescription(inner) => {
                if self.signals.contains_key(&inner.signal_name) {
                    (*self
                        .signals
                        .get_mut(&inner.signal_name)
                        .expect("Already checked for Signal key"))
                    .merge_entry(Entry::SignalDescription(inner))
                } else {
                    let name = inner.signal_name.clone();
                    let signal = DbcSignal::from_entry(Entry::SignalDescription(inner))?;
                    self.signals.insert(name, signal);
                    Ok(())
                }
            }
            Entry::SignalAttribute(inner) => {
                if self.signals.contains_key(&inner.signal_name) {
                    (*self
                        .signals
                        .get_mut(&inner.signal_name)
                        .expect("Already checked for Signal key"))
                    .merge_entry(Entry::SignalAttribute(inner))
                } else {
                    let name = inner.signal_name.clone();
                    let signal = DbcSignal::from_entry(Entry::SignalAttribute(inner))?;
                    self.signals.insert(name, signal);
                    Ok(())
                }
            }
            _ => Err(()),
        }
    }
}

impl FromDbc for DbcSignal {
    type Err = ();

    fn from_entry(entry: dbc::Entry) -> Result<Self, Self::Err>
    where
        Self: Sized,
    {
        match entry {
            Entry::SignalDefinition(definition) => Ok(DbcSignal {
                attributes: HashMap::new(),
                description: None,
                definition: Some(definition),
                value_definition: None,
            }),
            Entry::SignalDescription(dbc::DbcSignalDescription {
                id: _id,
                signal_name: _signal_name,
                description,
            }) => Ok(DbcSignal {
                attributes: HashMap::new(),
                description: Some(description),
                definition: None,
                value_definition: None,
            }),
            Entry::SignalAttribute(dbc::DbcSignalAttribute {
                name,
                id: _id,
                signal_name: _signal_name,
                value,
            }) => {
                let mut attributes = HashMap::new();
                attributes.insert(name, value);
                Ok(DbcSignal {
                    attributes,
                    description: None,
                    definition: None,
                    value_definition: None,
                })
            }
            _ => Err(()),
        }
    }

    fn merge_entry(&mut self, entry: dbc::Entry) -> Result<(), Self::Err> {
        match entry {
            Entry::SignalDefinition(definition) => {
                self.definition = Some(definition);
                Ok(())
            }
            Entry::SignalDescription(dbc::DbcSignalDescription {
                id: _id,
                signal_name: _signal_name,
                description,
            }) => {
                self.description = Some(description);
                Ok(())
            }
            Entry::SignalAttribute(dbc::DbcSignalAttribute {
                name,
                id: _id,
                signal_name: _signal_name,
                value,
            }) => {
                if let Some(_previous_value) = self.attributes.insert(name, value) {
                    // TODO: Warn that we somehow already had an existing entry
                }
                Ok(())
            }
            _ => Err(()),
        }
    }
}

/// A struct that represents a CANdb file, and provides APIs for interacting
/// with CAN messages and signals.
#[derive(Clone, Debug, Default)]
pub struct DbcLibrary {
    last_id: Option<u32>,
    frames: HashMap<u32, DbcFrame>,
}

impl DbcLibrary {
    /// Query frames with frame ID
    pub fn get_frame(&self, id: u32) -> Option<&DbcFrame> {
        self.frames.get(&id)
    }

    /// Returns how many frames are contained in the DBC
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Returns true if library is empty, false if it contains at least one CAN frame definition
    pub fn is_empty(&self) -> bool {
        self.frames.len() == 0
    }

    /// Returns a `SpnDefinition` entry reference, if it exists.
    pub fn get_signal(&self, name: &str) -> Option<&DbcSignal> {
        self.frames
            .iter()
            .find_map(|frame| frame.1.signals.get(name))
    }
}

use encoding::all::ISO_8859_1;
use encoding::{DecoderTrap, Encoding};
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::Path;

use super::{parser, DbcSignalDefinition, ValueDefinition};
use crate::dbc::Entry;

impl DbcLibrary {
    /// Creates a new `DbcLibrary` instance given an existing lookup table.
    pub fn new(messages: HashMap<u32, DbcFrame>) -> Self {
        DbcLibrary {
            last_id: None,
            frames: messages,
        }
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

        for line in data.lines() {
            if line.is_empty() {
                continue;
            }
            if let Some(entry) = parser::parse_dbc(line) {
                if let Err(_e) = lib.add_entry(entry) {
                    // TODO: Handle add_entry error
                }
            }
        }

        Ok(lib)
    }
}

impl DbcLibrary {
    /// Add DBC `Entry` to DBC library
    pub fn add_entry(&mut self, entry: Entry) -> Result<(), String> {
        let _id: u32 = *match entry {
            Entry::MessageDefinition(dbc::DbcFrameDefinition { ref id, .. }) => id,
            Entry::MessageDescription(dbc::DbcMessageDescription { ref id, .. }) => id,
            Entry::MessageAttribute(dbc::DbcMessageAttribute { ref id, .. }) => id,
            Entry::SignalDefinition(..) => {
                // no id, and by definition must follow MessageDefinition
                if let Some(last_id) = self.last_id.as_ref() {
                    last_id
                } else {
                    return Err("Tried to add SignalDefinition without last ID.".to_string());
                }
            }
            Entry::SignalDescription(dbc::DbcSignalDescription { ref id, .. }) => id,
            Entry::SignalAttribute(dbc::DbcSignalAttribute { ref id, .. }) => id,
            _ => {
                return Err(format!("Unsupported entry: {}.", entry));
            }
        };

        self.frames
            .entry(_id)
            .and_modify(|cur_entry| {
                cur_entry
                    .merge_entry(entry.clone())
                    .unwrap_or_else(|_| panic!("Already checked for Signal key: {:?}", entry))
            })
            .or_insert_with(|| {
                DbcFrame::from_entry(entry.clone())
                    .unwrap_or_else(|_| panic!("Some inserted a Signal for empty key: {:?}", _id))
            });

        self.last_id = Some(_id);

        Ok(())
    }
}
