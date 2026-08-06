mod map_operations;

pub(crate) use map_operations::MapOperation;
pub(crate) use map_operations::MapOperationApplyError;
pub(crate) use map_operations::MapOperationEffect;
pub(crate) use map_operations::apply_map_operation;
pub(crate) use map_operations::map_operation_capabilities;

#[cfg(test)]
#[path = "../taskspace_exec_tests.rs"]
mod tests;
