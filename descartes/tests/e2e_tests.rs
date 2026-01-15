//! E2E integration tests entry point
//!
//! Run with: cargo test --test e2e_tests

mod e2e;
mod user_stories;

// Re-export tests so they're discoverable
#[allow(unused_imports)]
pub use e2e::*;
