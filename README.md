# mpls

A movie playlist file (MPLS) parser. Written in Rust using the [nom](https://github.com/Geal/nom) parser combinator library.

Dual-licensed under MIT and Apache 2.0.

## Example

```rust
use std::fs::File;
use std::io::Read;
use mpls::Mpls;

fn main() -> std::io::Result<()> {
    // read the MPLS file into memory
    let bytes = {
        let mut f = File::open("00800.mpls")?;
        let mut buffer = Vec::new();
        f.read_to_end(&mut buffer)?;
        buffer
    };

    // parse the play list
    let mpls = Mpls::parse(&bytes).expect("failed to parse MPLS file.");

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

## Documentation
