//! Canonical TaskSpace domain model and deterministic state transitions.

#![forbid(unsafe_code)]

pub mod invariants;
pub mod model;

#[cfg(test)]
mod fixture_tests;
