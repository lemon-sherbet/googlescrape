#![cfg(not(feature = "ffi"))]
extern crate googlescrape;

fn main() {
    println!("{:?}", googlescrape::google(&(if std::env::args().len() > 1 { std::env::args().skip(1).collect::<Vec<String>>().join(" ") } else { panic!("\x1B[31m\n googel search query?\n\x1B[0m") })).unwrap());
}
