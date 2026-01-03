//! Instruction handlers for StreamSync STRM Token Program

pub mod initialize;
pub mod stake;
pub mod unstake;
pub mod claim;
pub mod settle;

pub use initialize::*;
pub use stake::*;
pub use unstake::*;
pub use claim::*;
pub use settle::*;
