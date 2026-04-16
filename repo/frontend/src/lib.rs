// Library crate entry point for unit testing.
//
// Exposes only the modules that compile on native targets (no browser APIs,
// no gloo-net, no WASM-specific code). This allows `cargo test --lib` to
// run pure-logic tests without a WASM toolchain.
//
// Run all lib tests:  cargo test --lib
// Run specific test:  cargo test --lib -- state::state_test
//
// Modules that depend on gloo-net (services/api_client.rs) or Dioxus rendering
// (pages/, components/) must be tested via `wasm-pack test` on the WASM target.

pub mod state;
pub mod models;
pub mod features;

// Pure URL/HTTP utilities extracted for testability without gloo-net.
pub mod url_utils;

// Test modules (lib-crate scope — runs with `cargo test --lib`)
#[cfg(test)]
mod url_utils_test;
