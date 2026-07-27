#![cfg_attr(not(test), no_std)]

pub mod handler;
pub mod partitions;
pub mod types;

#[cfg(feature = "target-esp")]
pub mod config;
#[cfg(feature = "target-esp")]
pub mod ota;
#[cfg(feature = "target-esp")]
pub mod pins;
#[cfg(feature = "target-esp")]
pub mod pump;
#[cfg(feature = "target-esp")]
pub mod server;
