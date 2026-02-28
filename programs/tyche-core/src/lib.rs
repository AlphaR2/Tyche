#![cfg_attr(target_os = "solana", no_std)]

use pinocchio::address;
use pinocchio::program_entrypoint;
#[cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]
use pinocchio::no_allocator;

pub mod discriminator;
pub mod entrypoint;
pub mod error;
pub mod instruction_args;
pub mod instructions;
// Instruction builders use solana_sdk types — host-only, not compiled into the SBF binary.
#[cfg(not(target_os = "solana"))]
pub mod instruction_builder;
pub mod processor;
pub mod state;

address::declare_id!("TYCANGQk6tumtij3tHwsRPSNkSHU3KGSNxNG59qJrHx");

// Gate the entrypoint behind `no-entrypoint` so this crate can be linked as a
// library by tyche-cpi and the test suite without registering a second entry symbol.
#[cfg(not(feature = "no-entrypoint"))]
program_entrypoint!(crate::entrypoint::process_instruction);

// Panic-on-alloc guard — only valid inside the SBF VM.
// Not registered when compiled as a library (no-entrypoint), because the
// dependent crate (e.g. tyche-escrow) will register its own allocator.
#[cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]
no_allocator!();
