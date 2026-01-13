//! Trait definitions for FOSSA operations.
//!
//! Each entity type implements the traits it supports, encapsulating
//! API differences in the implementations.

mod get;
mod list;
mod update;

pub use get::Get;
pub use list::List;
pub use update::Update;
