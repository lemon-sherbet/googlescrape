#![cfg(not(feature = "ffi"))]
extern crate googlescrape;
use std::env;

fn main() {
    println!("{:?}", googlescrape::google(&env::args().last().expect("Missing query").trim()).unwrap());
}