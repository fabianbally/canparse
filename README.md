# canparse

A CAN signal encoding and decoding library with dynamic DBC loading

## Usage

Add canparse to your `Cargo.toml` with:

## Example

For a predefined DBC file, a simple program which utilizes `DbcLibrary` can be
implemented as folows:

```rust
extern crate canparse;

use canparse::pgn::{PgnLibrary, SpnDefinition, ParseMessage};

fn main() {

    // Parse dbc file into PgnLibrary
    let lib = DbcLibrary::from_dbc_file("./j1939.dbc").unwrap();

    // Pull signal definition for engine speed
    let enginespeed_def: &SignalDefinition = lib
        .get_spn("Engine_Speed").unwrap();

    // Parse frame containing engine speed
    let msg: [u8; 8] = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
    let engine_speed: f32 = enginespeed_def.parse_message(&msg).unwrap();

    println!("Engine speed: {}", engine_speed);
}
```

## Feature flags

- `use-socketcan` - Support for [socketcan-rs](https://crates.io/crates/socketcan)
has been included for Linux systems requiring a translation from `CANFrame` messages,
using `SpnDefinition` values.

## Alternatives

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
