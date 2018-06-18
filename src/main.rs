//! # swf-rs
//!
//! Library for reading and writing Adobe Flash SWF files.
//!
//! # Organization
//!
//! This library consits of a `read` module for decoding SWF data, and a `write` library for
//! writing SWF data.
#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]

extern crate byteorder;
#[macro_use]
extern crate enum_primitive;
extern crate encoding_rs;
extern crate flate2;
extern crate num;
#[macro_use]
extern crate log;

pub mod avm1;
pub mod avm2;
pub mod read;
mod tag_codes;
mod types;
pub mod write;

#[cfg(test)]
mod test_data;

/// Parses an SWF from a `Read` stream.
pub use read::read_swf;

/// Writes an SWF to a `Write` stream.
pub use write::write_swf;

/// Types used to represent a parsed SWF.
pub use types::*;

use std::fs::File;
use std::io::{self, BufReader};

fn parse(path: &str) -> Swf {
    println!("parsing {}", path);
    let f = File::open(path).unwrap();
    let reader = BufReader::new(f);
    read_swf(reader).unwrap()
}

fn main() {
    parse("../swf2js/swf/analog20.swf");
    parse("../swf2js/swf/lines.swf");
    parse("../swf2js/swf/model.swf");
    parse("../swf2js/swf/mogura.swf");
    parse("../swf2js/swf/tiger.swf");
    parse("../swf2js/swf/yomi.swf");
}
