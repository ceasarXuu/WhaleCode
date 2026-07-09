use std::sync::Arc;
use std::time::Instant;

use tokio::sync::RwLock;
use tokio_util::either::Either;
use tokio_util::sync::CancellationToken;
use tokio_util::task::AbortOnDropHandle;
use tracing::Instrument;
use tracing::instrument;
use tracing::trace_span;

use crate::action_map::ActionClass;
use crate::action_map::ToolActionDescriptor;
use crate::function_tool::FunctionCallError;
use crate::session::session::Session;
use crate::session::turn_context::TurnContext;
use crate::tools::context::AbortedToolOutput;
use crate::tools::context::SharedTurnDiffTracker;
use crate::tools::context::ToolPayload;
use crate::tools::context::response_input_model_visible_preview;
use crate::tools::context::tool_output_model_visible_preview;
use crate::tools::registry::AnyToolResult;
use crate::tools::registry::ToolArgumentDiffConsumer;
use crate::tools::router::ToolCall;
use crate::tools::router::ToolCallSource;
use crate::tools::router::ToolRouter;
use codex_protocol::ThreadId;
use codex_protocol::error::CodexErr;
use codex_protocol::models::ResponseInputItem;
use codex_protocol::protocol::SessionSource;
use codex_protocol::protocol::SubAgentSource;
use codex_tools::ToolSpec;

#[derive(Clone)]
pub(crate) struct ToolCallRuntime {
    router: Arc<ToolRouter>,
    session: Arc<Session>,
    turn_context: Arc<TurnContext>,
    tracker: SharedTurnDiffTracker,
    parallel_execution: Arc<RwLock<()>>,
}

impl ToolCallRuntime {
    pub(crate) fn new(
        router: Arc<ToolRouter>,
        session: Arc<Session>,
        turn_context: Arc<TurnContext>,
        tracker: SharedTurnDiffTracker,
    ) -> Self {
        Self {
            router,
            session,
            turn_context,
            tracker,
            parallel_execution: Arc::new(RwLock::new(())),
        }
    }

    pub(crate) fn find_spec(&self, tool_name: &codex_tools::ToolName) -> Option<ToolSpec> {
        self.router.find_spec(tool_name)
    }

    pub(crate) fn create_diff_consumer(
        &self,
        tool_name: &codex_tools::ToolName,
    ) -> Option<Box<dyn ToolArgumentDiffConsumer>> {
        self.router.create_diff_consumer(tool_name)
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) fn handle_tool_call(
        self,
        call: ToolCall,
        cancellation_token: CancellationToken,
    ) -> impl std::future::Future<Output = Result<ResponseInputItem, CodexErr>> {
        let error_call = call.clone();
        let future =
            self.handle_tool_call_with_source(call, ToolCallSource::Direct, cancellation_token);
        async move {
            match future.await {
                Ok(response) => Ok(response.into_response()),
                Err(FunctionCallError::Fatal(message)) => Err(CodexErr::Fatal(message)),
                Err(other) => Ok(Self::failure_response(error_call, other)),
            }
        }
        .in_current_span()
    }

    #[instrument(level = "trace", skip_all)]
    pub(crate) fn handle_tool_call_with_source(
        self,
        call: ToolCall,
        source: ToolCallSource,
        cancellation_token: CancellationToken,
    ) -> impl std::future::Future<Output = Result<AnyToolResult, FunctionCallError>> {
        let supports_parallel = self.router.tool_supports_parallel(&call);
        let router = Arc::clone(&self.router);
        let session = Arc::clone(&self.session);
        let turn = Arc::clone(&self.turn_context);
        let tracker = Arc::clone(&self.tracker);
        let lock = Arc::clone(&self.parallel_execution);
        let invocation_cancellation_token = cancellation_token.clone();
        let started = Instant::now();
        let display_name = call.tool_name.display();
        let taskspace_attributed = Self::should_attribute_taskspace_tool(&call, &source);
        let taskspace_descriptor =
            taskspace_attributed.then(|| Self::classify_taskspace_tool_action(&call));
        let taskspace_tool_name = display_name.clone();
        let taskspace_call_id = call.call_id.clone();

        let dispatch_span = trace_span!(
            "dispatch_tool_call_with_code_mode_result",
            otel.name = display_name.as_str(),
            tool_name = display_name.as_str(),
            call_id = call.call_id.as_str(),
            aborted = false,
        );

        let handle: AbortOnDropHandle<Result<AnyToolResult, FunctionCallError>> =
            AbortOnDropHandle::new(tokio::spawn(async move {
                if let Some(descriptor) = taskspace_descriptor.as_ref() {
                    Self::prepare_taskspace_tool_call(&session, &turn, descriptor.clone()).await?;
                }

                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        let secs = started.elapsed().as_secs_f32().max(0.1);
                        dispatch_span.record("aborted", true);
                        let response = Self::aborted_response(&call, secs);
                        if let Some(descriptor) = taskspace_descriptor.as_ref() {
                            let preview = tool_output_model_visible_preview(
                                response.result.as_ref(),
                                &response.call_id,
                                &response.payload,
                            );
                            let preview =
                                taskspace_tool_result_preview_with_invocation(&call, preview);
                            Self::record_taskspace_tool_result(
                                &session,
                                &turn,
                                &taskspace_call_id,
                                &taskspace_tool_name,
                                Some(descriptor.action_class),
                                false,
                                preview,
                            )
                                .await;
                        }
                        Ok(response)
                    },
                    res = async {
                        let _guard = if supports_parallel {
                            Either::Left(lock.read().await)
                        } else {
                            Either::Right(lock.write().await)
                        };

                        let result = router
                            .dispatch_tool_call_with_code_mode_result(
                                Arc::clone(&session),
                                Arc::clone(&turn),
                                invocation_cancellation_token,
                                tracker,
                                call.clone(),
                                source,
                            )
                            .instrument(dispatch_span.clone())
                            .await;
                        match result {
                            Ok(result) => {
                                if let Some(descriptor) = taskspace_descriptor.as_ref() {
                                    let preview = tool_output_model_visible_preview(
                                        result.result.as_ref(),
                                        &result.call_id,
                                        &result.payload,
                                    );
                                    let preview =
                                        taskspace_tool_result_preview_with_invocation(&call, preview);
                                    Self::record_taskspace_tool_result(
                                        &session,
                                        &turn,
                                        &taskspace_call_id,
                                        &taskspace_tool_name,
                                        Some(descriptor.action_class),
                                        result.result.success_for_logging(),
                                        preview,
                                    )
                                        .await;
                                }
                                Ok(result)
                            }
                            Err(err) => {
                                if let Some(descriptor) = taskspace_descriptor.as_ref() {
                                    let response = Self::failure_response_for_error(&call, &err);
                                    let preview = response_input_model_visible_preview(&response);
                                    let preview =
                                        taskspace_tool_result_preview_with_invocation(&call, preview);
                                    Self::record_taskspace_tool_result(
                                        &session,
                                        &turn,
                                        &taskspace_call_id,
                                        &taskspace_tool_name,
                                        Some(descriptor.action_class),
                                        false,
                                        preview,
                                    )
                                        .await;
                                }
                                Err(err)
                            }
                        }
                    } => res,
                }
            }));

        async move {
            handle.await.map_err(|err| {
                FunctionCallError::Fatal(format!("tool task failed to receive: {err:?}"))
            })?
        }
        .in_current_span()
    }
}

impl ToolCallRuntime {
    fn failure_response(call: ToolCall, err: FunctionCallError) -> ResponseInputItem {
        Self::failure_response_for_error(&call, &err)
    }

    fn failure_response_for_error(call: &ToolCall, err: &FunctionCallError) -> ResponseInputItem {
        let message = function_call_error_model_visible_message(err);
        match &call.payload {
            ToolPayload::ToolSearch { .. } => ResponseInputItem::ToolSearchOutput {
                call_id: call.call_id.clone(),
                status: "completed".to_string(),
                execution: "client".to_string(),
                tools: Vec::new(),
            },
            ToolPayload::Custom { .. } => ResponseInputItem::CustomToolCallOutput {
                call_id: call.call_id.clone(),
                name: None,
                output: codex_protocol::models::FunctionCallOutputPayload {
                    body: codex_protocol::models::FunctionCallOutputBody::Text(message),
                    success: Some(false),
                },
            },
            _ => ResponseInputItem::FunctionCallOutput {
                call_id: call.call_id.clone(),
                output: codex_protocol::models::FunctionCallOutputPayload {
                    body: codex_protocol::models::FunctionCallOutputBody::Text(message),
                    success: Some(false),
                },
            },
        }
    }
}

impl ToolCallRuntime {
    fn aborted_response(call: &ToolCall, secs: f32) -> AnyToolResult {
        AnyToolResult {
            call_id: call.call_id.clone(),
            payload: call.payload.clone(),
            result: Box::new(AbortedToolOutput {
                message: Self::abort_message(call, secs),
            }),
            post_tool_use_payload: None,
        }
    }

    fn abort_message(call: &ToolCall, secs: f32) -> String {
        if call.tool_name.namespace.is_none()
            && matches!(
                call.tool_name.name.as_str(),
                "shell"
                    | "container.exec"
                    | "exec_command"
                    | "local_shell"
                    | "shell_command"
                    | "unified_exec"
            )
        {
            format!("Wall time: {secs:.1} seconds\naborted by user")
        } else {
            format!("aborted by user after {secs:.1}s")
        }
    }

    async fn prepare_taskspace_tool_call(
        session: &Arc<Session>,
        turn: &TurnContext,
        descriptor: ToolActionDescriptor,
    ) -> Result<(), FunctionCallError> {
        if let Some(parent_thread_id) = taskspace_parent_thread_id(&turn.session_source) {
            session
                .services
                .agent_control
                .prepare_action_map_child_tool_call(
                    parent_thread_id,
                    session.conversation_id,
                    descriptor,
                )
                .await
                .map_err(FunctionCallError::RespondToModel)?;
        } else {
            session
                .prepare_action_map_main_tool_call(turn, descriptor)
                .await
                .map_err(FunctionCallError::RespondToModel)?;
        }
        Ok(())
    }

    async fn record_taskspace_tool_result(
        session: &Arc<Session>,
        turn: &TurnContext,
        call_id: &str,
        tool_name: &str,
        action_class: Option<ActionClass>,
        success: bool,
        preview: String,
    ) {
        if let Some(parent_thread_id) = taskspace_parent_thread_id(&turn.session_source) {
            session
                .services
                .agent_control
                .record_action_map_child_tool_result(
                    parent_thread_id,
                    session.conversation_id,
                    call_id,
                    tool_name,
                    action_class,
                    success,
                    preview,
                )
                .await;
        } else {
            session
                .record_action_map_main_tool_result(
                    turn,
                    call_id,
                    tool_name,
                    action_class,
                    success,
                    preview,
                )
                .await;
        }
    }

    fn should_attribute_taskspace_tool(call: &ToolCall, source: &ToolCallSource) -> bool {
        if *source != ToolCallSource::Direct {
            return false;
        }
        if matches!(call.tool_name.name.as_str(), "update_plan") {
            return false;
        }
        if call.tool_name.namespace.is_some() {
            return true;
        }
        !matches!(
            call.tool_name.name.as_str(),
            "spawn_agent"
                | "wait_agent"
                | "close_agent"
                | "resume_agent"
                | "send_input"
                | "send_message"
                | "taskspace_control"
                | "list_agents"
                | "followup_task"
        )
    }

    fn classify_taskspace_tool_action(call: &ToolCall) -> ToolActionDescriptor {
        let tool_name = call.tool_name.display();
        let preview = call.payload.log_payload().to_string();
        let class = taskspace_action_contract_class(&call.call_id)
            .unwrap_or_else(|| classify_tool_payload(&tool_name, &call.payload));
        ToolActionDescriptor::new(tool_name, class, preview).with_call_id(call.call_id.clone())
    }
}

fn taskspace_action_contract_class(call_id: &str) -> Option<ActionClass> {
    let suffix = call_id.strip_prefix("taskspace-action-contract-")?;
    let (_, action) = suffix.rsplit_once('-')?;
    match action {
        "list_files" | "read_file" => Some(ActionClass::Read),
        "search" => Some(ActionClass::Search),
        "apply_patch" => Some(ActionClass::Edit),
        "run_test" => Some(ActionClass::Test),
        "taskspace_control" => Some(ActionClass::Control),
        _ => None,
    }
}

fn taskspace_tool_result_preview_with_invocation(call: &ToolCall, preview: String) -> String {
    let Some(command) = taskspace_tool_command_for_preview(call) else {
        return preview;
    };
    format!(
        "TaskSpaceToolInvocationV1:\n\
tool: {}\n\
command: {command}\n\
raw_output:\n{preview}",
        call.tool_name.display()
    )
}

fn taskspace_tool_command_for_preview(call: &ToolCall) -> Option<String> {
    let tool_name = call.tool_name.display().to_ascii_lowercase();
    if !is_shell_like_tool(&tool_name) {
        return None;
    }
    match &call.payload {
        ToolPayload::Function { arguments }
        | ToolPayload::Mcp {
            raw_arguments: arguments,
            ..
        } => extract_command_from_json(arguments),
        ToolPayload::LocalShell { params } => Some(params.command.join(" ")),
        _ => None,
    }
    .map(|command| command.trim().to_string())
    .filter(|command| !command.is_empty())
}

fn classify_tool_payload(tool_name: &str, payload: &ToolPayload) -> ActionClass {
    let tool = tool_name.to_ascii_lowercase();
    if tool.contains("apply_patch") {
        return ActionClass::Edit;
    }
    if tool.contains("taskspace_control") {
        return ActionClass::Control;
    }
    if tool.contains("spawn_agent") || tool.ends_with("spawn") {
        return ActionClass::Spawn;
    }
    if tool.contains("wait_agent") || tool.contains("close_agent") || tool.contains("resume_agent")
    {
        return ActionClass::Wait;
    }
    if tool.contains("review") {
        return ActionClass::Review;
    }
    match payload {
        ToolPayload::ToolSearch { .. } => ActionClass::Search,
        ToolPayload::LocalShell { params } => classify_shell_text(&params.command.join(" ")),
        ToolPayload::Function { arguments }
        | ToolPayload::Mcp {
            raw_arguments: arguments,
            ..
        } => {
            let command = extract_command_from_json(arguments).unwrap_or_else(|| arguments.clone());
            if is_shell_like_tool(&tool) {
                classify_shell_text(&command)
            } else if tool.contains("search") || tool.contains("find") {
                ActionClass::Search
            } else if tool.contains("read") || tool.contains("open") || tool.contains("list") {
                ActionClass::Read
            } else {
                ActionClass::Unknown
            }
        }
        ToolPayload::Custom { input } => {
            if tool.contains("apply_patch") {
                ActionClass::Edit
            } else {
                classify_shell_text(input)
            }
        }
    }
}

fn taskspace_parent_thread_id(session_source: &SessionSource) -> Option<ThreadId> {
    match session_source {
        SessionSource::SubAgent(SubAgentSource::ThreadSpawn {
            parent_thread_id, ..
        }) => Some(*parent_thread_id),
        _ => None,
    }
}

fn is_shell_like_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "shell"
            | "container.exec"
            | "exec_command"
            | "local_shell"
            | "shell_command"
            | "unified_exec"
    ) || tool_name.ends_with(".shell_command")
        || tool_name.ends_with(".shell")
}

fn extract_command_from_json(arguments: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(arguments).ok()?;
    value
        .get("command")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .or_else(|| {
            value
                .get("cmd")
                .and_then(|value| value.as_str())
                .map(str::to_string)
        })
}

fn classify_shell_text(command: &str) -> ActionClass {
    let lower = command.to_ascii_lowercase();
    let trimmed = lower.trim();
    if trimmed.is_empty() {
        return ActionClass::Unknown;
    }
    let padded = format!(" {trimmed} ");
    let command_words = normalized_shell_words(trimmed);
    let has_test_action = contains_any(
        &lower,
        &[
            "pytest",
            "cargo test",
            "cargo nextest",
            "npm test",
            "npm run test",
            "pnpm test",
            "pnpm run test",
            "yarn test",
            "dotnet test",
            "go test",
            "gradle test",
            "gradlew test",
            "mvn test",
            "connecteddebugandroidtest",
            "python -m unittest",
            "python -m jsonschema",
            "python3 -m jsonschema",
            "py -m jsonschema",
        ],
    ) || has_common_test_command(&command_words)
        || runs_python_test_file(&command_words)
        || runs_python_diagnostic_script(&command_words);
    let has_build_action = contains_any(
        &lower,
        &[
            "cargo build",
            "cargo check",
            "cargo clippy",
            "cargo fmt --check",
            "npm run build",
            "npm run lint",
            "npm run typecheck",
            "npm run check",
            "pnpm run build",
            "pnpm run lint",
            "pnpm run typecheck",
            "pnpm run check",
            "pnpm build",
            "pnpm lint",
            "yarn build",
            "yarn lint",
            "yarn typecheck",
            "yarn run typecheck",
            "dotnet build",
            "gradle build",
            "gradle check",
            "gradlew build",
            "gradlew check",
            "mvn package",
            "mvn verify",
            "rustfmt --check",
            "tsc --noemit",
            "tsc --no-emit",
        ],
    ) || has_common_build_command(trimmed, &command_words);
    let has_edit_action = has_file_redirection(&lower)
        || has_mutating_formatter_command(trimmed, &command_words)
        || contains_any(
            &lower,
            &[
                "apply_patch",
                "set-content",
                "add-content",
                "out-file",
                "new-item",
                "remove-item",
                "move-item",
                "copy-item",
                "rename-item",
                "git stash",
                "git commit",
                "git add",
                "git reset",
                "git clean",
                "git restore",
                "git checkout",
                "git switch",
                "git merge",
                "git rebase",
                "git cherry-pick",
                "git apply",
                "cargo fix",
                "prettier --write",
                "eslint --fix",
                "npm run format",
                "pnpm format",
                "yarn format",
                "sed -i",
                "perl -pi",
                "python -c",
                "python - <<",
                "py -c",
                "node -e",
                "tee ",
                " tee ",
            ],
        )
        || contains_shell_token(
            &command_words,
            &[
                "sc", "ac", "ni", "ri", "rm", "del", "erase", "rd", "rmdir", "mi", "mv", "cpi",
                "cp", "ren", "mkdir", "touch",
            ],
        );
    let has_search_action = contains_any(
        &padded,
        &[
            "rg ",
            " rg ",
            "select-string",
            "grep ",
            " grep ",
            "findstr",
            "search",
        ],
    ) || lower.contains("rg.exe");
    let has_read_action = contains_any(
        &lower,
        &[
            "get-content",
            "get-childitem",
            "get-location",
            "git diff",
            "git status",
            "git log",
            "git show",
        ],
    ) || has_bounded_sed_read_command(trimmed, &command_words)
        || contains_shell_token(&command_words, &["ls", "dir", "cat", "type", "pwd"]);
    if has_edit_action {
        return ActionClass::Edit;
    }
    if has_test_action {
        return ActionClass::Test;
    }
    if has_build_action {
        return ActionClass::Build;
    }
    if has_search_action {
        return ActionClass::Search;
    }
    if has_read_action {
        return ActionClass::Read;
    }
    ActionClass::Unknown
}

fn has_common_test_command(words: &str) -> bool {
    (contains_shell_token(words, &["dotnet", "mvn", "mvnw"])
        && contains_shell_token(words, &["test"]))
        || (contains_shell_token(words, &["gradle", "gradlew"])
            && contains_shell_token(words, &["test"]))
        || (contains_shell_token(words, &["npm", "pnpm", "yarn"])
            && contains_shell_token(words, &["test"]))
}

fn has_common_build_command(command: &str, words: &str) -> bool {
    (contains_shell_token(words, &["gradle", "gradlew"])
        && contains_shell_token(words, &["build", "check"]))
        || (contains_shell_token(words, &["npm", "pnpm", "yarn"])
            && contains_shell_token(words, &["build", "lint", "typecheck", "check"]))
        || (command.contains("cargo fmt") && command.contains("--check"))
        || (contains_shell_token(words, &["rustfmt"]) && command.contains("--check"))
}

fn has_mutating_formatter_command(command: &str, words: &str) -> bool {
    (command.contains("cargo fmt") && !command.contains("--check"))
        || (contains_shell_token(words, &["rustfmt"]) && !command.contains("--check"))
}

fn has_bounded_sed_read_command(command: &str, words: &str) -> bool {
    contains_shell_token(words, &["sed"]) && command.contains("sed -n")
}

fn runs_python_test_file(words: &str) -> bool {
    let tokens = words.split_whitespace().collect::<Vec<_>>();
    tokens.windows(2).any(|pair| {
        matches!(pair[0], "python" | "python3" | "py")
            && pair[1]
                .rsplit(['/', '\\'])
                .next()
                .is_some_and(|name| name.starts_with("test_") && name.ends_with(".py"))
    })
}

fn runs_python_diagnostic_script(words: &str) -> bool {
    let tokens = words.split_whitespace().collect::<Vec<_>>();
    tokens.windows(2).any(|pair| {
        matches!(pair[0], "python" | "python3" | "py")
            && pair[1].ends_with(".py")
            && !pair[1].starts_with('-')
    })
}

fn has_file_redirection(command: &str) -> bool {
    command.char_indices().any(|(index, ch)| {
        if ch != '>' {
            return false;
        }
        let after = command[index + ch.len_utf8()..].trim_start();
        !after.is_empty() && !after.starts_with('&')
    })
}

fn normalized_shell_words(command: &str) -> String {
    let normalized: String = command
        .chars()
        .map(|ch| match ch {
            ';' | '&' | '|' | '\n' | '\r' | '\t' | '(' | ')' | '{' | '}' | '[' | ']' | '"'
            | '\'' | ',' => ' ',
            _ => ch,
        })
        .collect();
    format!(" {normalized} ")
}

fn contains_shell_token(words: &str, tokens: &[&str]) -> bool {
    tokens
        .iter()
        .any(|token| words.contains(&format!(" {token} ")))
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| haystack.contains(needle))
}

fn function_call_error_model_visible_message(err: &FunctionCallError) -> String {
    match err {
        FunctionCallError::RespondToModel(message) => {
            taskspace_safe_local_validator_infra_error_summary(message)
                .unwrap_or_else(|| message.clone())
        }
        FunctionCallError::MissingLocalShellCallId => {
            "Tool call failed because the shell call id was missing.".to_string()
        }
        FunctionCallError::Fatal(_) => "Tool call failed with a fatal runtime error.".to_string(),
    }
}

fn taskspace_safe_local_validator_infra_error_summary(message: &str) -> Option<String> {
    let signal = taskspace_compact_ascii_signal(message);
    let has_bash_access_denied = signal.contains("bashservicecreateinstanceeaccessdenied")
        || signal.contains("bashservicecreateinstancee_accessdenied")
        || signal.contains("eaccessdenied")
        || signal.contains("e_accessdenied");
    if has_bash_access_denied {
        return Some(
            "Tool call failed before producing a result. local_validator_infra_failure: Bash/Service/CreateInstance/E_ACCESSDENIED"
                .to_string(),
        );
    }
    if signal.contains("invalidendofline") || signal.contains("notavalidstatementseparator") {
        return Some(
            "Tool call failed before producing a result. local_validator_infra_failure: PowerShell InvalidEndOfLine"
                .to_string(),
        );
    }
    None
}

fn taskspace_compact_ascii_signal(text: &str) -> String {
    text.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::ToolCallRuntime;
    use super::classify_shell_text;
    use super::classify_tool_payload;
    use super::taskspace_action_contract_class;
    use super::taskspace_tool_result_preview_with_invocation;
    use crate::action_map::ActionClass;
    use crate::function_tool::FunctionCallError;
    use crate::tools::context::ToolPayload;
    use crate::tools::context::response_input_model_visible_preview;
    use crate::tools::router::ToolCall;
    use codex_tools::ToolName;

    fn failure_response_preview(err: FunctionCallError) -> String {
        let call = ToolCall {
            tool_name: ToolName::plain("apply_patch"),
            call_id: "call-test".to_string(),
            payload: ToolPayload::Function {
                arguments: "{}".to_string(),
            },
        };
        let response = ToolCallRuntime::failure_response_for_error(&call, &err);
        response_input_model_visible_preview(&response)
    }

    #[test]
    fn failure_response_preview_records_model_visible_error_text() {
        let preview = failure_response_preview(FunctionCallError::RespondToModel(
            "failed to parse apply_patch: missing field `action`".to_string(),
        ));

        assert!(preview.contains("failed to parse apply_patch"));
        assert!(preview.contains("missing field `action`"));
    }

    #[test]
    fn failure_response_preview_bounds_model_visible_error_text() {
        let long_error = format!("apply_patch failed\n{}", "line\n".repeat(128));
        let preview = failure_response_preview(FunctionCallError::RespondToModel(long_error));

        assert!(preview.contains("apply_patch failed"));
        assert!(preview.contains("telemetry preview truncated"));
    }

    #[test]
    fn failure_response_preview_keeps_safe_local_validator_infra_signal() {
        let nul_separated = "Bash/Service/CreateInstance/E_ACCESSDENIED"
            .chars()
            .flat_map(|ch| [ch, '\0'])
            .collect::<String>();
        let preview = failure_response_preview(FunctionCallError::RespondToModel(format!(
            "garbled host output: {nul_separated}"
        )));

        assert!(preview.contains("local_validator_infra_failure"));
        assert!(preview.contains("Bash/Service/CreateInstance/E_ACCESSDENIED"));
        assert!(!preview.contains("garbled host output"));
    }

    #[test]
    fn taskspace_tool_result_preview_keeps_shell_command_context() {
        let call = ToolCall {
            tool_name: ToolName::plain("shell_command"),
            call_id: "taskspace-action-contract-16-run_test".to_string(),
            payload: ToolPayload::Function {
                arguments: serde_json::json!({
                    "command": "python scripts/validate.py",
                    "timeout_ms": 120000
                })
                .to_string(),
            },
        };
        let preview =
            taskspace_tool_result_preview_with_invocation(&call, "validator passed".to_string());

        assert!(preview.contains("TaskSpaceToolInvocationV1"));
        assert!(preview.contains("tool: shell_command"));
        assert!(preview.contains("command: python scripts/validate.py"));
        assert!(preview.contains("raw_output:"));
        assert!(preview.contains("validator passed"));
    }

    #[test]
    fn shell_action_classifier_identifies_core_taskspace_classes() {
        assert_eq!(
            classify_shell_text("Get-Content src/lib.rs"),
            ActionClass::Read
        );
        assert_eq!(classify_shell_text("ls"), ActionClass::Read);
        assert_eq!(classify_shell_text("dir src"), ActionClass::Read);
        assert_eq!(classify_shell_text("Get-Location"), ActionClass::Read);
        assert_eq!(
            classify_shell_text(
                "sed -n '1,240p' -- package.json && awk 'NR == 241 { exit }' package.json"
            ),
            ActionClass::Read
        );
        assert_eq!(
            classify_shell_text(
                "printf '===== %s\\n' package.json; sed -n '1,240p' -- package.json && awk 'NR == 241 { exit }' package.json"
            ),
            ActionClass::Read
        );
        assert_eq!(
            classify_shell_text(
                "printf '===== %s\\n' employees.csv; sed -n '1,240p' -- employees.csv && awk 'NR == 241 { truncated = 1; exit } { lines = NR } END { eof = truncated ? \"false\" : \"true\"; if (240 < lines) lines = 240; printf \"\\nTaskSpaceReadFileSummaryV1: path=%s lines_read=%d eof_reached=%s max_lines=240\\n\", FILENAME, lines + 0, eof }' employees.csv"
            ),
            ActionClass::Read
        );
        assert_eq!(
            classify_shell_text("cmd /c \"dir /s /b repo\\*.py\""),
            ActionClass::Read
        );
        assert_eq!(
            classify_shell_text("rg \"TaskSpace\" src"),
            ActionClass::Search
        );
        assert_eq!(classify_shell_text("grep -R foo src"), ActionClass::Search);
        assert_eq!(classify_shell_text("pytest -q"), ActionClass::Test);
        assert_eq!(
            classify_shell_text("python -m pytest tests/ -v 2>&1"),
            ActionClass::Test
        );
        assert_eq!(
            classify_shell_text(
                "node process.js && python -m jsonschema -i organization.json schema.json"
            ),
            ActionClass::Test
        );
        assert_eq!(
            classify_shell_text("python test_pricing.py"),
            ActionClass::Test
        );
        assert_eq!(
            classify_shell_text("python scripts/emit_large_log.py"),
            ActionClass::Test
        );
        assert_eq!(
            classify_shell_text("py tools\\diagnose_failure.py"),
            ActionClass::Test
        );
        assert_eq!(
            classify_shell_text("py tests\\test_pricing.py"),
            ActionClass::Test
        );
        assert_eq!(
            classify_shell_text("pytest -q > pytest.log"),
            ActionClass::Edit
        );
        assert_eq!(
            classify_shell_text("pytest -q 2> pytest.err.log"),
            ActionClass::Edit
        );
        assert_eq!(
            classify_shell_text("git stash push -- src/lib.rs; python -m pytest tests -q"),
            ActionClass::Edit
        );
        assert_eq!(classify_shell_text("cargo check"), ActionClass::Build);
        assert_eq!(
            classify_shell_text("cargo build --release"),
            ActionClass::Build
        );
        assert_eq!(classify_shell_text("cargo fmt --check"), ActionClass::Build);
        assert_eq!(
            classify_shell_text("rustfmt --check src/lib.rs"),
            ActionClass::Build
        );
        assert_eq!(classify_shell_text("dotnet test"), ActionClass::Test);
        assert_eq!(classify_shell_text("mvn -q test"), ActionClass::Test);
        assert_eq!(classify_shell_text("gradlew build"), ActionClass::Build);
        assert_eq!(classify_shell_text("npm run build"), ActionClass::Build);
        assert_eq!(classify_shell_text("npm run typecheck"), ActionClass::Build);
        assert_eq!(classify_shell_text("pnpm run build"), ActionClass::Build);
        assert_eq!(
            classify_shell_text("pnpm run typecheck"),
            ActionClass::Build
        );
        assert_eq!(classify_shell_text("yarn typecheck"), ActionClass::Build);
        assert_eq!(classify_shell_text("tsc --noEmit"), ActionClass::Build);
        assert_eq!(
            classify_shell_text("cargo build && cargo test"),
            ActionClass::Test
        );
        assert_eq!(
            classify_shell_text("Set-Content file.txt value"),
            ActionClass::Edit
        );
        assert_eq!(
            classify_shell_text("Set-Content file.txt value; pytest -q"),
            ActionClass::Edit
        );
        assert_eq!(classify_shell_text("pytest -q>out.log"), ActionClass::Edit);
        assert_eq!(classify_shell_text("rm file; pytest -q"), ActionClass::Edit);
        assert_eq!(
            classify_shell_text("del file & pytest -q"),
            ActionClass::Edit
        );
        assert_eq!(
            classify_shell_text("sc file value; pytest -q"),
            ActionClass::Edit
        );
        assert_eq!(classify_shell_text("ni file; pytest -q"), ActionClass::Edit);
        assert_eq!(classify_shell_text("mv a b; pytest -q"), ActionClass::Edit);
        assert_eq!(
            classify_shell_text("sed -i s/a/b/ file; pytest -q"),
            ActionClass::Edit
        );
        assert_eq!(
            classify_shell_text("python -c \"open('x','w').write('y')\"; pytest -q"),
            ActionClass::Edit
        );
        assert_eq!(
            classify_shell_text("python scripts/emit_large_log.py > out.log"),
            ActionClass::Edit
        );
        assert_eq!(classify_shell_text("git stash"), ActionClass::Edit);
        assert_eq!(classify_shell_text("cargo fmt"), ActionClass::Edit);
        assert_eq!(classify_shell_text("rustfmt src/lib.rs"), ActionClass::Edit);
        assert_eq!(
            classify_shell_text("some-unknown-tool"),
            ActionClass::Unknown
        );
    }

    #[test]
    fn exec_command_alias_uses_shell_action_classification() {
        assert_eq!(
            classify_tool_payload(
                "exec_command",
                &ToolPayload::Function {
                    arguments: serde_json::json!({ "cmd": "cat README.md" }).to_string(),
                },
            ),
            ActionClass::Read
        );
        assert_eq!(
            classify_tool_payload(
                "exec_command",
                &ToolPayload::Function {
                    arguments: serde_json::json!({ "cmd": "pytest -q" }).to_string(),
                },
            ),
            ActionClass::Test
        );
    }

    #[test]
    fn taskspace_action_contract_call_id_preserves_run_test_class() {
        assert_eq!(
            taskspace_action_contract_class("taskspace-action-contract-25-run_test"),
            Some(ActionClass::Test)
        );
        assert_eq!(
            taskspace_action_contract_class(
                "taskspace-action-contract-bootstrap-taskspace_control"
            ),
            Some(ActionClass::Control)
        );
        assert_eq!(
            classify_shell_text("bash /app/run_pipeline.sh"),
            ActionClass::Unknown
        );
    }
}
