use std::sync::Arc;
use std::sync::Mutex;

use codex_protocol::models::ResponseItem;

use super::HostedOutputFact;
use super::TASKSPACE_EXEC_TOOL_NAME;
use super::hosted::hosted_output_fact;

#[derive(Debug)]
pub(crate) struct TaskSpaceExecResponseScope {
    capability_identity: Arc<str>,
    state: Mutex<ResponseScopeState>,
}

#[derive(Debug, Default)]
struct ResponseScopeState {
    request: Option<TaskSpaceExecRequestSnapshot>,
    response: Option<TaskSpaceExecResponseIdentity>,
    facts: Vec<HostedOutputFact>,
    complete: bool,
    terminal: bool,
    error: Option<String>,
    exec_call_id: Option<String>,
    exec_call_count: usize,
    exec_claimed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceExecRequestSnapshot {
    pub(crate) map_id: String,
    pub(crate) revision: Option<u64>,
    pub(crate) capability_identity: Arc<str>,
}

#[derive(Debug, Clone)]
pub(crate) struct TaskSpaceExecResponseClaim {
    pub(crate) request: TaskSpaceExecRequestSnapshot,
    pub(crate) response: TaskSpaceExecResponseIdentity,
    pub(crate) hosted_facts: Vec<HostedOutputFact>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceExecResponseIdentity {
    pub(crate) provider_response_id: String,
    pub(crate) provider_request_id: Option<String>,
    pub(crate) provider_logical_request_id: Option<String>,
    pub(crate) provider_attempt_seq: Option<usize>,
}

impl TaskSpaceExecResponseScope {
    pub(crate) fn new(capability_identity: Arc<str>) -> Self {
        Self {
            capability_identity,
            state: Mutex::new(ResponseScopeState::default()),
        }
    }

    pub(crate) fn capability_identity(&self) -> Arc<str> {
        Arc::clone(&self.capability_identity)
    }

    pub(crate) fn begin_request(
        &self,
        map_id: impl Into<String>,
        revision: Option<u64>,
    ) -> Result<(), String> {
        let map_id = map_id.into();
        if map_id.trim().is_empty() {
            return Err("TaskSpace provider request has an empty Map identity".to_string());
        }
        *self
            .state
            .lock()
            .expect("TaskSpace response scope poisoned") = ResponseScopeState {
            request: Some(TaskSpaceExecRequestSnapshot {
                map_id,
                revision,
                capability_identity: Arc::clone(&self.capability_identity),
            }),
            ..ResponseScopeState::default()
        };
        Ok(())
    }

    pub(crate) fn record_completed_item(&self, output_index: Option<usize>, item: &ResponseItem) {
        if let ResponseItem::FunctionCall {
            name,
            namespace,
            call_id,
            ..
        } = item
            && namespace.is_none()
            && name == TASKSPACE_EXEC_TOOL_NAME
        {
            let mut state = self
                .state
                .lock()
                .expect("TaskSpace response scope poisoned");
            if state.terminal {
                state.error =
                    Some("TaskSpace Exec arrived after response finalization".to_string());
                return;
            }
            state.exec_call_count = state.exec_call_count.saturating_add(1);
            if state.exec_call_id.is_none() {
                state.exec_call_id = Some(call_id.clone());
            }
            return;
        }
        if let Some(tool) = unexpected_client_tool(item) {
            let mut state = self
                .state
                .lock()
                .expect("TaskSpace response scope poisoned");
            if state.error.is_none() {
                state.error = Some(format!(
                    "TaskSpace response contains forbidden top-level client Tool `{tool}`"
                ));
            }
            return;
        }
        let observed = hosted_output_fact(output_index, item);
        if matches!(observed, Ok(None)) {
            return;
        }
        let mut state = self
            .state
            .lock()
            .expect("TaskSpace response scope poisoned");
        if state.terminal {
            state.error = Some("hosted output arrived after response finalization".to_string());
            return;
        }
        match observed {
            Ok(Some(fact)) => state.facts.push(fact),
            Ok(None) => {}
            Err(error) => state.error = Some(error),
        }
    }

    pub(crate) fn finalize(
        &self,
        response_completed: bool,
        response: Option<TaskSpaceExecResponseIdentity>,
    ) -> Result<(), String> {
        let mut state = self
            .state
            .lock()
            .expect("TaskSpace response scope poisoned");
        state.response = response;
        state.terminal = true;
        state.complete = response_completed;
        let result = validate_finalized(&state);
        let request = state.request.as_ref();
        let response = state.response.as_ref();
        tracing::info!(
            target: "codex_core::taskspace_exec",
            event_name = "taskspace.exec.response_finalized",
            reason_code = if result.is_ok() { "accepted" } else { "response_contract_rejected" },
            provider_request_id = response.and_then(|value| value.provider_request_id.as_deref()).unwrap_or(""),
            provider_logical_request_id = response.and_then(|value| value.provider_logical_request_id.as_deref()).unwrap_or(""),
            provider_attempt_seq = ?response.and_then(|value| value.provider_attempt_seq),
            provider_response_id = response.map(|value| value.provider_response_id.as_str()).unwrap_or(""),
            outer_call_id = state.exec_call_id.as_deref().unwrap_or(""),
            map_id = request.map(|value| value.map_id.as_str()).unwrap_or(""),
            request_revision = ?request.and_then(|value| value.revision),
            capability_identity = request.map(|value| value.capability_identity.as_ref()).unwrap_or(""),
            response_completed,
            exec_call_count = state.exec_call_count,
            hosted_fact_count = state.facts.len(),
            accepted = result.is_ok(),
        );
        result
    }

    pub(crate) fn claim_response(
        &self,
        outer_call_id: &str,
    ) -> Result<TaskSpaceExecResponseClaim, String> {
        let mut state = self
            .state
            .lock()
            .expect("TaskSpace response scope poisoned");
        validate_finalized(&state)?;
        if state.exec_call_id.as_deref() != Some(outer_call_id) {
            return Err("TaskSpace Exec call identity does not match the response".to_string());
        }
        if state.exec_claimed {
            return Err("TaskSpace response was already claimed by another Exec call".to_string());
        }
        state.exec_claimed = true;
        Ok(TaskSpaceExecResponseClaim {
            request: state
                .request
                .clone()
                .ok_or_else(|| "TaskSpace response has no request Map snapshot".to_string())?,
            response: state.response.clone().ok_or_else(|| {
                "TaskSpace response has no provider response identity".to_string()
            })?,
            hosted_facts: state.facts.clone(),
        })
    }

    pub(crate) fn ensure_reconciled(&self) -> Result<(), String> {
        let state = self
            .state
            .lock()
            .expect("TaskSpace response scope poisoned");
        validate_finalized(&state)?;
        if state.exec_call_id.is_some() && !state.exec_claimed {
            return Err("TaskSpace Exec response was not reconciled".to_string());
        }
        Ok(())
    }
}

#[cfg(test)]
impl Default for TaskSpaceExecResponseScope {
    fn default() -> Self {
        Self::new(Arc::from("test-capability-identity"))
    }
}

fn validate_finalized(state: &ResponseScopeState) -> Result<(), String> {
    if !state.terminal {
        return Err("provider response did not complete before TaskSpace Exec".to_string());
    }
    if !state.complete && (state.exec_call_count > 0 || !state.facts.is_empty()) {
        return Err("provider response did not complete before TaskSpace Exec".to_string());
    }
    if !state.complete {
        return Ok(());
    }
    if let Some(error) = state.error.as_ref() {
        return Err(error.clone());
    }
    if state.exec_call_count > 1 {
        return Err("TaskSpace response contains more than one Exec call".to_string());
    }
    if state.exec_call_count == 1 && state.request.is_none() {
        return Err("TaskSpace response has no request Map snapshot".to_string());
    }
    if state.exec_call_count == 1 && state.response.is_none() {
        return Err("TaskSpace response has no provider response identity".to_string());
    }
    if !state.facts.is_empty() && state.exec_call_count != 1 {
        return Err("provider-hosted outputs require exactly one TaskSpace Exec call".to_string());
    }
    Ok(())
}

fn unexpected_client_tool(item: &ResponseItem) -> Option<String> {
    match item {
        ResponseItem::FunctionCall {
            name, namespace, ..
        } if namespace.is_some() || name != TASKSPACE_EXEC_TOOL_NAME => Some(
            namespace
                .as_ref()
                .map_or_else(|| name.clone(), |namespace| format!("{namespace}.{name}")),
        ),
        ResponseItem::LocalShellCall { .. } => Some("local_shell".to_string()),
        ResponseItem::CustomToolCall { name, .. } => Some(name.clone()),
        ResponseItem::ToolSearchCall { .. } => Some("tool_search".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_map::rooted_dag::ActionOutcome;
    use codex_protocol::models::WebSearchAction;
    use tracing_test::traced_test;

    fn response_identity() -> TaskSpaceExecResponseIdentity {
        TaskSpaceExecResponseIdentity {
            provider_response_id: "response-1".into(),
            provider_request_id: Some("request-1".into()),
            provider_logical_request_id: Some("logical-1".into()),
            provider_attempt_seq: Some(1),
        }
    }

    #[test]
    #[traced_test]
    fn scope_preserves_provider_identity_order_and_terminal_outcome() {
        let scope = TaskSpaceExecResponseScope::default();
        scope.begin_request("map-1", Some(7)).unwrap();
        scope.record_completed_item(
            Some(3),
            &ResponseItem::WebSearchCall {
                id: Some("ws-1".into()),
                status: Some("completed".into()),
                action: Some(WebSearchAction::Search {
                    query: Some("query".into()),
                    queries: None,
                }),
            },
        );
        scope.record_completed_item(
            Some(4),
            &ResponseItem::FunctionCall {
                id: None,
                name: TASKSPACE_EXEC_TOOL_NAME.into(),
                namespace: None,
                arguments: "{}".into(),
                call_id: "outer".into(),
            },
        );
        scope.finalize(true, Some(response_identity())).unwrap();
        let claim = scope.claim_response("outer").unwrap();
        assert_eq!(claim.request.map_id, "map-1");
        assert_eq!(claim.request.revision, Some(7));
        assert_eq!(claim.hosted_facts[0].output_index, 3);
        assert_eq!(claim.hosted_facts[0].provider_id, "ws-1");
        assert_eq!(claim.hosted_facts[0].outcome, ActionOutcome::Succeeded);
        assert_eq!(claim.response, response_identity());
        logs_assert(|lines: &[&str]| {
            lines
                .iter()
                .find(|line| {
                    line.contains("taskspace.exec.response_finalized")
                        && line.contains("provider_request_id=\"request-1\"")
                        && line.contains("provider_response_id=\"response-1\"")
                        && line.contains("outer_call_id=\"outer\"")
                        && line.contains("map_id=\"map-1\"")
                })
                .map(|_| Ok(()))
                .unwrap_or_else(|| Err("expected correlated response event".to_string()))
        });
    }

    #[test]
    fn incomplete_response_is_not_accepted() {
        let scope = TaskSpaceExecResponseScope::default();
        scope.begin_request("map-1", Some(7)).unwrap();
        scope.record_completed_item(
            Some(1),
            &ResponseItem::FunctionCall {
                id: None,
                name: TASKSPACE_EXEC_TOOL_NAME.into(),
                namespace: None,
                arguments: "{}".into(),
                call_id: "outer".into(),
            },
        );
        assert!(scope.finalize(false, None).is_err());
        assert!(
            scope
                .claim_response("outer")
                .unwrap_err()
                .contains("did not complete")
        );
    }

    #[test]
    fn incomplete_unrelated_response_preserves_the_native_error_path() {
        let scope = TaskSpaceExecResponseScope::default();
        scope.finalize(false, None).unwrap();
        scope.ensure_reconciled().unwrap();
    }

    #[test]
    fn hosted_outputs_require_wire_index_and_one_exec() {
        let scope = TaskSpaceExecResponseScope::default();
        scope.record_completed_item(
            None,
            &ResponseItem::WebSearchCall {
                id: Some("ws-1".into()),
                status: Some("completed".into()),
                action: None,
            },
        );
        assert!(
            scope
                .finalize(true, Some(response_identity()))
                .unwrap_err()
                .contains("output_index")
        );

        let scope = TaskSpaceExecResponseScope::default();
        scope.record_completed_item(
            Some(1),
            &ResponseItem::WebSearchCall {
                id: Some("ws-1".into()),
                status: Some("completed".into()),
                action: None,
            },
        );
        assert!(
            scope
                .finalize(true, Some(response_identity()))
                .unwrap_err()
                .contains("exactly one")
        );
    }

    #[test]
    fn response_rejects_multiple_exec_calls_and_requires_one_claim() {
        let scope = TaskSpaceExecResponseScope::default();
        scope.begin_request("map-1", Some(7)).unwrap();
        for call_id in ["outer-1", "outer-2"] {
            scope.record_completed_item(
                Some(1),
                &ResponseItem::FunctionCall {
                    id: None,
                    name: TASKSPACE_EXEC_TOOL_NAME.into(),
                    namespace: None,
                    arguments: "{}".into(),
                    call_id: call_id.into(),
                },
            );
        }
        assert!(
            scope
                .finalize(true, Some(response_identity()))
                .unwrap_err()
                .contains("more than one")
        );

        let scope = TaskSpaceExecResponseScope::default();
        scope.begin_request("map-1", Some(7)).unwrap();
        scope.record_completed_item(
            Some(1),
            &ResponseItem::FunctionCall {
                id: None,
                name: TASKSPACE_EXEC_TOOL_NAME.into(),
                namespace: None,
                arguments: "{}".into(),
                call_id: "outer".into(),
            },
        );
        scope.finalize(true, Some(response_identity())).unwrap();
        assert!(scope.ensure_reconciled().is_err());
        scope.claim_response("outer").unwrap();
        scope.ensure_reconciled().unwrap();
    }

    #[test]
    fn response_rejects_forbidden_top_level_client_call_before_exec_claim() {
        let scope = TaskSpaceExecResponseScope::default();
        scope.begin_request("map-1", Some(7)).unwrap();
        scope.record_completed_item(
            Some(0),
            &ResponseItem::FunctionCall {
                id: None,
                name: "inspect".into(),
                namespace: None,
                arguments: "{}".into(),
                call_id: "bypass".into(),
            },
        );
        scope.record_completed_item(
            Some(1),
            &ResponseItem::FunctionCall {
                id: None,
                name: TASKSPACE_EXEC_TOOL_NAME.into(),
                namespace: None,
                arguments: "{}".into(),
                call_id: "outer".into(),
            },
        );

        let error = scope.finalize(true, Some(response_identity())).unwrap_err();
        assert!(error.contains("forbidden top-level client Tool `inspect`"));
        assert!(scope.claim_response("outer").is_err());
    }

    #[test]
    fn exec_response_requires_request_time_map_snapshot() {
        let scope = TaskSpaceExecResponseScope::default();
        scope.record_completed_item(
            Some(0),
            &ResponseItem::FunctionCall {
                id: None,
                name: TASKSPACE_EXEC_TOOL_NAME.into(),
                namespace: None,
                arguments: "{}".into(),
                call_id: "outer".into(),
            },
        );

        assert!(
            scope
                .finalize(true, Some(response_identity()))
                .unwrap_err()
                .contains("no request Map snapshot")
        );
    }
}
