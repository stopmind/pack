use crate::pack::pack;
use crate::unpack::unpack;

mod pack;
mod core;
mod unpack;

fn main() {
    pack("out.pak", "./in").unwrap();
    unpack("in.pak", "./out").unwrap();
}
