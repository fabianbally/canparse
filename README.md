# fastcan

A CAN signal encoding and decoding library with dynamic DBC loading

## Usage

Add fastcan to your `Cargo.toml` with:

## Example

For a predefined DBC file, a simple program which utilizes `DbcLibrary` can be
implemented as folows:

```rust
use fastcan::{
    dbc::{
        DbcFrame, 
        DbcSignal, 
        DbcSignalDefinition, 
        DbcLibrary, 
        DbcVersion, 
        Entry
    },
    mapper::{
        DecodeMessage, 
        EncodeMessage
    },
};

use std::collections::HashMap;

let dbc = DbcLibrary::from_dbc_file("./tests/data/sample.dbc").unwrap();

let mut signal_map: HashMap<String, f64> = HashMap::new();

signal_map.insert("Engine_Speed".to_string(), 2728.5);

let frame = dbc.get_frame(2364539904).unwrap();

let ret: Vec<u8> = frame.encode_message(&signal_map).unwrap();

let signal = frame.get_signal("Engine_Speed").unwrap();

let data = signal.decode_message(ret);
```

## Alternatives
- [canparse](https://github.com/jmagnuson/canparse) (also Rust)
- [canmatrix](https://github.com/ebroecker/canmatrix) (Python)
- [libcanardbc](https://github.com/Polyconseil/libcanardbc) (C++)
- [CANBabel](https://github.com/julietkilo/CANBabel) (Java)
- [pyvit](https://github.com/linklayer/pyvit) (Python)
- [Kayak](https://github.com/dschanoeh/Kayak) (Java, OSS format `kcd`)

## License

Licensed under either of

- Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
