# Rust driver for notecard

This is a rust driver for the [blues.io](https://blues.io) [notecard](https://blues.io/products/notecard/) based on
[embedded-hal](https://github.com/rust-embedded/embedded-hal).

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
info!("note: time: {:?}", note.card().time().unwrap().wait());

info!("querying status..");
info!("status: {:?}", note.card().status().unwrap().wait());
```
