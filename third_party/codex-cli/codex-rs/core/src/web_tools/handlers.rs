use crate::function_tool::FunctionCallError;
use crate::tools::context::FunctionToolOutput;
use crate::tools::context::ToolInvocation;
use crate::tools::context::ToolPayload;
use crate::tools::handlers::parse_arguments;
use crate::tools::registry::ToolHandler;
use crate::tools::registry::ToolKind;
use crate::web_tools::WebToolError;
use crate::web_tools::providers::WebFetchArgs;
use crate::web_tools::providers::WebProviderRegistry;
use crate::web_tools::providers::WebSearchArgs;
use codex_protocol::config_types::WebSearchConfig;
use codex_protocol::models::WebSearchAction;
use codex_protocol::protocol::EventMsg;
use codex_protocol::protocol::WebFetchBeginEvent;
use codex_protocol::protocol::WebFetchEndEvent;
use codex_protocol::protocol::WebSearchBeginEvent;
use codex_protocol::protocol::WebSearchEndEvent;
use serde::Serialize;
use sha1::Digest;
use sha1::Sha1;
use tracing::info;

pub(crate) struct WebSearchHandler;
pub(crate) struct WebFetchHandler;

impl ToolHandler for WebSearchHandler {
    type Output = FunctionToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            call_id,
            payload,
            ..
        } = invocation;
        let ToolPayload::Function { arguments } = payload else {
            return Err(FunctionCallError::RespondToModel(
                "web_search handler received unsupported payload".to_string(),
            ));
        };
        let args: WebSearchArgs = parse_arguments(&arguments)?;
        let query_for_event = args.query.clone();
        let config = runtime_config_for_search(turn.config.web_search_config.clone())?;
        let registry = WebProviderRegistry::new(config, turn.config.codex_home.to_path_buf())
            .map_err(to_model_error)?;

        session
            .send_event(
                turn.as_ref(),
                EventMsg::WebSearchBegin(WebSearchBeginEvent {
                    call_id: call_id.clone(),
                }),
            )
            .await;
        info!(
            target: "codex_core::web_tools",
            tool = "web_search",
            query_hash = %hash_for_log(&query_for_event),
            "web search started"
        );

        let result = registry.search(args).await.map_err(to_model_error)?;
        session
            .send_event(
                turn.as_ref(),
                EventMsg::WebSearchEnd(WebSearchEndEvent {
                    call_id,
                    query: query_for_event.clone(),
                    action: WebSearchAction::Search {
                        query: Some(query_for_event),
                        queries: None,
                    },
                }),
            )
            .await;
        json_output(result)
    }
}

impl ToolHandler for WebFetchHandler {
    type Output = FunctionToolOutput;

    fn kind(&self) -> ToolKind {
        ToolKind::Function
    }

    async fn handle(&self, invocation: ToolInvocation) -> Result<Self::Output, FunctionCallError> {
        let ToolInvocation {
            session,
            turn,
            call_id,
            payload,
            ..
        } = invocation;
        let ToolPayload::Function { arguments } = payload else {
            return Err(FunctionCallError::RespondToModel(
                "web_fetch handler received unsupported payload".to_string(),
            ));
        };
        let args: WebFetchArgs = parse_arguments(&arguments)?;
        let config = runtime_config_for_fetch(turn.config.web_search_config.clone())?;
        let registry = WebProviderRegistry::new(config, turn.config.codex_home.to_path_buf())
            .map_err(to_model_error)?;

        session
            .send_event(
                turn.as_ref(),
                EventMsg::WebFetchBegin(WebFetchBeginEvent {
                    call_id: call_id.clone(),
                }),
            )
            .await;
        info!(
            target: "codex_core::web_tools",
            tool = "web_fetch",
            url_hash = %hash_for_log(&args.url),
            "web fetch started"
        );

        match registry.fetch(args).await {
            Ok(result) => {
                session
                    .send_event(
                        turn.as_ref(),
                        EventMsg::WebFetchEnd(WebFetchEndEvent {
                            call_id,
                            url: result.url.clone(),
                            final_url: result.final_url.clone(),
                            status: "ok".to_string(),
                            truncated: result.truncated,
                            content_chars: result.content_chars,
                        }),
                    )
                    .await;
                json_output(result)
            }
            Err(err) => {
                session
                    .send_event(
                        turn.as_ref(),
                        EventMsg::WebFetchEnd(WebFetchEndEvent {
                            call_id,
                            url: String::new(),
                            final_url: String::new(),
                            status: "error".to_string(),
                            truncated: false,
                            content_chars: 0,
                        }),
                    )
                    .await;
                Err(to_model_error(err))
            }
        }
    }
}

fn runtime_config_for_search(
    config: Option<WebSearchConfig>,
) -> Result<WebSearchConfig, FunctionCallError> {
    let config = config.unwrap_or_default();
    if !config.client.enabled {
        return Err(to_model_error(WebToolError::Disabled {
            tool: "web_search",
        }));
    }
    Ok(config)
}

fn runtime_config_for_fetch(
    config: Option<WebSearchConfig>,
) -> Result<WebSearchConfig, FunctionCallError> {
    let config = config.unwrap_or_default();
    if !config.fetch.enabled {
        return Err(to_model_error(WebToolError::Disabled { tool: "web_fetch" }));
    }
    Ok(config)
}

fn json_output<T: Serialize>(value: T) -> Result<FunctionToolOutput, FunctionCallError> {
    serde_json::to_string(&value)
        .map(|json| FunctionToolOutput::from_text(json, Some(true)))
        .map_err(|err| {
            FunctionCallError::Fatal(format!("failed to serialize web tool output: {err}"))
        })
}

fn to_model_error(err: WebToolError) -> FunctionCallError {
    FunctionCallError::RespondToModel(err.to_string())
}

fn hash_for_log(value: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}
