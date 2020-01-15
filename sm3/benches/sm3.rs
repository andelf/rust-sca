#![no_std]
#![feature(test)]
#[macro_use]
extern crate digest;
extern crate sm3;

bench!(sm3::Sm3);
