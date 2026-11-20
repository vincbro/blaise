//! A transit router over a compiled GTFS file: `RAPTOR` with walking, transfers and optional
//! GTFS-realtime delays, cancellations and skipped stops.

mod error;
pub mod raptor;
pub mod spatial;
#[cfg(test)]
mod test_support;

pub use error::{Error, Result};
