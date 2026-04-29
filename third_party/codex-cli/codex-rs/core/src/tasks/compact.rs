use std::sync::Arc;

use super::SessionTask;
use super::SessionTaskContext;
use crate::compact::CompactStrategy;
use crate::session::turn_context::TurnContext;
use crate::state::TaskKind;
use codex_protocol::user_input::UserInput;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Default)]
pub(crate) struct CompactTask;

impl SessionTask for CompactTask {
    fn kind(&self) -> TaskKind {
        TaskKind::Compact
    }

    fn span_name(&self) -> &'static str {
        "session_task.compact"
    }

    async fn run(
        self: Arc<Self>,
        session: Arc<SessionTaskContext>,
        ctx: Arc<TurnContext>,
        input: Vec<UserInput>,
        _cancellation_token: CancellationToken,
    ) -> Option<String> {
        let session = session.clone_session();
        let strategy = crate::compact::compact_strategy(ctx.provider.info());
        session.services.session_telemetry.counter(
            "codex.task.compact",
            /*inc*/ 1,
            &[("type", strategy.telemetry_type())],
        );
        let _ = if matches!(strategy, CompactStrategy::OpenAiRemote) {
            crate::compact_remote::run_remote_compact_task(session.clone(), ctx).await
        } else {
            crate::compact::run_compact_task(session.clone(), ctx, input).await
        };
        None
    }
}
