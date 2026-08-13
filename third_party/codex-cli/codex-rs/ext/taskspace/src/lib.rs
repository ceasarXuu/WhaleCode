//! Canonical TaskSpace domain model and deterministic state transitions.

#![forbid(unsafe_code)]

pub mod events;
pub mod invariants;
pub mod model;
pub mod transactions;
pub mod transitions;

#[cfg(test)]
mod event_tests;
#[cfg(test)]
mod fixture_tests;
#[cfg(test)]
mod phase_d_tests;
#[cfg(test)]
mod property_tests;
#[cfg(test)]
mod replay_tests;
