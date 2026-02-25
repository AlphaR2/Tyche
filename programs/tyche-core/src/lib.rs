// SBF/on-chain: disable std to keep the binary lean.
// Host (tests, instruction builders, SDK): std is available.
#![cfg_attr(target_os = "solana", no_std)]

use pinocchio::address;
use pinocchio::program_entrypoint;
#[cfg(target_os = "solana")]
use pinocchio::no_allocator;

pub mod discriminator;
pub mod entrypoint;
pub mod error;
pub mod instruction_args;
// Instruction builders use solana_sdk types — host-only, not compiled into the SBF binary.
#[cfg(not(target_os = "solana"))]
pub mod instruction_builder;
pub mod processor;
pub mod state;

address::declare_id!("CNLrKxAUsW9jWYCxcbsGkL1aD1czry5kChDCiNde3wbe");

// Gate the entrypoint behind `no-entrypoint` so this crate can be linked as a
// library by tyche-cpi and the test suite without registering a second entry symbol.
#[cfg(not(feature = "no-entrypoint"))]
program_entrypoint!(crate::entrypoint::process_instruction);

// Panic-on-alloc guard — only valid inside the SBF VM.
#[cfg(target_os = "solana")]
no_allocator!();
