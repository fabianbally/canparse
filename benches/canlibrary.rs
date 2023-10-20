#[macro_use]
extern crate lazy_static;
extern crate fastcan;

use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, Criterion as Bencher};
use fastcan::dbc::{library::DbcSignal, DbcSignalDefinition};
use fastcan::mapper::DecodeMessage;

lazy_static! {
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
    static ref MSG: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
}

fn bench_parse_array(b: &mut Bencher) {
    let dbc_signal = DbcSignal::new(Some(SIGNAL_DEF.clone()), None, HashMap::new(), None);

    b.bench_function("bench_parse_array", move |b| {
        b.iter(|| black_box(dbc_signal.decode_message(&MSG as &[u8; 8]).unwrap()))
    });
}

fn bench_parse_message(b: &mut Bencher) {
    let dbc_signal = DbcSignal::new(Some(SIGNAL_DEF.clone()), None, HashMap::new(), None);

    b.bench_function("bench_parse_message", move |b| {
        b.iter(|| black_box(dbc_signal.decode_message(MSG[..].to_vec()).unwrap()))
    });
}

criterion_group!(benches, bench_parse_array, bench_parse_message,);

criterion_main!(benches);
