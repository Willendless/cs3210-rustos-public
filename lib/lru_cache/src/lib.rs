#![no_std]

#![cfg_attr(feature = "no_std", no_std)]

#[cfg(not(feature = "no_std"))]
extern crate core;

extern crate alloc;

pub mod lrucache;

#[cfg(test)]
mod tests;
