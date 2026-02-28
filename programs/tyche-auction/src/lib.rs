#![cfg_attr(target_os = "solana", no_std)]

use pinocchio::address;
use pinocchio::program_entrypoint;
#[cfg(target_os = "solana")]
use pinocchio::no_allocator;

pub mod args;
pub mod discriminator;
pub mod entrypoint;
pub mod error;
pub mod instructions;

#[cfg(not(target_os = "solana"))]
pub mod instruction_builder;
pub mod processor;
pub mod state;

address::declare_id!("TYAKZZsLmYU65ScdqSGz6GxXs9KaUKF8sCFU52qmNTG");

#[cfg(not(feature = "no-entrypoint"))]
program_entrypoint!(crate::entrypoint::process_instruction);

#[cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]
no_allocator!();
