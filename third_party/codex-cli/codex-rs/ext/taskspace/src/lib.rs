//! Canonical TaskSpace domain model and deterministic state transitions.

#![forbid(unsafe_code)]

pub mod events;
pub mod invariants;
pub mod model;
pub mod transitions;

#[cfg(test)]
mod event_tests;
#[cfg(test)]
mod fixture_tests;
