
#![cfg_attr(target_os = "solana", no_std)]

use pinocchio::address;
use pinocchio::program_entrypoint;
#[cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]
use pinocchio::no_allocator;

pub mod args;
pub mod discriminator;
pub mod entrypoint;
pub mod error;
pub mod instructions;
pub mod processor;
pub mod state;
pub mod utils;

address::declare_id!("TYGwvLsQWTNgwQcuP4sREXHVinz14WG9caEZecbKTVg");

// Gate the entrypoint behind `no-entrypoint` so this crate can be linked as a
// library by the test suite without registering a second entry symbol.
#[cfg(not(feature = "no-entrypoint"))]
program_entrypoint!(crate::entrypoint::process_instruction);

// Panic-on-alloc guard — only valid inside the SBF VM.
#[cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]
no_allocator!();





