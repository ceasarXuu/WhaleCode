//! Canonical TaskSpace domain model and deterministic state transitions.

#![forbid(unsafe_code)]

mod event_emitter;
pub mod events;
mod execute_manifest;
mod extension;
mod initialize_manifest;
pub mod invariants;
mod lifecycle_manifest;
pub mod model;
mod preflight;
mod runtime;
mod service;
mod tool;
pub mod transactions;
pub mod transitions;
mod world_state;

pub use extension::install;
pub use extension::install_with_service;
pub use runtime::TaskSpaceMapBinding;
pub use runtime::TaskSpaceMapCommit;
pub use runtime::TaskSpaceMapRecord;
pub use runtime::TaskSpaceMapRelation;
pub use runtime::TaskSpaceMapWriteOutcome;
pub use runtime::TaskSpaceRuntimeHandle;
pub use runtime::TaskSpaceStore;
pub use runtime::TaskSpaceStoreFuture;
pub use service::TaskSpaceService;
pub use service::TaskSpaceServiceState;

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
#[cfg(test)]
mod runtime_extension_tests;
