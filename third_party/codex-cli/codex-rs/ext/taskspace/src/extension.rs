use std::sync::Arc;

use codex_extension_api::ContextContributor;
use codex_extension_api::ExtensionData;
use codex_extension_api::ExtensionFuture;
use codex_extension_api::ExtensionRegistryBuilder;
use codex_extension_api::ThreadLifecycleContributor;
use codex_extension_api::ThreadResumeInput;
use codex_extension_api::ThreadStartInput;
use codex_extension_api::ToolBatchPreflightContributor;
use codex_extension_api::ToolBatchPreflightFailure;
use codex_extension_api::ToolBatchPreflightInput;
use codex_extension_api::ToolCallOutcome;
use codex_extension_api::ToolContributor;
use codex_extension_api::ToolFinishInput;
use codex_extension_api::ToolLifecycleContributor;
use codex_extension_api::ToolLifecycleFuture;
use codex_extension_api::TurnLifecycleContributor;
use codex_extension_api::TurnStartInput;
use codex_extension_api::WorldStateContributionInput;
use codex_extension_api::WorldStateSectionContribution;
use codex_protocol::ThreadId;
use codex_tools::ToolCall;
use codex_tools::ToolExecutor;

use crate::runtime::TaskSpaceRuntimeHandle;
use crate::service::TaskSpaceService;
use crate::tool::ReadTaskSpaceTool;
use crate::world_state;

#[derive(Clone)]
struct TaskSpaceExtension {
    store: Arc<dyn crate::runtime::TaskSpaceStore>,
    service: Arc<TaskSpaceService>,
    event_emitter: crate::event_emitter::TaskSpaceEventEmitter,
}

impl<C> ThreadLifecycleContributor<C> for TaskSpaceExtension
where
    C: Send + Sync + 'static,
{
    fn on_thread_start<'a>(&'a self, input: ThreadStartInput<'a, C>) -> ExtensionFuture<'a, ()> {
        Box::pin(async move {
            let Ok(thread_id) = ThreadId::from_string(input.thread_store.level_id()) else {
                return;
            };
            let runtime = input.thread_store.get_or_init(|| {
                TaskSpaceRuntimeHandle::new(
                    thread_id,
                    Arc::clone(&self.store),
                    self.event_emitter.clone(),
                )
            });
            self.service.register(&runtime);
            let result = if let Some(parent_thread_id) = input.session_source.parent_thread_id() {
                runtime
                    .inherit(parent_thread_id, crate::TaskSpaceMapRelation::Child)
                    .await
                    .map(|_| ())
            } else if let Some(source_thread_id) = input.forked_from_thread_id {
                runtime
                    .inherit(source_thread_id, crate::TaskSpaceMapRelation::Fork)
                    .await
                    .map(|_| ())
            } else {
                runtime.refresh().await.map(|_| ())
            };
            if let Err(error) = result {
                tracing::warn!(%thread_id, %error, "failed to restore TaskSpace runtime");
            }
        })
    }

    fn on_thread_resume<'a>(&'a self, input: ThreadResumeInput<'a>) -> ExtensionFuture<'a, ()> {
        Box::pin(async move {
            let Some(runtime) = runtime(input.thread_store) else {
                return;
            };
            if let Err(error) = runtime.refresh().await {
                tracing::warn!(thread_id = %runtime.thread_id(), %error, "failed to resume TaskSpace runtime");
            }
        })
    }
}

impl TurnLifecycleContributor for TaskSpaceExtension {
    fn on_turn_start<'a>(&'a self, input: TurnStartInput<'a>) -> ExtensionFuture<'a, ()> {
        Box::pin(async move {
            let Some(runtime) = runtime(input.thread_store) else {
                return;
            };
            if let Err(error) = runtime.refresh().await {
                tracing::warn!(thread_id = %runtime.thread_id(), %error, "failed to refresh TaskSpace turn state");
            }
        })
    }
}

impl ContextContributor for TaskSpaceExtension {
    fn contribute_world_state<'a>(
        &'a self,
        input: WorldStateContributionInput<'a>,
    ) -> ExtensionFuture<'a, Vec<WorldStateSectionContribution>> {
        Box::pin(async move {
            let Some(runtime) = runtime(input.thread_store) else {
                return Vec::new();
            };
            if !runtime.is_enabled() {
                return Vec::new();
            }
            runtime
                .record()
                .await
                .map(world_state::section)
                .into_iter()
                .collect()
        })
    }
}

impl ToolContributor for TaskSpaceExtension {
    fn tools(
        &self,
        _session_store: &ExtensionData,
        thread_store: &ExtensionData,
    ) -> Vec<Arc<dyn ToolExecutor<ToolCall>>> {
        runtime(thread_store)
            .filter(|runtime| runtime.is_enabled())
            .map(|runtime| Arc::new(ReadTaskSpaceTool::new(runtime)) as Arc<_>)
            .into_iter()
            .collect()
    }
}

impl ToolBatchPreflightContributor for TaskSpaceExtension {
    fn preflight<'a>(
        &'a self,
        input: ToolBatchPreflightInput<'a>,
    ) -> ExtensionFuture<'a, Result<(), ToolBatchPreflightFailure>> {
        Box::pin(async move { crate::preflight::validate(&input).await })
    }
}

impl ToolLifecycleContributor for TaskSpaceExtension {
    fn on_tool_finish<'a>(&'a self, input: ToolFinishInput<'a>) -> ToolLifecycleFuture<'a> {
        Box::pin(async move {
            let Some(runtime) = runtime(input.thread_store) else {
                return;
            };
            let success = matches!(input.outcome, ToolCallOutcome::Completed { success: true });
            if let Err(error) = runtime
                .release_prepared(input.call_id, input.turn_id, success)
                .await
            {
                tracing::warn!(
                    thread_id = %runtime.thread_id(),
                    call_id = input.call_id,
                    %error,
                    "failed to release TaskSpace action reservation"
                );
            }
        })
    }
}

fn runtime(thread_store: &ExtensionData) -> Option<Arc<TaskSpaceRuntimeHandle>> {
    thread_store.get::<TaskSpaceRuntimeHandle>()
}

pub fn install<C>(
    registry: &mut ExtensionRegistryBuilder<C>,
    store: Arc<dyn crate::runtime::TaskSpaceStore>,
) -> Arc<TaskSpaceService>
where
    C: Send + Sync + 'static,
{
    let service = Arc::new(TaskSpaceService::default());
    install_with_service(registry, store, Arc::clone(&service));
    service
}

pub fn install_with_service<C>(
    registry: &mut ExtensionRegistryBuilder<C>,
    store: Arc<dyn crate::runtime::TaskSpaceStore>,
    service: Arc<TaskSpaceService>,
) where
    C: Send + Sync + 'static,
{
    let extension = Arc::new(TaskSpaceExtension {
        store,
        service,
        event_emitter: crate::event_emitter::TaskSpaceEventEmitter::new(registry.event_sink()),
    });
    registry.thread_lifecycle_contributor(extension.clone());
    registry.turn_lifecycle_contributor(extension.clone());
    registry.prompt_contributor(extension.clone());
    registry.tool_contributor(extension.clone());
    registry.tool_batch_preflight_contributor(extension.clone());
    registry.tool_lifecycle_contributor(extension);
}
