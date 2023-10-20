#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{
        dbc::DbcSignalDefinition,
        dbc::{DbcFrame, DbcLibrary, DbcSignal, DbcVersion, Entry},
        mapper::{DecodeMessage, EncodeMessage},
    };
    use approx::assert_relative_eq;

    lazy_static! {
        static ref DBC_EMPTY: DbcLibrary = DbcLibrary::default();
        static ref DBC_ONE: DbcLibrary = DbcLibrary::from_dbc_file("./tests/data/sample.dbc")
            .expect("Failed to create DbcLibrary from file");
        static ref DBC_FF: DbcLibrary = DbcLibrary::from_dbc_file("./tests/data/ff.dbc")
            .expect("Failed to create DbcLibrary from file");
        static ref SIGNAL_DEF: DbcSignalDefinition = DbcSignalDefinition {
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
        static ref SIGNAL_DEF_BE: DbcSignalDefinition = {
            let mut _spndef = SIGNAL_DEF.clone();
            _spndef.little_endian = false;
            _spndef
        };
        static ref SIGNAL_DEF_ALT: DbcSignalDefinition = {
            let mut sig_alt_def = SIGNAL_DEF.clone();
            sig_alt_def.offset = 10.0;
            sig_alt_def.little_endian = false;

            sig_alt_def.start_bit = 41;

            sig_alt_def
        };
        static ref FRAME_DEF: DbcFrame = {
            let mut signal_map: HashMap<String, DbcSignal> = HashMap::new();

            let dbc_signal = DbcSignal::new(Some(SIGNAL_DEF.clone()), None, HashMap::new(), None);

            let dbc_signal_alt =
                DbcSignal::new(Some(SIGNAL_DEF_ALT.clone()), None, HashMap::new(), None);

            signal_map.insert("Engine_Speed".to_string(), dbc_signal);
            signal_map.insert("Engine_Speed2".to_string(), dbc_signal_alt);

            DbcFrame::new(
                "test".to_string(),
                2364539904,
                6,
                "Vector_XXX".to_string(),
                HashMap::new(),
                None,
                signal_map,
            )
        };
        static ref MSG: Vec<u8> = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88].to_vec();
        static ref MSG_BE: Vec<u8> = [0x88, 0x77, 0x66, 0x55, 0x44, 0x33, 0x22, 0x11].to_vec();
    }

    #[test]
    fn default_dbclibrary() {
        assert_eq!(DBC_EMPTY.len(), 0);
    }

    #[test]
    fn get_signal_definition() {
        assert_eq!(
            *DBC_ONE
                .get_frame(2364539904)
                .expect("failed to get PgnDefinition from PgnLibrary")
                .get_signal("Engine_Speed")
                .expect("failed to get SpnDefinition from PgnDefinition")
                .get_definition(),
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
        let dbc_signal = DbcSignal::new(Some(SIGNAL_DEF.clone()), None, HashMap::new(), None);
        let dbc_signal_be = DbcSignal::new(Some(SIGNAL_DEF_BE.clone()), None, HashMap::new(), None);

        assert_relative_eq!(dbc_signal.decode_message(MSG.clone()).unwrap(), 2728.5f32);
        assert_relative_eq!(
            dbc_signal_be.decode_message(MSG_BE.clone()).unwrap(),
            2728.5
        );
    }

    #[test]
    fn test_long_names() {
        let name = DBC_FF
            .get_frame(1297)
            .expect("Did not find FSG_EXTRA_FRAME")
            .get_signal("FSG_DV_EBS_Brake_pressure_s_0000")
            .expect("Did not find signal")
            .get_attribute("SystemSignalLongSymbol")
            .expect("Did not find LongName Attribute");

        assert_eq!(name, "FSG_DV_EBS_Brake_pressure_sensor_rear");
    }

    #[test]
    fn test_parse_message() {
        let dbc_signal = DbcSignal::new(Some(SIGNAL_DEF.clone()), None, HashMap::new(), None);
        let dbc_signal_be = DbcSignal::new(Some(SIGNAL_DEF_BE.clone()), None, HashMap::new(), None);

        assert_relative_eq!(
            dbc_signal
                .decode_message(MSG.clone()[..7].to_vec())
                .unwrap(),
            2728.5
        );
        assert_relative_eq!(
            dbc_signal_be
                .decode_message(MSG_BE.clone()[..7].to_vec())
                .unwrap(),
            2728.5
        );
        assert!(dbc_signal
            .decode_message(MSG.clone()[..0].to_vec())
            .is_none());
        assert!(dbc_signal_be
            .decode_message(MSG_BE.clone()[..0].to_vec())
            .is_none());
    }

    #[test]
    fn test_encode_message() {
        let dbc_signal = DbcSignal::new(Some(SIGNAL_DEF.clone()), None, HashMap::new(), None);
        let dbc_signal_alt =
            DbcSignal::new(Some(SIGNAL_DEF_ALT.clone()), None, HashMap::new(), None);

        let mut signal_map: HashMap<String, f64> = HashMap::new();
        signal_map.insert("Engine_Speed".to_string(), 2728.5);
        signal_map.insert("Engine_Speed2".to_string(), 2728.5);

        let ret = FRAME_DEF.encode_message(&signal_map);

        assert!(ret.is_ok());

        let ret: Vec<u8> = ret.unwrap();

        assert!(ret[3] == 0x44 && ret[4] == 0x55);

        let sig = dbc_signal.decode_message(ret.clone());

        assert!(sig.is_some());

        assert_eq!(sig.unwrap(), 2728.5);

        let sig = dbc_signal_alt.decode_message(ret);

        assert!(sig.is_some());

        assert_eq!(sig.unwrap(), 2728.5);
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
