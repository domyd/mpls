# mpls

A movie playlist file (MPLS) parser. Written in Rust using the [nom](https://github.com/Geal/nom) parser combinator library.

Dual-licensed under MIT and Apache 2.0.

## Example

```rust
use std::fs::File;
use std::io::Read;
use mpls::Mpls;

fn main() -> std::io::Result<()> {
    // open the playlist file
    let mut file = File::open("00800.mpls")?;

    // parse the play list
    let mpls = Mpls::from(&file).expect("failed to parse MPLS file.");

    // extract the play list's angles
    let angles = mpls.angles();

    // extract the segments
    for angle in angles {
        let segment_numbers: Vec<i32> = angle
            .segments()
            .iter()
            .map(|s| s.file_name.parse::<i32>().unwrap())
            .collect();
        println!("angle {}: {:?}", angle, segment_numbers);
    }

    Ok(())
}
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
mpls = "0.1.0"
```

## Documentation

See the [reference docs](https://docs.rs/mpls/0.1.0) on crates.io.
