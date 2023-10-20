extern crate fastcan;

use fastcan::dbc::DbcLibrary;
use fastcan::dbc::DbcSignal;
use fastcan::mapper::DecodeMessage;

#[test]
fn canlib_build_parse() {
    let lib = DbcLibrary::from_dbc_file("./tests/data/sample.dbc").unwrap();

    // Pull signal definition for engine speed
    let enginespeed_def: &DbcSignal = lib.get_signal("Engine_Speed").unwrap();

    // Parse frame containing engine speed
    let msg: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    let engine_speed: f32 = enginespeed_def.decode_message(&msg).unwrap();

    assert!(engine_speed - 2728.5 < f32::EPSILON);
}

#[test]
fn canlib_from_dbc_file() {
    let lib = DbcLibrary::from_dbc_file("./tests/data/sample.dbc");
    assert!(lib.is_ok());

    let lib_fail = DbcLibrary::from_dbc_file("./tests/data/sample.dbc.fail");
    let e = lib_fail.map_err(|e| e.kind());

    assert!(e.is_err());
}
