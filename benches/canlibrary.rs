#[macro_use]
extern crate lazy_static;
extern crate fastcan;

use criterion::{black_box, criterion_group, criterion_main, Criterion as Bencher};
use fastcan::parser::*;

lazy_static! {
    static ref SPNDEF: SignalDefinition = SignalDefinition::new(
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
    static ref MSG: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
}

fn bench_parse_array(b: &mut Bencher) {
    b.bench_function("bench_parse_array", move |b| {
        b.iter(|| black_box(SPNDEF.decode_message(&MSG as &[u8; 8]).unwrap()))
    });
}

fn bench_parse_message(b: &mut Bencher) {
    b.bench_function("bench_parse_message", move |b| {
        b.iter(|| black_box(SPNDEF.decode_message(MSG[..].to_vec()).unwrap()))
    });
}

criterion_group!(benches, bench_parse_array, bench_parse_message,);

criterion_main!(benches);
