//! Proof-of-work implementation.
//!
//! Juno Cash uses RandomX proof-of-work instead of Equihash.

pub mod difficulty;
pub mod equihash;
pub mod randomx;
mod u256;

#[cfg(any(test, feature = "proptest-impl"))]
mod arbitrary;
#[cfg(test)]
mod tests;
