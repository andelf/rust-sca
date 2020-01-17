#![no_std]
#![feature(test)]
#[macro_use]
extern crate block_cipher_trait;
extern crate sms4;

bench!(sms4::Sm4, 16);
