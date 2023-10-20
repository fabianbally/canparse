use std::collections::HashMap;

use byteorder::{LittleEndian, BigEndian, ByteOrder};

use crate::dbc::library::{DbcSignal, DbcFrame};


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

impl<'a> DecodeMessage<&'a [u8; 8]> for DbcSignal {
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

    fn parser(&self) -> Box<dyn Fn(&[u8; 8]) -> Option<f32>> {
        let bit_len = self.get_definition().bit_len;
        let start_bit = self.get_definition().start_bit;
        let scale = self.get_definition().scale;
        let offset = self.get_definition().offset;
        let little_endian = self.get_definition().little_endian;

        let fun =
            move |msg: &[u8; 8]| parse_array(bit_len, start_bit, little_endian, scale, offset, msg);

        Box::new(fun)
    }
}

impl DecodeMessage<Vec<u8>> for DbcSignal {
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

    fn parser(&self) -> Box<dyn Fn(Vec<u8>) -> Option<f32>> {
        let bit_len = self.get_definition().bit_len;
        let start_bit = self.get_definition().start_bit;
        let scale = self.get_definition().scale;
        let offset = self.get_definition().offset;
        let little_endian = self.get_definition().little_endian;

        let fun = move |msg: Vec<u8>| {
            decode_message(bit_len, start_bit, little_endian, scale, offset, &msg)
        };

        Box::new(fun)
    }
}

impl EncodeMessage<Vec<u8>> for DbcFrame {
    fn encode_message(&self, signal_map: &HashMap<String, f64>) -> Result<Vec<u8>, String> {
        let signals = self.get_signals();

        let mut result: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

        for signal in signals.values() {
            if !signal_map.contains_key(&signal.get_definition().name) {
                return Err(format!("Missing signal data: {}", signal.get_definition().name));
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
    fn encode_message(&self, signal_map: &HashMap<String, f64>) -> Result<[u8; 8], String> {
        let signals = self.get_signals();

        let mut result: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];

        for signal in signals.values() {
            if !signal_map.contains_key(&signal.get_definition().name) {
                return Err(format!("Missing signal data: {}", signal.get_definition().name));
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