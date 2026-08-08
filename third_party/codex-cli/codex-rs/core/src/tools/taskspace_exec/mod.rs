mod catalog;
mod dispatch;
mod envelope;
mod handler;
mod map_operations;
mod plan;
mod preflight;
mod response_scope;
mod schema_validation;

pub(crate) use catalog::TASKSPACE_EXEC_TOOL_NAME;
pub(crate) use catalog::TaskSpaceExecCatalog;
pub(crate) use catalog::TaskSpaceExecCatalogError;
pub(crate) use dispatch::dispatch_client_calls;
pub(crate) use dispatch::prepare_client_calls;
pub(crate) use envelope::TaskSpaceExecEnvelope;
pub(crate) use envelope::TaskSpaceExecEnvelopeError;
pub(crate) use envelope::TaskSpaceExecInternalCallId;
pub(crate) use envelope::TaskSpaceExecRequestContext;
pub(crate) use handler::TaskSpaceExecHandler;
pub(crate) use map_operations::MapOperation;
pub(crate) use map_operations::MapOperationApplyError;
pub(crate) use map_operations::MapOperationEffect;
pub(crate) use map_operations::apply_map_operation;
pub(crate) use map_operations::map_operation_capabilities;
pub(crate) use plan::ClientCall;
pub(crate) use plan::ClientCallInput;
pub(crate) use plan::ExecCall;
pub(crate) use plan::TaskSpaceExecPlan;
pub(crate) use plan::TaskSpaceExecPlanDecodeError;
pub(crate) use preflight::HostedOutputFact;
pub(crate) use preflight::PreparedClientCall;
pub(crate) use preflight::PreparedHostedBinding;
#[cfg(test)]
pub(crate) use preflight::TaskSpaceExecPreflightError;
pub(crate) use preflight::preflight_taskspace_exec;
pub(crate) use response_scope::TaskSpaceExecResponseScope;

#[cfg(test)]
#[path = "../taskspace_exec_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "../taskspace_exec_catalog_tests.rs"]
mod catalog_tests;

#[cfg(test)]
#[path = "../taskspace_exec_envelope_tests.rs"]
mod envelope_tests;

#[cfg(test)]
#[path = "../taskspace_exec_preflight_tests.rs"]
mod preflight_tests;

#[cfg(test)]
#[path = "../taskspace_exec_hosted_preflight_tests.rs"]
mod hosted_preflight_tests;

#[cfg(test)]
#[path = "../taskspace_exec_dispatch_tests.rs"]
mod dispatch_tests;

#[cfg(test)]
#[path = "../taskspace_exec_handler_tests.rs"]
mod handler_tests;
