mod catalog;
mod envelope;
mod map_operations;
mod plan;

pub(crate) use catalog::TaskSpaceExecCatalog;
pub(crate) use catalog::TaskSpaceExecCatalogError;
pub(crate) use envelope::TaskSpaceExecEnvelope;
pub(crate) use envelope::TaskSpaceExecEnvelopeError;
pub(crate) use envelope::TaskSpaceExecInternalCallId;
pub(crate) use envelope::TaskSpaceExecRequestContext;
pub(crate) use map_operations::MapOperation;
pub(crate) use map_operations::MapOperationApplyError;
pub(crate) use map_operations::MapOperationEffect;
pub(crate) use map_operations::apply_map_operation;
pub(crate) use map_operations::map_operation_capabilities;
pub(crate) use plan::ClientCall;
pub(crate) use plan::ClientCallInput;
pub(crate) use plan::ExecCall;
pub(crate) use plan::HostedBinding;
pub(crate) use plan::TaskSpaceExecPlan;
pub(crate) use plan::TaskSpaceExecPlanDecodeError;

#[cfg(test)]
#[path = "../taskspace_exec_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../taskspace_exec_catalog_tests.rs"]
mod catalog_tests;

#[cfg(test)]
#[path = "../taskspace_exec_envelope_tests.rs"]
mod envelope_tests;
