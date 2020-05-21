#![doc(html_root_url = "https://docs.rs/mpls/0.1.0")]

//! A movie playlist file (MPLS) parser.
//!
//! The entry point into this crate is the [`Mpls`] struct. You can obtain an
//! instance of that struct through its [`parse`] method.
//!
//! For the basic tasks of extracting the playlist angles and segments, this
//! crate provides easy-to-use helper methods (see the example below). Beyond
//! that, however, this crate only provides a structured form of the playlist
//! data and does not re-interpret the movie playlist contents in any way.
//!
//! Documentation of the individual structs and properties is unfortunately
//! scarce. The MPLS file format seems to not be officially documented, and this
//! parser relies heavily on the excellent third-party file specs in the
//! [lw/BluRay] repository as well as the [bdinfo/mpls] Wikibooks page. Refer to
//! those for more in-depth information.
//!
//! See also [`Angle`] and [`Clip`].
//!
//! [`Mpls`]: types/struct.Mpls.html
//! [`parse`]: types/struct.Mpls.html#method.parse
//! [`Angle`]: types/struct.Angle.html
//! [`Clip`]: types/struct.Clip.html
//! [lw/BluRay]: https://github.com/lw/BluRay/wiki/MPLS
//! [bdinfo/mpls]: https://en.wikibooks.org/wiki/User:Bdinfo/mpls
//!
//! # Examples
//! ```no_run
//! # fn main() -> std::io::Result<()> {
//! use std::fs::File;
//! use mpls::Mpls;
//!
//! // open the playlist file
//! let mut file = File::open("00800.mpls")?;
//!
//! // parse the play list
//! let mpls = Mpls::from(&file).expect("failed to parse MPLS file.");
//!
//! // extract the play list's angles
//! let angles = mpls.angles();
//!
//! // extract the segments
//! for angle in angles {
//!     let segment_numbers: Vec<i32> = angle
//!         .segments()
//!         .iter()
//!         .map(|s| s.file_name.parse::<i32>().unwrap())
//!         .collect();
//!     println!("angle {}: {:?}", angle, segment_numbers);
//! }
//! # Ok(())
//! # }
//! ```
pub mod error;
mod parser;
pub mod types;

pub use error::MplsError;
pub use types::*;
