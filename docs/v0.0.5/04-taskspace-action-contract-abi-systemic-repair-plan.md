# TaskSpace Action Contract ABI 系统性修复计划

- Created: 2026-06-25
- Updated: 2026-06-25
- Version: 0.1
- Status: Draft
- Owner / Responsible: Unknown
- Related Systems: TaskSpace runtime, DeepSeek ChatCompletions transport, action_map, taskspace_control, benchmark instrumentation
- Related Links:
  - `coe/2026-06-22-15-24-taskspace-deepseek-cache-hit-rate.md`
  - `docs/v0.0.5/缓存命中问题修复/01-detailed-repair-plan.md`
  - `docs/v0.0.5/build-R2/02-phase-b-request-phase-attribution.md`
- Risk Level: High
- Plan Type: Full
- AI Agent 推理程度: high

## 1. 背景

TaskSpace 有两套已经落地的机制：

1. `taskspace_control` 原生工具。
   - 初始引入提交：`ce5ff70a43 feat(taskspace): add control tool for node growth`
   - 时间：2026-05-25
   - 目的：让模型通过工具维护 TaskSpace 的 task/map/node 生命周期。

2. `cache_optimized_action_contract` provider transport。
   - 关键修复提交：`b036d792f Fix DeepSeek TaskSpace cache transport`
   - 时间：2026-06-23
   - 目的：解决 DeepSeek 官方 ChatCompletions 路径中 TaskSpace 请求缓存命中率低的问题。
   - 机制：provider-native tools 不再进入 DeepSeek 热路径；模型只输出 `taskspace-action-v1` JSON，runtime 在本地解析并转成真实工具调用。

Phase B 真实 B-tier run 暴露了两套机制叠加后的协议漂移：

```text
run = target/phase-b-complete-B-rerun45/single-file-fast-fix/20260624-203208-963
taskspace_control_count = 0
state_commit_count = 0
stderr = failed to parse function arguments: missing field `action`
business_success = False
request_2_plus_hit_rate = 0.991148
native_tools_schema_hot_path_count = 0
tool_free_action_contract_count = 7
```

该 run 的缓存形态达标，但 action contract 没有可靠执行：模型输出了外层 `taskspace-action-v1`，但 `args` 内部使用 `action_name` 或 `command` 表示 `taskspace_control` 子动作，最终传到原生 handler 时缺少顶层 `action` 字段。

## 2. 问题定义

### 当前行为

`cache_optimized_action_contract` 要求模型输出：

```json
{
  "schema_version": "taskspace-action-v1",
  "action": "taskspace_control",
  "node_id": "node-1",
  "args": {
    "action": "finish_node"
  },
  "rationale": "..."
}
```

但失败样本中出现：

```json
{
  "schema_version": "taskspace-action-v1",
  "action": "taskspace_control",
  "args": {
    "action_name": "start_task"
  }
}
```

以及：

```json
{
  "schema_version": "taskspace-action-v1",
  "action": "taskspace_control",
  "args": {
    "command": "finish_node"
  }
}
```

`taskspace_control.rs` 的原生 handler 使用 `#[serde(tag = "action", rename_all = "snake_case")]` 解析 `TaskSpaceControlArgs`，因此传入参数必须有顶层 `action`。当前 translator 没有形成单一 ABI，也没有在进入 handler 前做完整规范化和稳定错误分类。

### 期望行为

DeepSeek TaskSpace cache-optimized transport 应同时满足：

1. provider-visible 请求保持 tool-free 和稳定 prefix，继续保证 DeepSeek cache hit。
2. `taskspace-action-v1` 是单一、类型化、可测试、可版本化的 ABI。
3. 所有 action contract 输出在进入任何工具 handler 前完成 parse、normalize、validate、compile。
4. handler 不再承担 action-contract 方言兼容责任。
5. malformed action 不应进入原生 handler 后才暴露 serde 错误。

### 差距

当前实现存在以下结构性缺口：

| Gap | 当前证据 | 影响 |
|---|---|---|
| 双层 `action` 语义混用 | 外层 `action=taskspace_control`，内层也要求 `args.action` | 模型容易输出 `action_name`、`command` 等变体 |
| ABI 定义不是单一来源 | prompt 文本、Rust parser、handler enum、测试各自维护 | schema 漂移无法在编译期或契约测试中发现 |
| compiler 层职责不足 | `normalize_taskspace_action_contract_control_args` 只覆盖 `control_action/control_type` | malformed payload 穿透到 handler |
| 错误没有稳定分类 | stderr 只显示 `missing field action` | benchmark 难以区分 parser、compiler、handler、agent 策略失败 |
| 缓存 gate 与 action 成功 gate 分离不足 | cache hit 达标但业务失败 | 高命中空转请求仍会消耗预算 |

## 3. 目标

| Goal | Expected Benefit | Baseline | Target | Measurement |
|---|---|---:|---:|---|
| 单一 Action ABI | 消除 prompt/parser/handler schema 漂移 | 当前存在双层 action 歧义 | 100% action contract schema 从同一 typed 定义生成或验证 | contract tests + schema snapshot |
| 编译器层拦截错误 | malformed action 不进入原生 handler | stderr `missing field action` | `taskspace_control_handler_parse_error_count = 0` for action-contract path | benchmark artifacts + runtime trace |
| 保持 DeepSeek cache 收益 | 不因修复可靠性而回退 native tools | request 2+ hit rate 0.991148 | `request_2_plus_hit_rate >= 0.95`, `native_tools_schema_hot_path_count = 0` | provider-cache-trace-summary.json |
| 提升有效执行率 | 减少高命中但无效的 follow-up 请求 | B rerun taskspace_control_count=0 | B-tier sample taskspace business_success=true, state_commit_count > 0 when workflow requires it | pair-report + taskspace-control-usage |
| 可诊断性 | 快速定位 action 失败阶段 | generic serde error | stable error code coverage >= 99% for rejected action outputs | action-compiler events |

## 4. 非目标

- 不回退到 DeepSeek provider-native tools 作为默认修复路径。
- 不把自然语言用户输入做本地固定回复或关键词式绕过模型。
- 不在 handler 内继续无边界追加 alias 作为主要修复方式。
- 不要求本计划同时解决所有 TaskSpace 业务策略问题，例如节点规划质量、最终答案质量、subagent 策略。
- 不更改 DeepSeek provider 的认证、计费或模型选择策略。

## 5. 复杂度与风险评估

| 项目 | 评估 |
|---|---|
| 工作类型 | bug fix + refactor + architecture change + performance protection |
| 风险等级 | High |
| 主要风险 | 破坏 DeepSeek 缓存命中、工具权限绕过、benchmark 误判、旧 action 合约兼容性下降 |
| 计划深度 | Full |
| 需要的核心证据 | contract fixture、unit/integration tests、B-tier rerun、cache trace、failure taxonomy |

## 6. 事实、约束与假设

### 已确认事实

| Fact | Evidence |
|---|---|
| `taskspace_control` handler 要求顶层 `action` | `taskspace_control.rs` 使用 `#[serde(tag = "action")]` |
| DeepSeek cache 修复依赖 tool-free action contract | `b036d792f`、缓存修复文档、COE H-006 |
| 失败 run 缓存命中达标但业务失败 | B rerun45 `request_2_plus_hit_rate=0.991148`, `business_success=False` |
| 失败 run 没有成功 taskspace_control | `taskspace_control_count=0`, `state_commit_count=0` |
| 失败 run 有 malformed control payload | stderr 两次 `missing field action` |

### 约束

| Constraint | Impact |
|---|---|
| DeepSeek TaskSpace 默认必须保持 `cache_optimized_action_contract` | 修复不能用 native tools fallback 作为默认路径 |
| TaskSpace 是本地 runtime 权限边界 | action compiler 必须走现有 ToolCall / handler 权限路径 |
| benchmark gate 必须可复现 | 需要 JSONL fixture 和 summary artifact |
| docs 默认中文 | 本计划和后续执行文档用中文维护 |

### 假设

| Assumption | Verification Method | If Assumption Fails |
|---|---|---|
| 当前主要失败来自 ABI 漂移，不是 DeepSeek cache regression | replay rerun45 JSONL action outputs through compiler tests | 若仍失败，进入 Phase 0 扩展 root-cause |
| 保持 tool-free action contract 可继续满足 cache target | rerun provider cache trace | 若 hit rate 下降，停止默认切换并比较 prompt diff |
| typed action compiler 可以覆盖现有 action contract 生产路径 | code inventory + integration tests | 若路径分裂，先引入 adapter 和 deprecation gates |

## 7. 依赖

| Dependency | Type | Current Status | Blocking Risk | Handling Plan |
|---|---|---|---|---|
| DeepSeek official ChatCompletions cache behavior | third-party | Ready but external | provider cache behavior变化会影响收益验证 | 使用 live cache trace，不把离线测试当最终收益证据 |
| TaskSpace benchmark harness | system | Ready | gate 不覆盖 compiler error 会误判 | Phase 4 扩展 artifacts 和 release decision |
| Existing `taskspace_control` handler | system | Ready | handler enum 与 compiler enum 双维护 | Phase 1 建立单一 schema source 或生成一致性测试 |
| Phase B request attribution | system | Ready | 新 phase producer 需要真实 state_commit 才能覆盖 | Phase 5 B-tier rerun 同时看 phase_counts 和 state_commit_count |
| Adversarial review | person/process | Unknown | 架构迁移遗漏边界 | Phase 2/5 设置审查 gate |

## 8. 总体技术设计

### 8.1 目标架构

```text
Provider output text
  -> TaskSpaceActionParser
  -> TaskSpaceActionNormalizer
  -> TaskSpaceActionValidator
  -> TaskSpaceActionCompiler
  -> ToolCall
  -> existing ToolRuntime / handlers
  -> runtime trace + benchmark artifacts
```

### 8.2 单一 ABI

建立一个明确的 Rust 类型边界：

```rust
enum TaskSpaceActionV1 {
    ListFiles(ListFilesArgs),
    Search(SearchArgs),
    ReadFile(ReadFileArgs),
    ApplyPatch(ApplyPatchArgs),
    RunTest(RunTestArgs),
    Control(TaskSpaceControlActionV1),
    FinalAnswer(FinalAnswerArgs),
    Blocked(BlockedArgs),
}

enum TaskSpaceControlActionV1 {
    StartTask(StartTaskArgs),
    FinishNode(FinishNodeArgs),
    CreateNode(CreateNodeArgs),
    BindNode(BindNodeArgs),
    StateCommit(StateCommitArgs),
    MarkResultValidity(MarkResultValidityArgs),
    AdoptResult(AdoptResultArgs),
    ...
}
```

`serde_json::Value args` 只能保留在 parser 输入边界，不能作为内部 compiler 的长期数据结构。

### 8.3 方言兼容策略

兼容层只存在于 `TaskSpaceActionNormalizer`，并且必须：

- 记录 `normalized_aliases`。
- 给每个兼容项稳定错误码和测试 fixture。
- 只在进入 typed enum 前运行。
- 有去除计划，不能散落在 handler。

示例兼容映射：

| Legacy input | Canonical |
|---|---|
| `args.action_name` | `args.action` |
| `args.command` when outer action is `taskspace_control` | `args.action` |
| `args.control_action` | `args.action` |
| `args.control_type` | `args.action` |
| `args.first_node_kind` | `args.node_kind` for `start_task` |

### 8.4 错误模型

所有拒绝必须在 compiler 层归类：

| Error Code | Meaning | Example |
|---|---|---|
| `E_ACTION_JSON_NOT_STRICT` | provider 输出不是严格 JSON | fenced JSON |
| `E_ACTION_SCHEMA_VERSION` | schema_version 不支持 | v2/v0 |
| `E_ACTION_UNKNOWN_OUTER` | 外层 action 不支持 | `edit_file` |
| `E_CONTROL_ACTION_MISSING` | taskspace_control 子动作缺失 | `args={}` |
| `E_CONTROL_ACTION_UNKNOWN` | 子动作未知 | `args.action=foo` |
| `E_NODE_POLICY_VIOLATION` | 当前 node kind 不允许该动作 | inspect 上 apply_patch |
| `E_BUDGET_TRANSITION_REQUIRED` | budget gate 要求 control only | pressure 下 read_file |
| `E_COMPILE_ARGUMENT_INVALID` | typed args 缺字段或类型不匹配 | start_task 缺 title 且无法默认 |

### 8.5 缓存稳定性设计

修复 action ABI 时必须保持：

- provider-native tools list 为空。
- stable action contract 文本稳定。
- 动态 state 只进入 bounded suffix。
- compiler 兼容表不泄漏大量动态内容到 provider prefix。
- 错误恢复消息短、结构稳定、带 error code。

## 9. 阶段计划

### Phase 0: 证据基线与失败重放

#### Objective

把当前失败固定成可重放 regression，不再依赖 live benchmark 才暴露问题。

#### Entry Criteria

- 当前分支干净。
- rerun45 artifacts 可读。
- `b036d792f` COE 和 Phase B 文档可引用。

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| 失败样本存在 | `Test-Path target/.../whale-exec.jsonl` | path exists | Unknown |
| 当前测试基线可跑 | `cargo test -p codex-core taskspace_action_contract --lib` | pass/fail baseline | Unknown |
| 缓存基线已记录 | 读取 `provider-cache-trace-summary.json` | hit rate / shape counts | Unknown |

#### Design Approach

把失败 run 中的 action outputs 抽成 fixture，覆盖：

- `args.action_name=start_task`
- `args.command=finish_node`
- fenced JSON / prose prefix / DSML suffix 已有兼容样本
- malformed control args 的错误码

#### Implementation Tasks

1. 新增 action-contract replay fixture。
2. 新增 parser/normalizer/compiler baseline tests。
3. 新增 benchmark artifact parser，统计 action contract compile error。
4. 记录当前行为：哪些 fixture 失败，失败阶段在哪里。

#### Deliverables

- `taskspace_action_contract_replay_rerun45_*` tests。
- `action-contract-compile-errors.jsonl` fixture schema 草案。
- Phase 0 证据摘要。

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| Replay fixture | `session/turn.rs` tests or dedicated module | cargo test | rerun45 malformed samples fail with expected codes | none | fixture-only, blocks completion | planned |
| Compile error artifact schema | benchmark scripts | benchmark artifact generation | script selftest | JSON artifact path | none | planned |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | replay current failure | unit test | failure classified before handler |
| Benefit | diagnostic repeatability | fixture run | no live provider required to reproduce |

#### Exit Criteria

- Current malformed action no longer appears only as generic serde error in tests.
- Baseline documents exact failing stage.

#### Review Plan

- Code owner review for fixture fidelity.
- Optional adversarial review before implementation phases.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Fixture does not match live output | false confidence | live rerun fails differently | include raw JSONL snippets and parser roundtrip | add second live sample |

#### Gate To Next Phase

Phase 1 starts only after current failure can be reproduced without provider calls.

### Phase 1: 单一 ABI 与 Schema Source

#### Objective

建立 typed `TaskSpaceActionV1` / `TaskSpaceControlActionV1`，让 prompt、compiler、handler 共享同一契约。

#### Entry Criteria

- Phase 0 replay fixture landed.
- 当前 handler enum inventory 完成。

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| handler variants 完整列出 | `rg "enum TaskSpaceControlArgs"` | inventory doc | Unknown |
| action contract tests baseline | cargo test | known failures/pass | Unknown |

#### Design Approach

优先做内部 typed ABI，不急于删除原 handler。先让 action contract compiler 输出 canonical handler JSON，然后再决定是否把 handler 迁移到共享类型。

#### Implementation Tasks

1. 新建 `taskspace_action_contract` 模块，避免继续扩大 `session/turn.rs`。
2. 定义 typed outer action enum。
3. 定义 typed control action enum。
4. 编写 schema snapshot 生成或一致性测试。
5. 更新 static action contract prompt，让 provider 看到的 schema 与 typed ABI 一致。

#### Deliverables

- typed ABI module。
- schema snapshot 或 golden prompt。
- handler compatibility mapping table。

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| Typed outer action | `core/src/session/taskspace_action_contract.rs` | DeepSeek action transport | unit tests | none | none | planned |
| Typed control action | same module or shared action_map module | taskspace_control compile path | unit tests | none | none | planned |
| Prompt schema from ABI | prompt construction path | provider request build | snapshot test | provider payload scan | none | planned |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | schema consistency | golden snapshot | prompt schema and typed ABI do not drift |
| Correctness | handler coverage | unit test | every handler action has compiler mapping or explicit non-support reason |

#### Exit Criteria

- No new action contract behavior depends on raw `serde_json::Value` beyond parser boundary.
- All existing allowed actions compile through typed ABI.

#### Review Plan

- Architecture review focused on ABI boundaries.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Scope too large | delayed repair | many handler variants block typed migration | compile canonical JSON first, migrate handler later | keep handler enum, but typed compiler is mandatory |

#### Gate To Next Phase

Phase 2 starts only after typed ABI can represent the current action set.

### Phase 2: Action Compiler 与稳定错误模型

#### Objective

所有 provider action 输出必须在进入 ToolRuntime 前完成 parse、normalize、validate、compile，并产生稳定错误码。

#### Entry Criteria

- Phase 1 ABI landed.
- replay fixture 可覆盖 current failure。

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| typed ABI tests pass | cargo test | pass | Unknown |
| malformed samples present | fixture list | includes rerun45 | Unknown |

#### Design Approach

实现 `TaskSpaceActionCompiler`：

```text
parse raw text
normalize dialects
decode typed action
validate node/budget/security policy
compile to ToolCall or terminal action
emit stable event
```

#### Implementation Tasks

1. 把 `parse_taskspace_action_v1`、`taskspace_action_to_tool_call` 从 `turn.rs` 移到专用模块。
2. 实现 `TaskSpaceActionCompileError`。
3. 实现 alias normalizer，但集中在 compiler。
4. 对 `action_name/command` 做 canonical normalization 或稳定拒绝。
5. 保证拒绝不会调用 handler。
6. 给每次 compile 结果记录 `taskspace_action_contract_compile` trace。

#### Deliverables

- `TaskSpaceActionCompiler`。
- 稳定错误码枚举。
- compile trace event。
- malformed input regression tests。

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| Compiler | action contract transport path | `run_sampling_request` | unit + integration | compile trace | none | planned |
| Error model | compiler module | rejected action follow-up | unit tests | error code artifacts | none | planned |
| Alias normalizer | compiler module | action parse path | rerun45 fixture tests | normalized_aliases trace | none | planned |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | malformed control args | unit test | returns `E_CONTROL_ACTION_MISSING` or canonical control action, no handler serde error |
| Correctness | policy violation | unit test | compile rejects before tool call |
| Correctness | valid actions | integration test | compile emits expected ToolCall |
| Benefit | diagnostic quality | artifact selftest | rejected actions all have stable error codes |

#### Exit Criteria

- `missing field action` cannot originate from action-contract path for known malformed samples.
- Compile success/failure event coverage >= 99% in benchmark artifacts.

#### Review Plan

- Code review for permission boundaries and no native tools fallback.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Over-normalization executes unintended action | safety risk | ambiguous alias accepted | strict ambiguity rejection | require explicit canonical action |
| Error messages become dynamic and hurt cache | cache regression | hit rate drops | stable short error code text | shrink recovery message |

#### Gate To Next Phase

Phase 3 starts only after compiler rejects or canonicalizes rerun45 malformed outputs before handler.

### Phase 3: Handler 边界收敛与旧兼容治理

#### Objective

清理 handler 与 action contract 之间的职责边界，避免继续在多个层面添加 alias。

#### Entry Criteria

- compiler path production wired。
- compile trace artifacts available。

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| action contract path uses compiler | code inspection + test | production path evidence | Unknown |
| handler parse errors separated | benchmark artifact | error source field | Unknown |

#### Design Approach

handler 保持原生工具入口，但 action-contract path 不直接传 raw args。所有兼容逻辑移动到 compiler。handler 的 `normalize_taskspace_arguments` 只服务 native tool/manual tool path。

#### Implementation Tasks

1. 给 compiled `taskspace_control` 输出 canonical handler JSON。
2. 将 handler alias 表标记为 native-tool compatibility，不作为 action-contract contract。
3. 增加 action-contract path 禁止 raw passthrough 的测试。
4. 文档化 legacy alias 去除策略。

#### Deliverables

- canonical control JSON compiler。
- handler boundary tests。
- compatibility/deprecation note。

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| Canonical handler JSON | compiler -> ToolCall | taskspace_control ToolCall | integration test | action_compile trace | none | planned |
| Handler boundary | taskspace_control handler | native tool path | regression test | parse error source | none | planned |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | no raw passthrough | unit test | malformed args rejected/normalized before handler |
| Compatibility | native tool path | handler tests | existing taskspace_control tests pass |

#### Exit Criteria

- action-contract path has no direct raw `args` passthrough to handler.
- handler remains backward-compatible for native tool path.

#### Review Plan

- Review action-contract and native-tool compatibility separately.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Native path regresses | manual/debug flows break | handler tests fail | keep native handler aliases | feature flag action-contract compiler only |

#### Gate To Next Phase

Phase 4 starts after handler boundary tests pass.

### Phase 4: Benchmark Gates 与观测建设

#### Objective

让 benchmark 能同时判断缓存收益和 action 执行可靠性，避免高命中空转被误判为成功。

#### Entry Criteria

- compiler emits structured trace。
- benchmark scripts currently pass。

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| trace event emitted | unit/integration test | event sample | Unknown |
| cost instrumentation baseline | `test-cost-instrumentation.ps1` | pass | Unknown |

#### Design Approach

新增 action-contract 维度 artifacts：

```text
action-contract-events.jsonl
action-contract-summary.json
```

建议字段：

```json
{
  "schema_version": "taskspace-action-contract-summary-v1",
  "compile_event_count": 8,
  "compile_success_count": 7,
  "compile_failure_count": 1,
  "compile_success_rate": 0.875,
  "handler_parse_error_count": 0,
  "stable_error_code_coverage": 1.0,
  "normalized_alias_counts": {
    "action_name_to_action": 1
  }
}
```

#### Implementation Tasks

1. 扩展 `cost-instrumentation.ps1` 提取 compile events。
2. 扩展 `test-cost-instrumentation.ps1` fixture。
3. 扩展 release decision gate。
4. 在 B-tier report 中显示 action contract success/failure。

#### Deliverables

- action-contract summary artifacts。
- PowerShell selftests。
- release decision gate。

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| Artifact extractor | benchmark scripts | run-taskspace-benchmark | PS selftest | JSON/JSONL artifacts | none | planned |
| Release gate | write-release-decision.ps1 | release decision | PS selftest | decision report | none | planned |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | artifact extraction | PS selftest | counts and error codes correct |
| Benefit | action reliability visibility | B-tier artifact | action compile success visible and gateable |

#### Exit Criteria

- release decision fails if action-contract path has handler parse errors.
- cache gate remains independent but not sufficient for business success.

#### Review Plan

- Review gate thresholds with benchmark maintainers.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| Gate too strict early | blocks diagnosis | many old fixtures fail | staged warn-only then fail | use `diagnostic_only` gate for one phase |

#### Gate To Next Phase

Phase 5 starts only after artifacts distinguish cache pass from action failure.

### Phase 5: End-to-End 验证与收益证明

#### Objective

证明系统性修复同时保持 DeepSeek cache 收益并恢复 TaskSpace 有效执行。

#### Entry Criteria

- Phase 0-4 landed。
- Debug build passes。

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| Rust tests pass | cargo test | pass | Unknown |
| script tests pass | PowerShell selftests | pass | Unknown |
| whale builds | cargo build | binary path | Unknown |

#### Design Approach

先跑低成本 deterministic fixture，再跑 B-tier live sample。不要直接扩大到 E3。

#### Implementation Tasks

1. Run focused compiler tests。
2. Run benchmark selftests。
3. Build `whale`。
4. Run `single-file-fast-fix` B-tier。
5. Compare against rerun45 and Phase A/B previous pass/fail evidence。

#### Deliverables

- test log summary。
- B-tier rerun artifacts。
- cache/action/business gate comparison。

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| E2E action execution | CLI action-contract transport | B-tier run | validation exit codes | action-contract-summary + pair-report | none | planned |
| Cache preservation | DeepSeek provider path | B-tier run | provider cache gate | provider-cache-trace-summary | none | planned |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | compiler unit tests | cargo test | all pass |
| Correctness | benchmark scripts | PS tests | all pass |
| Correctness | B-tier business | run benchmark | `business_success=True`, public/hidden validation 0 |
| Benefit | cache | provider trace | request 2+ hit rate >= 0.95, native tools hot path 0 |
| Benefit | action reliability | action summary | handler parse errors 0, compile success rate >= 0.99 or failures terminal/blocked with stable code |

#### Exit Criteria

- No `missing field action` in stderr for action-contract path。
- `taskspace_control_count > 0` when workflow requires lifecycle transition。
- `state_commit_count > 0` for stateful workflow or documented reason why not needed。
- B-tier business pass。
- Cache gate still passes。

#### Review Plan

- Ask for adversarial review before declaring release-ready.

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| B-tier remains business-failing for unrelated agent quality | ambiguous closure | action gates pass but patch wrong | separate action reliability from agent quality taxonomy | run second focused scenario |
| Live provider variance | flaky cache evidence | hit rate unstable | repeat small sample | preserve old default until stable |

#### Gate To Next Phase

Phase 6 starts after B-tier demonstrates both action reliability and cache preservation.

### Phase 6: Rollout、默认路径与清理

#### Objective

把新 compiler 作为 DeepSeek TaskSpace 默认路径，清理过期兼容代码，并保留安全回退。

#### Entry Criteria

- Phase 5 validation pass。
- No P0/P1 unresolved review findings。

#### Entry Criteria Checks

| Entry Criterion | Check Method | Evidence / Output | Owner |
|---|---|---|---|
| gates pass | release decision | pass report | Unknown |
| review complete | review doc | no blocking findings | Unknown |

#### Design Approach

默认继续使用 `cache_optimized_action_contract`，但 compiler 支持 feature/config rollback 到旧 action-contract parser，而不是回退 provider-native tools。

#### Implementation Tasks

1. Update docs and runbooks。
2. Add config fallback if needed: `WHALE_TASKSPACE_ACTION_COMPILER=legacy|v1`。
3. Mark old alias locations deprecated。
4. Remove or quarantine duplicate tests。
5. Archive rerun evidence。

#### Deliverables

- rollout note。
- fallback instructions。
- updated v0.0.5 status doc。

#### Implementation Completeness Evidence

| Plan Item | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|
| Default compiler path | session turn sampling | DeepSeek TaskSpace CLI | build + B-tier | action summary | none | planned |
| Fallback | config/env gate | CLI runtime | config tests | selected compiler trace | none | planned |

#### Testing And Validation

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | default path | smoke run | compiler v1 selected |
| Correctness | fallback | config test | legacy parser selectable for diagnosis |
| Benefit | post-cleanup cache | cache trace | no regression from Phase 5 |

#### Exit Criteria

- Default path uses compiler v1。
- Fallback documented and tested。
- Old compatibility code either removed or explicitly scoped。

#### Review Plan

- Final code review and optional adversarial review。

#### Risks And Fallback

| Risk | Impact | Trigger Signal | Mitigation | Fallback |
|---|---|---|---|---|
| cleanup removes needed compatibility | regression | old fixture fails | staged deprecation | keep compatibility but move to compiler |

#### Gate To Next Phase

Plan can close after docs and evidence are committed.

## 10. 总体实现完整性矩阵

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| Typed Action ABI | 所有 action contract 有单一类型定义 | new action contract module | DeepSeek action transport | cargo unit tests | none | none | planned |
| Action Compiler | provider 输出编译成 ToolCall | compiler module + `run_sampling_request` | model response processing | unit + integration | compile trace | none | planned |
| Stable Error Codes | malformed action 被稳定分类 | compiler error enum | rejected action follow-up | fixture tests | action-contract-summary | none | planned |
| Handler Boundary | handler 不接收 raw action-contract args | compiler -> ToolCall | taskspace_control | regression tests | parse error source | none | planned |
| Benchmark Gates | cache/action/business 分层判定 | PS scripts | benchmark run | PS selftests | summary artifacts | fixture-based selftest only | planned |
| B-tier Validation | business + cache + action reliability 全部可证 | CLI runtime | run-taskspace-benchmark | B-tier run | pair-report/cache/action summaries | none | planned |

## 11. 测试与验证策略

| Validation Type | Test Type | Scope | Execution Method | Passing Standard |
|---|---|---|---|---|
| Correctness | Unit | parser/normalizer/compiler | `cargo test -p codex-core taskspace_action_contract --lib` | targeted tests pass |
| Correctness | Integration | compiler -> ToolCall -> handler | cargo integration/unit tests | valid actions execute expected tool call |
| Correctness | Regression | rerun45 malformed outputs | fixture replay | no generic handler serde error |
| Correctness | Script | benchmark artifacts | `test-cost-instrumentation.ps1`, `test-release-decision.ps1` | selftests pass |
| Benefit | Cache | DeepSeek B-tier provider trace | benchmark rerun | request 2+ hit rate >= 0.95, native hot path 0 |
| Benefit | Reliability | action contract summary | benchmark rerun | handler parse error 0, compile success/failure classified |
| Benefit | Business | focused B-tier | benchmark rerun | business_success true, public/hidden validation 0 |

## 12. 收益验证

| Benefit Hypothesis | Metric | Baseline | Target | Measurement Method | Data Source | Observation Window | Pass / Fail Threshold |
|---|---|---:|---:|---|---|---|---|
| 保持 DeepSeek cache 修复收益 | request 2+ hit rate | 0.991148 in rerun45 | >= 0.95 | provider cache trace | `provider-cache-trace-summary.json` | focused B-tier run | fail if < 0.95 |
| 移除 native tools schema 热路径 | native tools hot path count | 0 in rerun45 | 0 | request shape classifier | cache trace summary | focused B-tier run | fail if > 0 |
| action contract 执行可靠 | handler parse error count | generic `missing field action` observed | 0 | compiler/handler error artifact | action-contract-summary | focused B-tier run | fail if > 0 |
| 任务有效执行恢复 | business success | false in rerun45 | true | benchmark pair report | pair-report.md | focused B-tier run | fail if false |
| 诊断质量提升 | stable error code coverage | unavailable | >= 99% | compile event aggregation | action-contract-summary | focused B-tier run | fail if < 99% |

## 13. 发布、回滚与降级

### Release Strategy

- Release method: land behind default-on compiler path for DeepSeek TaskSpace after tests pass.
- Canary scope: `single-file-fast-fix` B-tier first; do not jump directly to E3.
- Expansion criteria: B-tier business/cache/action gates pass.
- Pause criteria: cache hit < 0.95, native tools hot path > 0, handler parse error > 0, or new permission boundary issue.
- Owner: Unknown.
- Release window: Unknown.

### Rollback Strategy

- Rollbackable changes:
  - compiler selection env/config。
  - benchmark gate thresholds。
  - prompt schema text。
- Non-directly rollbackable changes:
  - removing old compatibility code before a stable window。
- Rollback triggers:
  - DeepSeek cache regression。
  - action execution regression。
  - security/permission review finding。
- Rollback steps:
  - Switch compiler config to legacy parser if implemented。
  - If cache is impacted, do not switch to native tools by default; first revert prompt/compiler change。
  - Re-run focused B-tier and cache verification。
- Rollback validation:
  - `provider-cache-trace-summary.json` gate pass。
  - `action-contract-summary.json` no handler parse errors。

### Fallback / Degradation Strategy

- Degradable capability: action-contract compiler v1 strictness。
- Trigger: strict compiler rejects too many otherwise safe actions。
- User-visible impact: more blocked/follow-up turns, but no permission bypass。
- System behavior while degraded: stable error code returned to model with bounded recovery prompt。
- Recovery steps: add typed normalization fixture, not handler alias patch。

## 14. 观测与成功指标

| Metric | Current Baseline | Target | Alert Threshold | Observation Window |
|---|---:|---:|---:|---|
| request_2_plus_hit_rate | 0.991148 | >= 0.95 | < 0.95 | focused B-tier |
| native_tools_schema_hot_path_count | 0 | 0 | > 0 | focused B-tier |
| tool_free_action_contract_count | 7 | > 0 | = 0 | focused B-tier |
| action_compile_success_rate | Unknown | >= 0.99 or stable terminal failures | < 0.99 with unclassified failure | focused B-tier |
| handler_parse_error_count | generic missing field observed | 0 | > 0 | focused B-tier |
| taskspace_control_count | 0 | > 0 when lifecycle transition required | = 0 for stateful workflow | focused B-tier |
| business_success | false | true | false | focused B-tier |

## 15. 替代方案与取舍

| Alternative | Pros | Cons | Decision |
|---|---|---|---|
| 在 handler 里继续补 alias | 快 | schema 漂移继续扩大，错误仍晚暴露 | Reject |
| 回退 native tools | action 执行更接近原 Codex | 破坏 DeepSeek cache 修复目标 | Reject as default |
| prompt 里加强说明 | 低成本 | 无法防止模型输出变体，缺少工程保障 | Insufficient |
| typed compiler + single ABI | 系统性解决 drift 和诊断 | 实现成本较高 | Choose |
| 完全移除 legacy handler | 边界清晰 | 风险高，调试/native path 受影响 | Defer |

## 16. 安全与权限边界

- compiler 只能生成现有 ToolCall，不能直接执行文件或 shell。
- action policy 必须在 ToolRuntime/handler 权限之前再做一层本地校验。
- `apply_patch` 必须继续走现有 patch 工具路径。
- `run_test` 必须继续走 shell_command 权限路径。
- malformed action 的 recovery prompt 不得包含敏感 payload。
- action-contract artifacts 不应记录 secrets、完整 provider payload 或直接个人信息。

## 17. 开放问题

| Question | Why It Matters | Proposed Resolution |
|---|---|---|
| 是否允许 `args.command` 作为长期兼容别名？ | 可能过度兼容错误心理模型 | Phase 0 统计出现频率，Phase 2 只在明确映射时 canonicalize |
| typed ABI 是否应从 handler enum 生成？ | 决定长期维护成本 | Phase 1 做 inventory 后决定 |
| compile success target 是否必须 99%？ | blocked/final action 可能不是工具调用 | 用 classified outcome coverage + handler parse error 0 组合 |
| 是否需要保留 legacy parser config？ | 回滚策略 | Phase 6 决定，默认不回退 native tools |

## 18. Change Log

| Version | Date | Change |
|---|---|---|
| 0.1 | 2026-06-25 | Initial systemic repair plan for TaskSpace action contract ABI |

## 19. Plan Quality Checklist

- [x] 问题定义区分 current / expected / gap。
- [x] 目标包含 correctness 与 benefit。
- [x] 明确非目标，禁止 native tools 默认回退。
- [x] 分离事实、假设、约束、风险。
- [x] 阶段包含 entry、tasks、deliverables、validation、exit、review、fallback。
- [x] 包含 plan-to-code completeness evidence。
- [x] 包含 cache benefit validation。
- [x] 包含 release / rollback / fallback。
- [x] 包含 observability metrics。
- [x] 不把补 alias 作为系统性修复。
