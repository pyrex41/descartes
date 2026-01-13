//! E2E integration tests entry point
//!
//! Run with: cargo test --test e2e_tests

mod e2e;

// Re-export tests so they're discoverable
pub use e2e::*;
