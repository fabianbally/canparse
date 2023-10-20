//! Functions for encoding and decoding CAN frames

use std::collections::HashMap;

use byteorder::{BigEndian, ByteOrder, LittleEndian};

use crate::dbc::{DbcFrame, DbcSignal};

/// The collection of functions for parsing CAN messages `N` into their defined signal values.
pub trait DecodeMessage<N> {
    /// Parses CAN message type `N` into generic `f32` signal value on success, or `None`
    /// on failure.
    fn decode_message(&self, msg: N) -> Option<f32>;
}

/// Interface for encoding a hashmap into a can frame
pub trait EncodeMessage<N> {
    /// Encode a can frame from signals in a hashmap
    fn encode_message(&self, signal_map: &HashMap<String, f64>) -> Result<N, String>;
}

impl<'a> DecodeMessage<&'a [u8; 8]> for DbcSignal {
    ///
    /// Decodes a signal from a CAN frame
    ///
    /// # Arguments
    ///
    /// `msg`: CAN frame as byte slice
    ///
    /// Returns the signal as float
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fastcan::{dbc::{library::{DbcFrame, DbcSignal},
    ///     DbcLibrary},
    ///     mapper::DecodeMessage,
    /// };
    ///
    /// use std::collections::HashMap;
    ///
    /// let dbc = DbcLibrary::from_dbc_file("./tests/data/sample.dbc").unwrap();
    ///
    /// let frame = dbc.get_frame(2364539904).unwrap();
    ///
    /// let payload: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    ///
    /// let signal_def = frame.get_signal("Engine_Speed").unwrap();
    ///
    /// let data = signal_def.decode_message(&payload).unwrap();
    /// ```
    ///
    fn decode_message(&self, msg: &[u8; 8]) -> Option<f32> {
        parse_array(
            self.get_definition().bit_len,
            self.get_definition().start_bit,
            self.get_definition().little_endian,
            self.get_definition().scale,
            self.get_definition().offset,
            msg,
        )
    }
}

impl DecodeMessage<Vec<u8>> for DbcSignal {
    ///
    /// Decodes a signal from a CAN frame
    ///
    /// # Arguments
    ///
    /// `msg`: CAN frame as byte vector
    ///
    /// Returns the signal as float
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fastcan::{dbc::{library::{DbcFrame, DbcSignal},
    ///     DbcLibrary},
    ///     mapper::DecodeMessage,
    /// };
    ///
    /// use std::collections::HashMap;
    ///
    /// let dbc = DbcLibrary::from_dbc_file("./tests/data/sample.dbc").unwrap();
    ///
    /// let frame = dbc.get_frame(2364539904).unwrap();
    ///
    /// let payload: Vec<u8> = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88].to_vec();
    ///
    /// let signal_def = frame.get_signal("Engine_Speed").unwrap();
    ///
    /// let data = signal_def.decode_message(payload).unwrap();
    /// ```
    ///
    fn decode_message(&self, msg: Vec<u8>) -> Option<f32> {
        decode_message(
            self.get_definition().bit_len,
            self.get_definition().start_bit,
            self.get_definition().little_endian,
            self.get_definition().scale,
            self.get_definition().offset,
            &msg,
        )
    }
}

impl EncodeMessage<Vec<u8>> for DbcFrame {
    ///
    /// Encodes Hashmap of signal data into a DBC frame
    ///
    /// # Arguments
    ///
    /// `signal_map`: HashMap for signal data; signal name maps to signal data (normalized to float)
    ///
    /// Returns a byte vector of max 8 bytes (success) or an error string (failure)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fastcan::{dbc::{library::{DbcFrame, DbcSignal},
    ///     DbcLibrary},
    ///     mapper::EncodeMessage,
    /// };
    ///
    /// use std::collections::HashMap;
    ///
    /// let dbc = DbcLibrary::from_dbc_file("./tests/data/sample.dbc").unwrap();
    ///
    /// let mut signal_map: HashMap<String, f64> = HashMap::new();
    /// signal_map.insert("Engine_Speed".to_string(), 2728.5);
    ///
    /// let frame = dbc.get_frame(2364539904).unwrap();
    ///
    /// let ret: Vec<u8> = frame.encode_message(&signal_map).unwrap();
    /// ```
    ///
    fn encode_message(&self, signal_map: &HashMap<String, f64>) -> Result<Vec<u8>, String> {
        let signals = self.get_signals();

        let mut result: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

        for signal in signals {
            if !signal_map.contains_key(&signal.get_definition().name) {
                return Err(format!(
                    "Missing signal data: {}",
                    signal.get_definition().name
                ));
            }

            let byte_data = encode_signal(
                signal.get_definition().bit_len,
                signal.get_definition().start_bit,
                signal.get_definition().little_endian,
                signal.get_definition().scale,
                signal.get_definition().offset,
                *signal_map.get(&signal.get_definition().name).unwrap(),
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

impl EncodeMessage<[u8; 8]> for DbcFrame {
    ///
    /// Encodes Hashmap of signal data into a DBC frame
    ///
    /// # Arguments
    ///
    /// `signal_map`: HashMap for signal data; signal name maps to signal data (normalized to float)
    ///
    /// Returns a slice of 8 bytes (success) or an error string (failure)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use fastcan::{dbc::DbcSignalDefinition,
    ///     dbc::{library::{DbcFrame, DbcSignal},
    ///     DbcLibrary, DbcVersion, Entry},
    ///     mapper::{DecodeMessage, EncodeMessage},
    /// };
    ///
    /// use std::collections::HashMap;
    ///
    /// let dbc = DbcLibrary::from_dbc_file("./tests/data/sample.dbc").unwrap();
    ///
    /// let mut signal_map: HashMap<String, f64> = HashMap::new();
    /// signal_map.insert("Engine_Speed".to_string(), 2728.5);
    ///
    /// let frame = dbc.get_frame(2364539904).unwrap();
    ///
    /// let ret: [u8; 8] = frame.encode_message(&signal_map).unwrap();
    /// ```
    ///
    fn encode_message(&self, signal_map: &HashMap<String, f64>) -> Result<[u8; 8], String> {
        let signals = self.get_signals();

        let mut result: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

        for signal in signals {
            if !signal_map.contains_key(&signal.get_definition().name) {
                return Err(format!(
                    "Missing signal data: {}",
                    signal.get_definition().name
                ));
            }

            let byte_data = encode_signal(
                signal.get_definition().bit_len,
                signal.get_definition().start_bit,
                signal.get_definition().little_endian,
                signal.get_definition().scale,
                signal.get_definition().offset,
                *signal_map.get(&signal.get_definition().name).unwrap(),
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
