//! Regex-based DBC parser

use regex::Regex;

use super::{
    DbcFrameDefinition, DbcMessageAttribute, DbcMessageDescription, DbcSignalAttribute,
    DbcSignalDefinition, DbcSignalDescription, Entry, ParseEntryError,
};
type LazyRegex = once_cell::sync::Lazy<Regex>;

pub fn parse_dbc(line: &str) -> Result<Entry, ParseEntryError> {
    match parse_message_definition(line) {
        Some(entry) => return Ok(Entry::MessageDefinition(entry)),
        None => (),
    };

    match parse_message_description(line) {
        Some(entry) => return Ok(Entry::MessageDescription(entry)),
        None => (),
    }

    match parse_message_attribute(line) {
        Some(entry) => return Ok(Entry::MessageAttribute(entry)),
        None => (),
    }

    match parse_signal_definition(line) {
        Some(entry) => return Ok(Entry::SignalDefinition(entry)),
        None => (),
    }

    match parse_signal_description(line) {
        Some(entry) => return Ok(Entry::SignalDescription(entry)),
        None => (),
    }

    match parse_signal_attribute(line) {
        Some(entry) => return Ok(Entry::SignalAttribute(entry)),
        None => Err(ParseEntryError {
            kind: super::EntryErrorKind::RegexNoMatch,
        }),
    }
}

pub fn parse_message_definition(line: &str) -> Option<DbcFrameDefinition> {
    static RE: LazyRegex = LazyRegex::new(|| {
        Regex::new(r#"BO_ (?P<id>\d+) (?P<name>\S+) ?: (?P<len>\d+) (?P<sending_node>.*) ?"#)
            .unwrap()
    });

    RE.captures(line).and_then(|cap| {
        Some(DbcFrameDefinition {
            id: cap
                .name("id")
                .map(|id| id.as_str().to_string().parse::<u32>().unwrap())
                .unwrap(),
            name: cap
                .name("name")
                .map(|name| name.as_str().to_string())
                .unwrap(),
            message_len: cap
                .name("len")
                .map(|len| len.as_str().to_string().parse::<u32>().unwrap())
                .unwrap(),
            sending_node: cap
                .name("sending_node")
                .map(|sending_node| sending_node.as_str().to_string())
                .unwrap(),
        })
    })
}

pub fn parse_message_description(line: &str) -> Option<DbcMessageDescription> {
    static RE: LazyRegex =
        LazyRegex::new(|| Regex::new(r#"CM_ BO_ (?P<id>\d+) "(?P<description>.*)";"#).unwrap());

    RE.captures(line).and_then(|cap| {
        Some(DbcMessageDescription {
            id: cap
                .name("id")
                .map(|id| id.as_str().to_string().parse::<u32>().unwrap())
                .unwrap(),
            description: cap
                .name("description")
                .map(|description| description.as_str().to_string())
                .unwrap(),
        })
    })
}

pub fn parse_message_attribute(line: &str) -> Option<DbcMessageAttribute> {
    static RE: LazyRegex = LazyRegex::new(|| {
        Regex::new(r#"BA_ "(?P<name>\w+)" BO_(?P<id>\d+) (?P<value>\S*);"#).unwrap()
    });

    RE.captures(line).and_then(|cap| {
        Some(DbcMessageAttribute {
            name: cap
                .name("name")
                .map(|key| key.as_str().to_string())
                .unwrap(),
            id: cap
                .name("id")
                .map(|id| id.as_str().to_string().parse::<u32>().unwrap())
                .unwrap(),
            value: cap
                .name("value")
                .map(|value| value.as_str().to_string())
                .unwrap(),
        })
    })
}

pub fn parse_signal_definition(line: &str) -> Option<DbcSignalDefinition> {
    static RE: LazyRegex = LazyRegex::new(|| {
        Regex::new(
            r#" SG_ (?P<name>\S*)[ \t]((?P<multiplexed>m\d+)|(?P<multiplexor>M))? ?:[ ]?(?P<start_bit>\d+)\|(?P<bit_len>\d+)@(?P<little_endian>\d)(?P<is_signed>[+-]) \((?P<scale>-?\d+(\.\d+)?(e-?\d+)?),(?P<offset>-?\d+(\.\d+)?(e-?\d+)?)\) \[(?P<min_value>-?\d+(\.\d+)?(e-?\d+)?)\|(?P<max_value>-?\d+(\.\d+)?(e-?\d+)?)\] "(?P<units>.*)" (?P<receiving_node>.*)"#,
        )
        .unwrap()
    });

    RE.captures(line).and_then(|cap| {
        Some(DbcSignalDefinition {
            name: cap
                .name("name")
                .map(|name| name.as_str().to_string())
                .unwrap(),
            start_bit: cap
                .name("start_bit")
                .map(|start_bit| start_bit.as_str().to_string().parse::<usize>().unwrap())
                .unwrap(),
            bit_len: cap
                .name("bit_len")
                .map(|bit_len| bit_len.as_str().to_string().parse::<usize>().unwrap())
                .unwrap(),
            little_endian: cap
                .name("little_endian")
                .map(|little_endian| little_endian.as_str() == "1")
                .unwrap(),
            signed: cap
                .name("is_signed")
                .map(|is_signed| is_signed.as_str() == "-")
                .unwrap(),
            scale: cap
                .name("scale")
                .map(|scale| scale.as_str().to_string().parse::<f32>().unwrap())
                .unwrap(),
            offset: cap
                .name("offset")
                .map(|offset| offset.as_str().to_string().parse::<f32>().unwrap())
                .unwrap(),
            min_value: cap
                .name("min_value")
                .map(|min_value| min_value.as_str().to_string().parse::<f32>().unwrap())
                .unwrap(),
            max_value: cap
                .name("max_value")
                .map(|min_value| min_value.as_str().to_string().parse::<f32>().unwrap())
                .unwrap(),
            units: cap
                .name("units")
                .map(|units| units.as_str().to_string())
                .unwrap(),
            receiving_node: cap
                .name("receiving_node")
                .map(|receving_node| receving_node.as_str().to_string())
                .unwrap(),
        })
    })
}

pub fn parse_signal_description(line: &str) -> Option<DbcSignalDescription> {
    static RE: LazyRegex = LazyRegex::new(|| {
        Regex::new(r#"CM_ SG_ (?P<id>\d+) (?P<name>\w+)[ \t]"(?P<description>.*)";"#).unwrap()
    });

    RE.captures(line).and_then(|cap| {
        Some(DbcSignalDescription {
            id: cap
                .name("id")
                .map(|id| id.as_str().to_string().parse::<u32>().unwrap())
                .unwrap(),
            signal_name: cap
                .name("name")
                .map(|name| name.as_str().to_string())
                .unwrap(),
            description: cap
                .name("description")
                .map(|description| description.as_str().to_string())
                .unwrap(),
        })
    })
}

pub fn parse_signal_attribute(line: &str) -> Option<DbcSignalAttribute> {
    static RE: LazyRegex = LazyRegex::new(|| {
        Regex::new(r#"BA_ "(?P<key>\w+)" SG_ (?P<id>\d+) (?P<name>\w+)[ \t]"?(?P<value>\w+)"?;"#)
            .unwrap()
    });

    RE.captures(line).and_then(|cap| {
        Some(DbcSignalAttribute {
            name: cap.name("key").map(|key| key.as_str().to_string()).unwrap(),
            id: cap
                .name("id")
                .map(|id| id.as_str().to_string().parse::<u32>().unwrap())
                .unwrap(),
            signal_name: cap
                .name("name")
                .map(|name| name.as_str().to_string())
                .unwrap(),
            value: cap
                .name("value")
                .map(|value| value.as_str().to_string())
                .unwrap(),
        })
    })
}

#[cfg(test)]
mod tests {
    use crate::dbc::*;
    use super::*;

    #[test]
    fn test_signal_definition() {
        let sig = DbcSignalDefinition {
            name: "Engine_Speed".to_string(),
            start_bit: 24,
            bit_len: 16,
            little_endian: true,
            signed: false,
            scale: 0.125,
            offset: 0.0,
            min_value: 0.0,
            max_value: 8031.88,
            units: "rpm".to_string(),
            receiving_node: "Vector__XXX".to_string()
        };

        assert!(parse_signal_definition(
            r#" SG_ Engine_Speed : 24|16@1+ (0.125,0) [0|8031.88] "rpm" Vector__XXX"#
        )
        .unwrap() == sig);
    }
}
