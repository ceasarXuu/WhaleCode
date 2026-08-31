use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

use anyhow::Context;
use anyhow::Result;
use codex_core::config::Config;
use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionFuture;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::JsonToolOutput;
use codex_extension_api::ResponsesApiTool;
use codex_extension_api::ToolBatchPreflightContributor;
use codex_extension_api::ToolBatchPreflightFailure;
use codex_extension_api::ToolBatchPreflightInput;
use codex_extension_api::ToolCall;
use codex_extension_api::ToolContributor;
use codex_extension_api::ToolExecutor;
use codex_extension_api::ToolExecutorFuture;
use codex_extension_api::ToolName;
use codex_extension_api::ToolOutput;
use codex_extension_api::ToolSpec;
use codex_tools::JsonSchema;
use core_test_support::responses;
use core_test_support::test_codex::test_codex;

struct RejectingToolBatchExtension {
    dispatch_count: Arc<AtomicUsize>,
}

impl ToolContributor for RejectingToolBatchExtension {
    fn tools(
        &self,
        _session_store: &ExtensionData,
        _thread_store: &ExtensionData,
    ) -> Vec<Arc<dyn for<'call> ToolExecutor<ToolCall<'call>>>> {
        vec![Arc::new(CountingTool {
            dispatch_count: Arc::clone(&self.dispatch_count),
        })]
    }
}

impl ToolBatchPreflightContributor for RejectingToolBatchExtension {
    fn preflight<'a>(
        &'a self,
        input: ToolBatchPreflightInput<'a>,
    ) -> ExtensionFuture<'a, std::result::Result<(), ToolBatchPreflightFailure>> {
        Box::pin(async move {
            assert_eq!(input.calls.len(), 1);
            Err(ToolBatchPreflightFailure::new(
                "test_batch_rejected",
                "the test batch must not dispatch",
            ))
        })
    }
}

struct CountingTool {
    dispatch_count: Arc<AtomicUsize>,
}

impl<'call> ToolExecutor<ToolCall<'call>> for CountingTool {
    fn tool_name(&self) -> ToolName {
        ToolName::plain("count_dispatch")
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::Function(ResponsesApiTool {
            name: "count_dispatch".into(),
            description: "Count test dispatches.".into(),
            strict: false,
            defer_loading: None,
            parameters: JsonSchema::object(BTreeMap::new(), None, Some(false.into())),
            output_schema: None,
        })
    }

    fn handle<'a>(&'a self, _call: ToolCall<'call>) -> ToolExecutorFuture<'a>
    where
        'call: 'a,
    {
        Box::pin(async move {
            self.dispatch_count.fetch_add(1, Ordering::SeqCst);
            Ok(
                Box::new(JsonToolOutput::new(serde_json::json!({"ok": true})))
                    as Box<dyn ToolOutput>,
            )
        })
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejected_complete_response_closes_call_without_dispatch() -> Result<()> {
    let server = responses::start_mock_server().await;
    let dispatch_count = Arc::new(AtomicUsize::new(0));
    let extension = Arc::new(RejectingToolBatchExtension {
        dispatch_count: Arc::clone(&dispatch_count),
    });
    let mut extensions = ExtensionRegistryBuilder::<Config>::new();
    extensions.tool_contributor(extension.clone());
    extensions.tool_batch_preflight_contributor(extension);
    let mut builder = test_codex().with_extensions(Arc::new(extensions.build()));
    let test = builder.build(&server).await?;
    let call_id = "rejected-call";
    let response_mock = responses::mount_sse_sequence(
        &server,
        vec![
            responses::sse(vec![
                responses::ev_response_created("resp-1"),
                responses::ev_function_call(call_id, "count_dispatch", "{}"),
                responses::ev_completed("resp-1"),
            ]),
            responses::sse(vec![
                responses::ev_response_created("resp-2"),
                responses::ev_assistant_message("msg-1", "done"),
                responses::ev_completed("resp-2"),
            ]),
        ],
    )
    .await;

    test.submit_turn("exercise the tool batch gate").await?;

    assert_eq!(dispatch_count.load(Ordering::SeqCst), 0);
    let request = response_mock
        .last_request()
        .context("missing follow-up request with preflight output")?;
    let output = request
        .function_call_output_text(call_id)
        .context("preflight rejection must close the provider call")?;
    assert!(output.contains("test_batch_rejected"), "{output}");
    Ok(())
}
