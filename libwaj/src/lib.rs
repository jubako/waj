mod common;
pub mod create;
mod entry;
//pub mod fs_adder;
pub mod error;
mod serve;
mod waj;
pub mod walk;

pub use common::{AllProperties, Builder, Entry, FullBuilderTrait, VENDOR_ID};
pub use entry::*;
pub use serve::{HostRouter, Router, Server, SubPathRouter, WajServer};
pub use waj::Waj;
//pub use walk::*;

#[cfg(test)]
#[rustest::main]
fn main() {}
