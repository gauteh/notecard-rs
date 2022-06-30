[![Crates.io](https://img.shields.io/crates/v/blues-notecard.svg)](https://crates.io/crates/blues-notecard)
[![Documentation](https://docs.rs/blues-notecard/badge.svg)](https://docs.rs/blues-notecard/)



# Rust driver for notecard

This is a rust driver for the [blues.io](https://blues.io) [notecard](https://blues.io/products/notecard/) based on
[embedded-hal](https://github.com/rust-embedded/embedded-hal).

<img src="notecard_measurements.png" width="50%"></img>

```rust
use notecard::Note;

let mut note = Note::new(i2c);
note.initialize().expect("could not initialize notecard.");

if note.ping() {
    info!("notecard found!");
} else {
    error!("notecard not found!");
}


info!("note: card.time");
info!("note: time: {:?}", note.card().time(&mut delay).unwrap().wait(&mut delay));

info!("querying status..");
info!("status: {:?}", note.card().status(&mut delay).unwrap().wait(&mut delay));
```
