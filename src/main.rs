use crate::pack::pack;

mod pack;
mod core;

fn main() {
    pack("out.pak", "./in").unwrap();
}
