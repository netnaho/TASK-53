/// Feature modules: domain-specific logic grouped by bounded context.
///
/// All submodules here contain pure-logic functions (no browser APIs, no gloo-net)
/// and are compiled and tested via `cargo test --lib` on native targets.
///
/// Modules that depend on Dioxus rendering or gloo-net are in `pages/` and
/// `services/` respectively; they are not part of this lib crate.

pub mod reporting;
pub mod scoring;
pub mod billing;
pub mod ops;

#[cfg(test)]
mod features_test;
