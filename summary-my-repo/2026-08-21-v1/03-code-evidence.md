# 代码证据

## S01

- File: `third_party/codex-cli/codex-rs/core/src/config/mod.rs:1028-1033,2060-2064`
- Claim: 当前运行配置默认选择 DeepSeek provider，并为它选择 `deepseek-v4-flash`。

```rust
fn default_model_for_provider(model_provider_id: &str) -> Option<String> {
    match model_provider_id {
        DEEPSEEK_PROVIDER_ID => Some("deepseek-v4-flash".to_string()),
        _ => None,
    }
}

let model_provider_id = model_provider
    .or(config_profile.model_provider)
    .or(cfg.model_provider)
    .unwrap_or_else(|| DEEPSEEK_PROVIDER_ID.to_string());
```

- Interpretation: DeepSeek-first 不只是文档措辞，它进入了默认 provider/model 选择路径。

## S02

- File: `third_party/codex-cli/codex-rs/core/src/compact.rs:72-116,217-260`
- Claim: 上下文压缩会识别 DeepSeek provider，并切换到专有策略、提示词附录和 Flash 压缩模型。

```rust
pub(crate) enum CompactStrategy {
    OpenAiRemote,
    DeepSeek,
    LocalFallback,
}

pub(crate) fn compact_strategy(provider: &ModelProviderInfo) -> CompactStrategy {
    if provider.supports_remote_compaction() {
        CompactStrategy::OpenAiRemote
    } else if provider.is_deepseek() {
        CompactStrategy::DeepSeek
    } else {
        CompactStrategy::LocalFallback
    }
}

fn compact_prompt_for_strategy(base_prompt: &str, strategy: CompactStrategy) -> String {
    match strategy {
        CompactStrategy::DeepSeek => format!("{base_prompt}{WHALE_COMPACT_PROMPT_APPENDIX}"),
        CompactStrategy::OpenAiRemote | CompactStrategy::LocalFallback => base_prompt.to_string(),
    }
}
```

- Interpretation: 这验证了“针对 DeepSeek 特性优化，而非仅兼容一个 OpenAI 风格 endpoint”的方向。

## S03

- File: `third_party/codex-cli/codex-rs/protocol/src/taskspace.rs:4-57`
- Claim: TaskSpace 把工作图与工具结果定义为带版本、拒绝未知字段的 canonical artifact。

```rust
pub const TASKSPACE_CANONICAL_SCHEMA_VERSION: &str = "taskspace-canonical-map-v5";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceMapNode {
    pub node_id: TaskSpaceNodeId,
    pub goal: String,
    pub state: TaskSpaceNodeState,
    pub content: String,
    pub parents: Vec<TaskSpaceNodeId>,
    pub actions: Vec<TaskSpaceNodeAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskSpaceCanonicalMap {
    pub schema_version: String,
    pub map_id: TaskSpaceMapId,
    pub root: TaskSpaceMapNode,
    pub work_nodes: Vec<TaskSpaceMapNode>,
    pub finish: TaskSpaceMapNode,
    pub revision: TaskSpaceRevision,
}
```

- Interpretation: 用户目标、工作节点、依赖、状态和 action 不再只存在于模型上下文，而有稳定协议边界。

## S04

- File: `third_party/codex-cli/codex-rs/core/src/action_map/rooted_dag/invariants.rs:95-110,150-223`
- Claim: runtime 用确定性代码验证图身份、形状、环和 root/finish 可达性。

```rust
pub(crate) fn validate(map: &TaskSpaceMap) -> Vec<Violation> {
    let mut found = Violations::new();
    validate_identity(map, &mut found);
    let graph = validate_parents(map, &mut found);
    validate_shape(map, &graph, &mut found);
    validate_reachability(map, &graph, &mut found);
    validate_states(map, &mut found);
    validate_actions(map, &mut found);
    found
        .into_iter()
        .map(|(code, subjects)| Violation {
            code,
            subjects: subjects.into_iter().collect(),
        })
        .collect()
}

if is_cyclic_directed(&graph) {
    add_empty(found, ViolationCode::CycleDetected);
}
```

- Interpretation: 这对应最初的 DAG validation 和 gate-enforced 理念；模型选择任务含义，runtime 拒绝机械上不合法的状态。

## S05

- File: `third_party/codex-cli/codex-rs/core/src/session/taskspace_store/producer.rs:23-67,70-91`
- Claim: Session 结束前会关闭 TaskSpace action producer admission，等待已接纳 producer 排空，并记录结构化事件。

```rust
fn close_admission(&self) {
    let mut accepting = self
        .accepting
        .lock()
        .expect("TaskSpace Action producer gate poisoned");
    *accepting = false;
    self.tasks.close();
}

async fn wait(&self) {
    let producer_count = self.tasks.len();
    if producer_count > 0 {
        tracing::debug!(
            target: "codex_core::taskspace",
            event_name = "taskspace.action_producer_drain_started",
            producer_count,
            "waiting for TaskSpace Action producers"
        );
    }
    self.tasks.wait().await;
}
```

- Interpretation: 可回放状态依赖可靠的生命周期边界；该代码防止 session 收口与后台状态写入竞争。

## 追溯检查

- DeepSeek-first：`S01`、`S02`
- Artifact-first / replayable state：`S03`
- Gate-enforced / DAG validation：`S04`
- Session lifecycle / observability：`S05`
- Create/Debug、Viewer、参考驱动和 PrimitiveModule 的完整现状仍需模块级审计；本摘要只将其标为最初理念或规划，不将其误报为已完成。

