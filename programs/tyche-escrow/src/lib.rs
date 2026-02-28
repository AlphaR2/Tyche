
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

// Instruction builders use solana_sdk types — host-only, not compiled into the SBF binary.
#[cfg(not(target_os = "solana"))]
pub mod instruction_builder;

address::declare_id!("TYEhGGkbujScDqPK1KTKCRu9cjVzjBH2Yf9Jb5L5Xtk");

// Gate the entrypoint behind `no-entrypoint` so this crate can be linked as a
// library by tyche-cpi and the test suite without registering a second entry symbol.
#[cfg(not(feature = "no-entrypoint"))]
program_entrypoint!(crate::entrypoint::process_instruction);

// Panic-on-alloc guard — only valid inside the SBF VM.
#[cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]
no_allocator!();
