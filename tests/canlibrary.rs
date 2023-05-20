extern crate canparse;

use canparse::parser::{DecodeMessage, DbcLibrary, SignalDefinition};

#[test]
fn canlib_build_parse() {
    let lib = DbcLibrary::from_dbc_file("./tests/data/sample.dbc").unwrap();

    // Pull signal definition for engine speed
    let enginespeed_def: &SignalDefinition = lib.get_signal("Engine_Speed").unwrap();

    // Parse frame containing engine speed
    let msg: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    let engine_speed: f32 = enginespeed_def.parse_message(&msg).unwrap();

    assert!(engine_speed - 2728.5 < std::f32::EPSILON);
}

#[test]
fn canlib_from_dbc_file() {
    let lib = DbcLibrary::from_dbc_file("./tests/data/sample.dbc");
    assert!(lib.is_ok(), "PgnLibrary should have built successfully.");

    let lib_fail = DbcLibrary::from_dbc_file("./tests/data/sample.dbc.fail");
    assert_eq!(
        lib_fail.map_err(|e| e.kind()),
        Err(std::io::ErrorKind::NotFound)
    )
}
