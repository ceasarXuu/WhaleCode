# R4 静态 tools 链路图和分支推理

> 本文记录 TaskSpace tools 调用信息从 agent intent 到 provider 下一轮 payload 的静态路径。
> 目标是先证明“信息怎么走”，再决定“哪里需要修”。R4 不能只修当前暴露的样本路径。

## 1.1 目标链路

R4 期望的最佳路径如下：

```text
Agent tool intent
  -> standard tool execution / standard validation
  -> canonical ToolFeedbackEnvelope
  -> ResponseInputItem / provider-visible item
  -> conversation history append
  -> provider projection decision with explicit reason
  -> TaskSpace map record using same semantic payload or stable ref
  -> trace event and benchmark artifact
  -> next agent request can see the exact actionable feedback
```

TaskSpace 可以增加 map、node、edge、summary、projection 和 cache planner，但不应该在默认路径中重写
tool result 的核心语义。核心语义至少包括：

```text
tool name
call id
source path / target path
arguments summary
exit status / structured status
stdout preview or output ref
stderr preview or error ref
failure type
provider-visible payload id/hash
TaskSpace node id / action id
projection decision and reason
```

## 1.2 已识别代码入口

| File | Function / Area | Role | Static Risk |
|---|---|---|---|
| `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs` | `handle_tool_call` / `handle_tool_call_with_source` | direct tool 执行入口 | success 和 error 记录路径不同 |
| `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs` | `record_taskspace_tool_result` | direct tool 写入 TaskSpace map | 只覆盖 `should_attribute_taskspace_tool=true` 的工具 |
| `third_party/codex-cli/codex-rs/core/src/tools/parallel.rs` | `should_attribute_taskspace_tool` | 决定是否写入 map | 排除 non-direct、multi-agent control、taskspace control |
| `third_party/codex-cli/codex-rs/core/src/tools/context.rs` | `ToolOutput::to_response_item` | standard model-visible feedback 抽象 | 应成为 TaskSpace 反馈语义源 |
| `third_party/codex-cli/codex-rs/core/src/tools/context.rs` | `tool_output_model_visible_preview` | 从 response item 抽取 preview | success path 已用，error/internal path 未全覆盖 |
| `third_party/codex-cli/codex-rs/core/src/session/turn.rs` | action-contract prompt / response helpers | action-contract 内部 tool call/result 合成 | 可能绕过 standard tool result 入口 |
| `third_party/codex-cli/codex-rs/core/src/session/turn.rs` | `provider_visible_history_action` | active projection 过滤/保留 | 可能 omit tool result 或破坏 pairing |
| `third_party/codex-cli/codex-rs/core/src/session/turn.rs` | `is_large_raw_tool_output` | large output 判断 | 只覆盖部分 output 类型 |
| `third_party/codex-cli/codex-rs/core/src/tools/code_mode/mod.rs` | `call_nested_tool` | CodeMode nested tool | 使用 `ToolCallSource::CodeMode`，不走 direct map attribution |
| `third_party/codex-cli/codex-rs/core/src/tools/handlers/multi_agents_common.rs` | `tool_output_response_item` | multi-agent output 包装 | JSON text wrapper 与 direct path 语义未完全统一 |

## 1.3 分支推理矩阵

| Branch | Current Static Observation | Required R4 Decision |
|---|---|---|
| direct tool success | 已可通过 `ToolOutput::to_response_item` 生成 model-visible preview 并写 map | 保留，但补 payload hash / node attribution proof |
| direct tool execution error | 2026-06-30 已改为从 `failure_response_for_error` 的 standard `ResponseInputItem` 提取 map preview | 继续补 payload hash / envelope trace，确认 stderr/exit/path 不丢 |
| direct tool fatal / abort | 可能不进入普通 response item | 明确是否 provider-visible、是否 map-visible、是否 terminal |
| action-contract internal apply_patch | 2026-06-30 已在 recent-output summary 中为 failed `taskspace-action-contract-*` 输出添加 `TaskSpaceToolFeedbackV1`，missing update target 会带 `failure_kind`、`target`、`next_valid_action` 和 raw output | 继续复跑 `count-call-stack`，证明真实下一轮 payload 可纠错 |
| action-contract shell/run_test | 可能被 node policy 拦截或作为 validation/recovery text | 区分 policy rejection、tool execution failure、test failure |
| action-contract JSON parse rejection | 当前像 prompt correction/recovery 文本 | 需要 tool-like structured feedback，避免模型无视失败继续提交 |
| node policy violation | 例如 `unknown:read_file`、`inspect_code_context:run_test` | 反馈应包含当前 node 可用 action、原因、下一步可执行选择 |
| active projection absent | history 全量进入 payload 的风险较高 | 需要保证 pairing 正确但允许作为 fallback |
| active projection present | legacy TaskSpace output / large raw output 可被 omit | 必须有 explicit projection reason 和 ref，并测试 pairing |
| small output | 可直接进入 payload/map preview | preview 与 standard output 内容一致 |
| large output | 可能走 large raw omission 或 ref | ref 必须可检索，summary 不能替代 actionable error |
| CodeMode nested tool | `ToolCallSource::CodeMode` 被 direct attribution 排除 | 决定是否归入当前 node 子 trace，至少不能丢 provider-visible feedback |
| multi-agent tool | spawn/wait/close 等被 map attribution 排除 | control 类可排除，但 agent 结果和错误需要明确归属 |
| MCP / tool-search output | `to_response_item` 支持，但 recent-output selection 未全覆盖 | 补 coverage 或明确非 TaskSpace path |

## 1.4 语义丢失检查点

R4 静态审计和实现时必须逐点检查：

1. `call_id` 是否跨 synthetic call、runtime call、provider output、map record 保持可追踪。
2. `stderr` 是否被 summary 截断到失去可执行原因，例如缺少目标路径、缺少 verification failed。
3. `stdout` 大输出是否用 ref 保存，并在 agent 需要时可渐进暴露。
4. tool `exit_code`、timeout、abort 是否与业务失败区分。
5. `apply_patch` 失败是否区分语法错误、路径错误、上下文不匹配和权限错误。
6. `run_test` 失败是否区分测试断言失败、命令找不到、环境缺失和 policy 不允许。
7. active projection 是否同时处理 tool call 和 tool result，不能只 omit 一侧。
8. TaskSpace map 摘要是否基于同一个 canonical feedback，而不是独立手写摘要。

## 1.5 管理机制缺口

当前缺口不是“缺一两个 if 分支”，而是缺少 tools 链路治理：

| Governance Area | Required Mechanism |
|---|---|
| Path ownership | 每类 tool result path 必须有 owner phase 和测试名 |
| Semantic contract | 定义 `ToolFeedbackEnvelope` 或等价 contract，统一 provider/map/projection 使用 |
| Projection audit | 每次 omit/summary/ref 都记录 reason、source item id、target ref |
| Replay safety | replay 能根据 trace 重建 provider-visible tool feedback |
| Benchmark observability | pair report 增加 tool feedback loss、projection omit、large-output ref、policy-loop 指标 |
| Regression fixtures | known-bad samples 固化成轻量 fixture，避免只靠人工读 target 目录 |

## 1.6 R4-A 退出条件

R4-A 完成时需要产出：

1. tool path coverage table：direct、action-contract、nested、multi-agent、MCP、large-output 全覆盖。
2. 每个 path 标记 `canonical`、`needs-fix`、`intentionally-excluded` 或 `out-of-scope`。
3. 每个 `intentionally-excluded` path 有工程理由和回归测试。
4. P0/P1 path 有对应 R4 phase owner。
5. 至少一个静态测试或脚本能防止未来新增 tool path 未登记。

2026-06-30 工程化补充：

```text
Manifest:
  docs/v0.0.5/build-R4/r4-tool-path-coverage.json
Validator:
  scripts/taskspace-benchmark/test-r4-tool-path-coverage.ps1
Gate integration:
  scripts/taskspace-benchmark/build-v005-non-agent-gates.ps1
Gate name:
  r4_tool_path_coverage
```

该 manifest 把 R4-A 静态链路矩阵变成机器可读门禁。validator 会检查：

1. path id 唯一。
2. status 只能是 `canonical`、`needs-fix`、`intentionally-excluded`、`out-of-scope`。
3. 不允许 `unknown` 或 `unowned`。
4. P0/P1 path 必须绑定 `R4-A` 到 `R4-H` 中的 owner phase。
5. 每个 source anchor 必须能在当前源码中找到。
6. 每个 path 必须声明 required semantics 和 required evidence。
7. `intentionally-excluded` path 必须写明 rationale 和 test。

这只能关闭 R4-A 的“管理机制/覆盖登记”缺口，不能代替 R4-C 到 R4-F 的 runtime 语义修复。

## 1.7 2026-06-30 R4-D 链路更新

`count-call-stack` 真实样本暴露出 action-contract internal tool path 不是单一 bug，而是连续 5 个反馈断点：

1. failed edit feedback 有 stderr，但 generic needs-edit recovery 没把失败原因作为下一轮行动依据。
2. fixed retry 能改对文件后，validation node 仍继续 read/list，进入发现循环。
3. validation gate 已发现 `scripts/validate.py`，但 pytest-only failure 没有变成足够明确的 next action。
4. unreviewed-result blocker 要求 `result_validities`，但反馈不够结构化，模型继续读文件。
5. inspect 节点读到的源文件在 implement 投影中被压成 preview，导致 implement 节点认为还没读过可编辑源。

对应 runtime 修复如下：

| Path | Runtime Decision | Evidence |
|---|---|---|
| failed edit feedback | 保留最近 failed edit summary，插入 `TaskSpaceEditFailureRecoveryV1`，不再用固定 recovery 次数硬停 turn | `edit_failure_recovery_preserves_failed_tool_feedback` |
| internal shell/test result map preview | TaskSpace 记录 tool result 前加 `TaskSpaceToolInvocationV1`，保留 tool 和 command 语义 | `taskspace_tool_result_preview_keeps_shell_command_context` |
| validation node policy | validation node 只允许 `run_test`、`taskspace_control`、`blocked`，拒绝继续 read/search/list | `validation_needs_test_recovery_blocks_discovery_loop` |
| local validator coverage | 如果已发现 `scripts/validate.py`，pytest-only validation 被拒绝，并给出 exact next action `python scripts/validate.py` | `validation_gate_requires_discovered_local_validator_over_pytest_only` |
| action-contract gate feedback | local-validator coverage 和 unreviewed-result blocker 输出 `TaskSpaceToolFeedbackV1`，进入 recent tool feedback | `action_contract_prompt_structures_local_validator_coverage_failure` / `action_contract_prompt_structures_unreviewed_result_blocker` |
| dependency read evidence | implement projection 增加 `dependency_read_evidence`，从上游 inspect 节点携带成功 read 的路径和有界内容 | `implement_projection_includes_dependency_read_evidence` |

真实收益证明：

```text
RunDir: target/r4-d-count-call-stack-dependency-read-20260630/count-call-stack/20260630-204427-136
outcome_standard: solved
outcome_taskspace: solved
failure_taxonomy: none
standard_wall_time_ms: 138205
taskspace_wall_time_ms: 154525
standard_tool_call_count: 20
taskspace_tool_call_count: 11
changed_paths: src/call_stack_counter.py
public_validation_exit_code: 0
```

结论：`action-contract-internal-tool-error` 这个 P0 path 已从 `needs-fix` 提升为 `canonical`。这不代表所有 parse/policy rejection、large output、CodeMode、multi-agent、MCP 路径都已关闭；这些仍由 R4-E/R4-F/R4-G 验证。

## 1.8 2026-06-30 R4-E 链路更新

`large-output-ref-smoke` 历史现场证明：如果大输出只在 provider projection 层处理，而 rollout
持久化仍保留完整 raw tool output，TaskSpace 会出现 artifact/log bloat，之前的右侧 rollout 达到
`490,846,386` bytes 并触发 900s timeout。

本轮补齐 persistence-layer 防线：

| Path | Runtime Decision | Evidence |
|---|---|---|
| rollout persistence | `persist_rollout_items` 写入前调用 `sanitize_rollout_items_for_persistence` | `rollout_persistence_referenceizes_large_tool_outputs` |
| large function output | `sanitize_rollout_output_text_for_persistence` 将大 text 输出写入 output-ref artifact，并在 rollout 中保留 `OutputReferenceV1` 摘要 | `target/r4-e-large-output-ref-20260630/.../right/artifacts/output-ref-events.jsonl` |
| provider payload scan | exact payload scanner 确认 provider payload 没有 legacy history 和 large raw output | `large_raw_output_tokens=0`、`replacement_confirmed=true` |
| real sample log bound | rerun 后 TaskSpace rollout 从 `490,846,386` bytes 降到 `360,600` bytes，且不再 900s timeout | `target/r4-e-large-output-ref-20260630/large-output-ref-smoke/20260630-211225-432/pair-001/pair-report.md` |

该结果把 `large-raw-tool-output-ref` 从 `needs-fix` 提升为 `canonical`，但不关闭整个 R4-E：
`provider-visible-history-projection` 仍需证明 tool call/result pair-safe projection。

同次运行暴露新 P0 path：

| Path | Symptom | Owner |
|---|---|---|
| `validation-closeout-tool-drain` | `forced_validation_closeout` 将一次诊断工具成功误当成验证成功，模型最终声明 `Validation passed`，但 public validation exit code 为 1，目标源文件未修改 | R4-D |

这个问题必须按 validation/tool-result 语义链路修，不应归因于 large output 或简单提示词。

## 1.9 2026-07-01 R4-F non-direct 链路更新

R4-F 对 CodeMode、multi-agent、MCP 三类非 direct tool path 做了收口分类。结论不是把所有
non-direct tool 都强行写入 TaskSpace map，而是把“谁能看到什么反馈、归属在哪里、是否可 replay”
写成可验证规则。

| Path | Runtime Decision | Evidence |
|---|---|---|
| CodeMode nested tool | `ToolCallSource::CodeMode` 不伪装成 direct model tool call；结果返回给 CodeMode runtime，并在 trace 中保留 code cell parent attribution、runtime tool id 和 raw result payload | `dispatch_lifecycle_trace_records_direct_and_code_mode_requesters`; `mcp_tool_output_code_mode_result_stays_raw_call_tool_result` |
| multi-agent tool output | multi-agent control/result wrapper 通过 `FunctionToolOutput::from_text(...).to_response_item(...)` 生成 standard function-call output；CodeMode 调用返回 structured JSON | `multi_agent_tool_output_response_item_preserves_json_and_success`; `multi_agent_tool_output_code_mode_result_preserves_structured_fields` |
| MCP tool output | MCP output 继续由 `McpToolOutput::to_response_item` 处理，保留 wall time、structured content、content items，并对大 structured content 做 bounded truncation | `mcp_tool_output_response_item_includes_wall_time`; `mcp_tool_output_response_item_truncates_large_structured_content`; `mcp_tool_output_response_item_preserves_content_items` |

同时把 `r4-tool-path-coverage.json` 中所有 canonical path 绑定 `coverage_test`，并让
`test-r4-tool-path-coverage.ps1` 检查该字段。这样 R4-A 的静态治理门禁能防止以后出现
“path 标记 canonical 但没有可运行证据”的回归。

验证：

```text
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/taskspace-benchmark/test-r4-tool-path-coverage.ps1
PASS: R4 tool path coverage gate passed: 10 paths
```

## 1.10 2026-07-03 R4-D 工具运行时启动失败链路更新

`organization-json-generator` keyed rerun 暴露出一类新的 P0 feedback-layer 问题：普通 action-contract
tool 在真正执行业务命令前就因为 sandbox/tool runtime bootstrap 失败而不可用。现场 raw feedback 中有明确证据：

```text
bwrap: loopback: Failed RTM_NEWADDR: Operation not permitted
TaskSpaceNoActionRecoveryV1 ... Recovery attempt 32
```

该 case 的本质不是模型没看到任何错误，而是缺少终止语义：

| Path | Symptom | Required Runtime Decision |
|---|---|---|
| `tool-runtime-bootstrap-failure` | `bwrap`/loopback/RTM_NEWADDR 启动失败被当成普通工具失败，node-level `blocked` 之后仍可继续 `create_node` 重试 | 标记 `failure_kind=sandbox_bootstrap_failed`，block 当前任务路径上的普通工具，下一轮只允许 `final_answer` 或 `blocked` |

修复后的语义要求：

1. 能力层识别 bwrap sandbox bootstrap signature，但 `SandboxType::None` 不误判。
2. `ActionMapRuntime` 对任意普通工具节点识别 `tool_runtime_bootstrap_failure`，记录 blocker 并释放 current node。
3. 验证节点里的同类失败继续归入 local validator infra invalidation，但不生成可重试 rework node。
4. 无 active node 且已存在 runtime bootstrap blocker 时，action-contract payload 和 runtime rewrite 都禁止 `start_task`、`create_node`、`read_file`、`search`、`run_test`、`apply_patch`、`spawn_agent`。

覆盖测试：

```text
cargo test -j1 -p codex-core sandbox_detection_identifies_bwrap_loopback_bootstrap_failure --lib
cargo test -j1 -p codex-core bwrap_bootstrap_failure_auto_blocks_validation_as_local_infra --lib
cargo test -j1 -p codex-core tool_runtime_bootstrap_failure_blocks_inspect_node --lib
cargo test -j1 -p codex-core taskspace_action_contract_tool_runtime_bootstrap_failure_forbids_new_nodes --lib
```

## 1.11 2026-07-03 R4-D tools 问题类型扩展

`organization-json-generator` keyed 复验后，R4-D 的问题类型不再只覆盖“内部 tool error 是否进入下一轮”。
当前需要把能力层和反馈层分开记录：

| Problem Type | Layer | Symptom | Runtime Decision | Status |
|---|---|---|---|---|
| `tool-platform-command-mismatch` | feedback | Linux recovery payload 给出 `Get-Content`，或 Windows payload 给出 `sed` | `read_file` recovery command 按 host platform 生成 | fixed by focused tests |
| `tool-runtime-bootstrap-failure` | ability + feedback | `bwrap`/loopback/RTM_NEWADDR 启动失败后继续开新节点重试 | 分类为 `sandbox_bootstrap_failed` task-level blocker；无 active node 时只允许 `final_answer` / `blocked` | fixed by focused tests |
| `linux-sandbox-restricted-netns-proc` | ability | 容器/宿主不允许 userns/netns/proc mount，工具在业务命令前失败 | 非 proxy restricted network 可退化到 bwrap full network + seccomp；legacy-compatible 场景可退回 Landlock/seccomp | fixed by sandbox tests |
| `duplicate-successful-evidence-loop` | feedback | inspect 中重复读同一文件或重复搜索同一命令 | 分类为 `inspect_duplicate_successful_read_or_search`，回传 previous result 和 repeat state | fixed by focused tests |
| `inspect-data-artifact-evidence-gap` | feedback | `.json`/`.csv` 输入已读但不计为 working evidence | 从 command、result body、evidence refs 合并 input data artifact refs | fixed by focused tests |
| `validation-changed-artifact-coverage-feedback` | feedback | validation gate 拒绝 vacuous test，但下一轮缺少 exact coverage command | 输出 `validation_test_missing_changed_artifact_coverage` 和 required command / next_valid_action | fixed by focused tests |
| `validation-command-missing-script-feedback` | feedback | `python process.py` 这类不存在脚本被误路由成 implementation rework | 留在 validation node，输出 `validation_command_missing_script` 和缺失脚本名 | fixed by focused tests |
| `duplicate-inspect-premature-fact-source-convergence` | feedback + phase gate | 重复 read/search recovery 在只读了部分声明 fact sources 时强制进入 implement | duplicate gate 列出缺失 fact-source artifacts；manual/forced inspect finish 都要求覆盖声明 artifact | fixed by focused tests |
| `provider-budget-advisory-runaway` | control loop + feedback | `request_count` 或单节点 `node_request_count` 到达 active budget 后仍可能继续发 provider 请求 | `gate_provider_request_pre_dispatch` 在 provider dispatch 前 hard stop；session 插入 `TaskSpaceProviderBudgetHardStopV1` 并结束当前 turn；保留一次明确 `budget_recovery` grace | focused fixed / real rerun pending |
| `provider-node-budget-premature-inspect-stop` | control loop + phase gate | per-node hard limit 低于声明 fact-source evidence floor，导致 inspect 未读全 `employees.csv`/`projects.csv` 就 hard stop | inspect 节点的 effective `max_model_requests_per_node` 根据声明 fact-source artifacts 扩展；recovery item 到达边界时下一次请求标记为 `budget_recovery` | focused fixed / real rerun pending |
| `implementation-rework-feedback-evidence-join` | feedback + dependency projection | validation rework 能看到失败，但 recovery 没把最新 validation failure 和上游 inspect 的 CSV/schema evidence 合并成同一行动上下文，导致逐行修 `IndentationError` 或凭空使用 `salary` 等未观察字段 | `current_main_working_evidence_summary` 使用当前节点的有界依赖闭包并合并 `validation_rework` 摘要；`TaskSpaceImplementNeedsEditRecoveryV1` 明确 validation failure 优先、Python 顶层缩进按文件/块整体修、`KeyError` 只能用已观察字段 | focused fixed / real rerun pending |
| `inspect-projection-finish-before-fact-source-coverage` | feedback + provider projection | 底层 duplicate/manual/forced finish guard 已知道缺声明 fact sources，但 context projection 的 `next_valid_actions` 仍暴露 `finish_node -> implement_solution`，模型继续在 inspect 中重复读已读文件直到 node budget hard stop | `projection_next_valid_actions` 接收 `TaskState` 并复用声明 fact-source coverage guard；缺 artifact 时只提示继续读取缺失 fact sources，禁止在投影里广告 `finish_node` / implement transition | focused fixed / real rerun pending |
| `implementation-editable-validation-failure-misblocked` | control loop + feedback | implement rework 的依赖 validation 明确是 `IndentationError` / `SyntaxError` / `KeyError` 等可编辑实现失败，但模型可以 `block_node` 并把它说成 closed validation / infra blocker | `block_main_node` 拒绝这类 editable validation failure blocker；recent feedback 输出 `editable_validation_failure_blocker_rejected`，要求 patch 失败 artifact，Python 顶层缩进/语法错误按文件或块整体修 | focused fixed / real rerun pending |
| `validation-closeout-output-contract-coverage-gap` | validation + feedback + closeout | validation tool result 只是 generator execution success，例如 `python generate_json.py` exit 0 并打印 `organization.json generated successfully`，但 runtime forced closeout 将其当成 output/schema contract 已验证，final answer 声称成功，public validator 仍因 `members` / `averageDepartmentBudget` 等字段缺失失败 | validation gate 要求声明 output contract artifacts 被同一次 `run_test` 的真实 validator/schema/assertion 覆盖；generator-only command 输出 `validation_test_missing_output_contract_coverage`；forced closeout 备份会重开引用该结果的 success criteria 并把 generator-only validation result 标记 invalid | focused fixed / real rerun pending |
| `validation-output-contract-schema-fact-source-gap` | validation + feedback | generator-only 被拒绝后，模型改用 `python -c json.load(open("organization.json"))` 这种弱 JSON parse；runtime 因 `schema.json` 只在 fact source / success criterion 中出现而没有把它纳入 schema target，误把 parse success 当成 schema validation | output-contract coverage 从 output contracts、success criteria、fact sources 一并提取 schema/validator artifacts；有 schema/validator target 时必须看到 `jsonschema` / `validate` / `pytest` / `run-tests` 等真实 schema/validator 语义，普通 `json.load` / `python -c` 不再足够 | focused fixed / real rerun pending |
| `success-criteria-output-artifact-validation-target-gap` | validation + feedback | output contract 是泛化描述时，生成目标 `organization.json` 只写在 success criteria 里；runtime 只提取 `schema.json`，没有 output target，导致 `json.load(open("organization.json"))` 弱验证被 forced closeout 当成完成 | validation requirements 从 `problem_ledger.success_criteria` 和 legacy success criteria 中提取 artifact；非 schema `.json` 生成物进入 output targets，schema/validator artifact 仍进入 schema_targets；弱 JSON parse 输出 `validation_test_missing_output_contract_coverage` 和 exact schema validation command | focused fixed / real rerun pending |
| `validation-recovery-next-action-projection-dilution` | feedback + provider projection | validation gate 和 `TaskSpaceValidationNeedsTestRecoveryV1` 已给出精确 schema validation 命令，但 active/shadow `ContextProjectionV1.next_valid_actions` 重新退化为 `run validator/test command`，模型继续弱重试直到 smoke node hard stop | runtime 记录 latest gate recovery `next_valid_actions`；validation node projection 优先原样输出精确 recovery command，并追加“不替换为更弱 validation”的约束；当前节点记录新 main tool result、清理 blocked repeats 时同步清理该 recovery 状态 | focused fixed / real rerun pending |
| `validation-rework-target-artifact-read-gap` | feedback + action contract + dependency projection | exact schema validation 已执行并返回业务 schema 错误后，runtime 路由到 implement rework；但该 schema failure 没有 traceback/file path，`implementation_needs_edit` 又把 `read_file` 一刀切拦截，模型无法读取依赖变更工件 `generate_org.py` 来修复，只能 block | runtime 从 blocked validation dependency 的 changed artifacts 推导 rework target；provider snapshot/projection 暴露 `current_node_validation_rework_artifacts` 和命名 read action；session action contract 在 `implementation_needs_edit` 下只允许读取这些命名 validation rework target artifacts，继续拦截 `schema.json` 等泛读 | focused fixed / real rerun pending |
| `validation-jsonschema-module-missing-rework-misroute` | validation + feedback + provider projection | `python3 -c "import jsonschema"` 在 agent host 环境缺模块时以 `ModuleNotFoundError` 失败，runtime 将其当成非 infra validation failure 路由到 implement rework；模型读到目标 artifact 后反复 `finish_node`，直到 node hard stop | `ModuleNotFoundError: No module named 'jsonschema'` 从 noninfra implementation rework 分类中排除；validation projection 基于 output contract/schema requirements 给出 `python -m jsonschema -i organization.json schema.json` 的默认 Python CLI recovery，并明确不要把缺 validator dependency 路由成 implementation rework | focused fixed / real rerun pending |
| `implementation-rework-repeat-read-budget-drain` | feedback + attribution + control loop | validation rework 已允许读取目标 artifact，但 Unix action-contract `read_file` 通过 `sed -n '1,240p' -- csv_processor.py` 执行，read result 记录为 `artifactRefs=[]`；runtime 的 duplicate rework read gate 无法识别同一文件已读，模型反复读 `csv_processor.py` 直到 node budget hard stop | `read_command_artifact_ref` 识别稳定 Unix `sed -n ... -- path` 读文件命令并把 path 写入 read result evidence；第一次 target read 仍允许，第二次同 target read 在无成功 edit 前触发 `validation_rework_duplicate_artifact_read`，要求 `apply_patch` 或 blocked | focused fixed / real rerun pending |
| `validation-blocker-manual-rework-origin-loss` | feedback + DAG origin + control loop | duplicate read gate 已阻止重复读取后，模型可在 validation recovery 中先手动 `create_node(implement_solution)` 再 `block` validation；runtime 默认把新 rework node 挂到最近 completed implementation，`origin_node_id` 为空，后续 patch 被 `result still unreviewed` gate 拦截并耗尽 provider budget | detached `implement_solution` 若从 active validation node 创建，会记录该 validation node 为 `origin_node_id` 并加入依赖边；当 origin validation 被 blocked 时，只刷新匹配的 pending rework node 为 Ready；active rework edit 可使用该 blocker input，不需要额外 state_commit 自救 | focused fixed / real rerun pending |
| `validation-stale-failure-block-without-current-test` | feedback + validation evidence + control loop | rework patch 后新建的 smoke/regression node 尚未运行当前验证命令，却可用上一轮 `IndentationError` 等旧失败文本 `block_node`；graph 被关掉但 public validation 仍失败 | smoke/regression `block_node` 若声称 validation/test failure，必须先有同节点 `Build`/`Test` tool result；local validator infrastructure blocker 仍允许按 infra path 路由 | focused fixed / real rerun pending |
| `validation-rework-duplicate-read-projection-loop` | feedback + projection + control loop | target read 的 artifact identity 已存在且 duplicate gate 已拦截重复 `read_file`，但 compact projection 仍显示 read/search 可用、target contents 未作为 critical evidence 展示，模型反复读同一 rework artifact 直到 node budget hard stop | target artifact 被读过后，projection 将该 result 作为 `validation_rework_target_read` critical evidence 展示，next_valid_actions 改为 use existing result + `apply_patch`，allowed actions 收窄为 edit/control 并声明 read/search 会被 blocked | focused fixed / real rerun pending |
| `validation-rework-duplicate-read-recovery-dilution` | feedback + session recovery + control loop | projection/action-contract 已正确拒绝重复 rework target read 并要求 patch，但 session follow-up 又降级成 generic `TaskSpaceImplementNeedsEditRecoveryV1`，模型连续重复读直到 node hard stop | session 新增 `TaskSpaceValidationReworkDuplicateReadRecoveryV1`，保留 failure_kind、target、previous result、repair contract 和 GateRecovery；implementation recovery selection 优先该专用 patch-only recovery | focused fixed / real rerun pending |
| `validation-rework-duplicate-read-immediate-recovery-bypass` | feedback + session recovery routing | 专用 duplicate-read recovery selector 已存在，但 response-completed 的即时 `response_actionability.needs_recovery()` 分支仍直接调用 generic implement-needs-edit recovery；真实 trace 中 gate 文本已说明 target 已读、必须 patch，后续 warning 仍连续插入 `TaskSpaceImplementNeedsEditRecoveryV1` 到 node hard stop | 即时 implementation recovery 分支改用统一 `build_taskspace_implementation_recovery_item` selector，并传入 failed-edit summary；selector 可从自然语言 blocked-read 文本识别 duplicate rework read；warning 明确输出 `TaskSpaceValidationReworkDuplicateReadRecoveryV1` | focused fixed / real rerun pending |
| `apply-patch-native-hunk-recovery-dilution` | feedback + session recovery + observability | action-contract 正确拒绝 `apply_patch_mixed_native_unified:<target>`，但 advisory warning 将专用 recovery 标成 generic `TaskSpaceImplementNeedsEditRecoveryV1`；随后 duplicate-read recovery 只说 patch now，没有保留 native apply_patch grammar 约束，模型继续提交 unified/range hunk 到 node hard stop | advisory warning helper 覆盖 `TaskSpaceApplyPatchNativeHunkRecoveryV1` / unanchored / format 等 patch grammar recovery；duplicate-read recovery 补充 native apply_patch grammar 约束，禁止 `--- a/...`、`+++ b/...`、range hunk 和 placeholder hunk | focused fixed / real rerun pending |
| `apply-patch-dash-native-header-feedback-gap` | action contract + feedback + control loop | provider 在 rework 中输出 `--- Update File: <path>` 加 `@@ -... +@@ ... @@`，这既不是合法 native `*** Update File`，也不是标准 unified diff；action contract 漏检后进入 apply_patch 工具失败，只剩 generic `TaskSpaceEditFailureRecoveryV1`，后续又允许同目标 read_file 到 node hard stop | action contract 将 `--- Update File:` / `--- Add File:` / `--- Delete File:` 归类为 native hunk/header 语法错误，dispatch 前拒绝为 `apply_patch_native_hunk_header:<target>`；NativeHunk recovery 文案显式禁止 `--- Update File:` | focused fixed / local regression passed / real rerun pending |
| `validation-rework-duplicate-read-after-patch-grammar-feedback-loss` | session recovery composition + observability + control loop | NativeHunk recovery 已正确插入后，provider 又重复读已读 rework artifact；duplicate-read recovery 优先级高于 recent failed edit summary，导致 `apply_patch_mixed_native_unified:<target>` 语义被普通 duplicate-read recovery 覆盖，并在 node hard stop 前丢失最具体 patch grammar 反馈 | duplicate-read recovery 接收并展示最近 failed edit summary；若包含 `apply_patch_mixed_native_unified` / `apply_patch_native_hunk_header`，明确 read_file/context refresh 不是有效恢复；warning 细分为 `TaskSpaceValidationReworkDuplicateReadAfterPatchGrammarRecoveryV1` | focused fixed / local regression passed / real rerun pending |
| `apply-patch-mixed-native-unified-auto-normalization-gap` | ability + action contract + control loop | provider 连续输出 `*** Update File` 内夹 `--- a/...` / `+++ b/...` / concrete range hunk 的可机械转换 patch；runtime 已多次回传 `TaskSpaceApplyPatchNativeHunkRecoveryV1`，但 action contract 在 normalizer 前拒绝为 `apply_patch_mixed_native_unified:<target>`，最终靠 node hard stop 收尾 | malformed `--- Update File:` 和 placeholder `@@ ... @@` 仍在 dispatch 前拒绝；安全 mixed native/unified payload 先走 patch normalizer，strip unified file headers、range hunk 归一为 native `@@`，规范化后再执行 mixed/native/unanchored/missing-target 检查 | real rerun crossed / next blocker: `apply-patch-non-diff-update-payload-feedback-gap` |
| `apply-patch-non-diff-update-payload-feedback-gap` | action contract + feedback + control loop | provider 输出 `*** Update File: organization.json` 后直接放入 `python3 -c` / JSON transformation command，没有 `@@`、`-old`、`+new` native hunk；runtime 将其送进 apply_patch 工具，失败后退化成 generic `TaskSpaceEditFailureRecoveryV1` 并重新打开 read drain path | `*** Update File` section 若有内容但没有任何 native diff change 行，dispatch 前拒绝为 `apply_patch_unanchored_update:<target>`；recovery 明确 apply_patch 只接受 native diff，不能嵌 shell/Python/JSON transformation command；deletion-only native patch 保持合法 | focused fixed / local regression passed / real rerun pending |
| `python-add-file-common-indent-normalization-gap` | ability + apply_patch normalization + control loop | provider 用 native `*** Add File: generate_org_json.py` 创建 Python 脚本，但每个新增内容行都是 `+ import...` / `+ def...` 这类统一多一格前导空格；工具按字面创建文件后触发 `IndentationError`，rework 阶段重复读已读目标到 node hard stop | 仅对 `*** Add File: *.py` / `*.pyw` 且所有非空新增内容行都统一多一个前导空格的 section，去掉一层共同缩进；保留相对缩进，不处理非 Python 文件或混合缩进内容 | focused fixed / local regression passed / real rerun pending |
| `apply-patch-anchored-placeholder-hunk-normalization-gap` | ability + action contract + control loop | provider 在预算恢复末尾终于输出目标 patch，但 hunk 是 `@@ ... @@` placeholder，且带有真实上下文行；runtime 将所有 placeholder hunk 硬拒绝为 `apply_patch_native_hunk_header:<target>`，NativeHunk recovery 出现后立即 hard stop | 带上下文/变更行的 `@@ ... @@` 规范化为 native `@@`，再交给 unanchored/context/missing-target 检查兜底；malformed `--- Update File:` 仍拒绝 | focused fixed / local regression passed / real rerun pending |
| `apply-patch-targetless-unified-header-fake-target` | feedback + action contract normalization | provider 输出 targetless `---` / `+++` unified-like patch；bare-file normalizer 把 separator-only `---` 当文件名并 fallback 成 `src/---`，recovery 看到的是伪目标而不是缺目标 | `normalize_taskspace_bare_file_patch` 禁止 `---` / `+++` 作为 bare path；action contract 在 dispatch 前识别 targetless unified headers，拒绝为 `apply_patch_mixed_native_unified:(missing patch target)`，不再生成 `src/---` | focused fixed / local regression passed / real rerun pending |
| `apply-patch-separator-update-section-normalization-gap` | ability + action contract normalization | provider 在 `*** Update File` section 内用 `<old block>` / separator-only `---` / `<new block>` 表达替换，并可能在完整 apply_patch JSON 后多输出一个 `"`；runtime 只能进入 strict JSON / edit-failure recovery hard-stop | 仅对 apply_patch action 容忍单个尾随 `"`；无已有 hunk/diff marker 且恰有一个 separator-only `---` 的 Update File section 机械转成 native `@@`、`-old`、`+new` hunk | focused fixed / local regression passed / real rerun pending |
| `closed-validation-success-final-blocked-false-positive` | feedback + terminal closeout | schema validation 已通过并 forced closeout，但旧 blocked validation / local infra evidence 仍可能注入 blocker contract 或接受 terminal `blocked` | successful validation final readiness 优先于旧 blocker；无 active node 且已有 accepted validation 时不再注入 blocker contract，并把 terminal `blocked` 转为 `final_answer` | focused fixed / real rerun pending |

其中 `duplicate-inspect-premature-fact-source-convergence` 是本次新增收录的 case。它不是工具原始失败，也不是单纯模型策略错误；
raw evidence 存在，问题在 feedback/phase gate 语义缺失：runtime 把“重复读已成功”恢复成“inspect 可结束”，但没有检查
`initial_fact_sources` / `fact_sources` 中声明的 `employees.csv`、`projects.csv` 是否已经被成功 inspect。

manual validation rework origin 修复后的 rerun `20260704-032001-321` 已证明 origin/lifecycle gate 不再是当前 blocker，
但暴露 `validation-stale-failure-block-without-current-test`：模型 patch 了 `generate_org.py` 的第一行后进入新的
smoke node，却没有在该 node 运行新的 validation command，而是复用上一轮 `IndentationError` 文案直接 block。
这是反馈层的“旧失败语义缺少当前节点证据约束”：语义没有完全丢失，但被跨节点复用成了当前 validation 结果。

stale validation block guard 修复后的 rerun `20260704-033716-688` 进一步越过旧失败复用：TaskSpace 执行了真实 schema
validation 并进入 rework。但 rework target `process.py` 已被成功读取后，projection 仍把 `read_file process.py only if current
contents are not visible` 作为 next action，并没有把 `result-11` 作为当前 critical evidence 展示。新的问题类型是
`validation-rework-duplicate-read-projection-loop`：底层 gate 正确，provider-visible projection 仍冲淡了“现在必须 patch”的约束。

`provider-budget-advisory-runaway` 是本轮继续推进后收录并修复的控制环 case。真实 keyed rerun
`target/r4-org-json-real-keyed-20260703d/.../whale-exec.jsonl` 显示前置 feedback fixes 生效：TaskSpace 读到了
`schema.json`、`departments.csv`、`employees.csv`，没有再因 duplicate read/search 过早强制进入 implement；
但 `projects.csv` 未读时，provider request budget 从 `19->20 max=20` 以后继续到 `26->27`，
全部处于 `over_profile_hint`。根因不是预算语义写错，而是预算 gate 没有接入 provider dispatch 前的硬阻断路径。

后续 keyed rerun `20260704-000713-854` 证明 hard stop 已真实生效并消除 900s timeout，但同时暴露
`provider-node-budget-premature-inspect-stop`：右侧在 `node_request_count=5/5` 时 hard stop，尚未读取
`employees.csv` 和 `projects.csv`。这说明预算硬门不能低于状态机的最低证据地板；否则 runtime 虽然没有超越
状态机强行推进 phase，却会在 phase 完成条件可达前终止。

再次 rerun `20260704-001749-411` 后，inspect 已越过 evidence floor 并读完 `employees.csv`、`departments.csv`、
`projects.csv` 和 schema，但 implement rework 暴露 `implementation-rework-feedback-evidence-join`：第一次生成文件的顶层缩进
系统性错误被逐行修复，后续 replacement 又使用了 CSV 中不存在的 `salary` 字段，最终 `KeyError: 'salary'`。
这不是能力层工具不可用，而是 feedback 层没有把“最新 validation failure”和“已经读到的字段证据”合并为同一 next-action contract。

rework evidence join 修复后的 rerun `20260704-003459-046` 又暴露 `inspect-projection-finish-before-fact-source-coverage`：
TaskSpace 已读 `schema.json`、`departments.csv`、`employees.csv`，但没有读 `projects.csv`；runtime 的 duplicate/read guard
能够识别缺失 fact source，最后也被 provider node budget hard stop 截断。然而该轮 context projection 的
`next_valid_actions` 仍然包含：

```text
taskspace_control(action=finish_node, ... next_node_kind="implement_solution" ...)
```

这说明语义不是完全缺失，也不是底层 guard 被扭曲，而是 provider-visible projection 层缺少同一套 fact-source
coverage 判断。模型看到“可以 finish”的合法动作后，继续在 inspect 内重复读取 `schema.json`，没有被清晰导向
`projects.csv`。

projection guard 修复后的 rerun `20260704-004643-993` 证明 TaskSpace 已按要求读取 `projects.csv`，并进入
implement/validation 链路；新的失败类型是 `implementation-editable-validation-failure-misblocked`：

```text
python generate_organization.py
IndentationError: unexpected indent
```

该失败在验证输出中是明确的实现代码错误。TaskSpace 已经把验证失败路由到 implement rework，但 rework 节点随后接受了
`block_node`，最终 final action 说“closed validation state prevents further editing”并把缩进问题归为
`infra-evidence-unresolved-indentation`。这属于 control/feedback 语义扭曲：可编辑实现失败被错误提升为不可继续的
infra blocker。

第二次 rerun `20260704-005922-113` 仍属于同一类型，但 provider wording 变成
`cannot read files to diagnose because read actions are not allowed in current narrowed state`。因此 detector 不能只匹配
`need to inspect` / `closed validation`，还要把 `cannot read`、`read actions are not allowed`、`read restriction`、
`insufficient information` 和 `current narrowed state` 这类“把可编辑失败归因为读权限/状态限制”的说法纳入拒绝条件。

editable blocker wording 修复后的 rerun `20260704-010752-603` 进一步暴露
`validation-closeout-output-contract-coverage-gap`：TaskSpace 已生成 `organization.json`，但 validation 只运行了
`python generate_json.py`。该命令 exit 0 表示 generator 执行成功，不表示输出满足 schema/public tests。runtime 仍触发
`TaskSpaceForcedValidationCloseoutV1 trigger=validation_success_after_tool_drain`，final answer 声称成功；public validator 随后报
`KeyError: 'members'` 和 `KeyError: 'averageDepartmentBudget'`。这属于反馈层验证语义缺失：工具成功没有丢失，但成功的含义被提升得过宽。

对应修复把 output contract coverage 接入 validation gate 和 forced closeout 备份路径：如果任务声明了
`organization.json` / `schema.json` 等 output contract artifact，`run_test` 必须执行变更 artifact 并检查输出契约，例如
`python generate_json.py && python -m jsonschema -i organization.json schema.json`，或运行真实项目 validator。若 generator-only
结果已被记录，closeout 会撤销由该结果支撑的 satisfied success criterion，并将该 result 标记 invalid。

该修复后的 rerun `20260704-013819-201` 证明 generator-only 语义已经传回模型：`python process.py` 被拒绝并插入
`TaskSpaceValidationNeedsTestRecoveryV1`。但模型随后使用 `python process.py && python -c 'json.load(open("organization.json"))'`，
runtime 接受了该弱验证，final answer 声称 schema validated，public validator 仍因 `members` /
`averageDepartmentBudget` 失败。根因是 schema artifact 不一定由 output contract 直接声明；它可以来自
`initial_fact_sources` 或 success criteria。现在 schema/validator fact sources 也进入 output-contract validation
requirements，且有 schema/validator target 时必须看到真正的 schema/validator validation 语义，不能只做 JSON parse。

schema fact-source guard 修复后的 rerun `20260704-014928-473` 进一步说明 feedback 链路需要跨层保持精确语义：
`TaskSpaceGateRecoveryV1` 和随后的 `TaskSpaceValidationNeedsTestRecoveryV1` 都包含
`python process.py && python -m jsonschema -i organization.json schema.json`，但 active projection 又把
`next_valid_actions` 泛化成 `run validator/test command`。这不是工具能力层问题，也不是 gate recovery 缺失，而是
provider-visible projection 稀释了已经生成的恢复动作。现在 runtime 会把最新 gate recovery 的 `next_valid_actions`
作为节点级状态传入 projection；只要当前 smoke/regression node 仍未产生新状态，projection 就优先展示 exact command，
避免下一轮上下文给模型一个更弱的合法动作。

对应 focused gate：

```text
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_duplicate_read_reports_missing_fact_source_artifacts_without_finish --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core inspect_missing_fact_sources_block_manual_and_forced_finish_until_read --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core taskspace_active_budget --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core provider_budget --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_summary_merges_transitive_inspect_evidence_and_failure --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core implement_recovery_prioritizes_validation_failure_and_inspected_fields --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core projection_blocks_inspect_finish_until_declared_fact_sources_read --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core validation_rework_rejects_editable_validation_failure_blocker_before_edit --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -j1 -p codex-core action_contract_prompt_structures_editable_validation_failure_blocker_rejection --lib
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks_generator_only_command_for_schema_output_contract --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation_rejects_generator_only_output_contract_success --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core action_contract_prompt_structures_output_contract_coverage_failure --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_requires_schema_fact_source_for_output_contract_check --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_node_blocks --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core force_finish_validation --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo test -p codex-core validation_ --locked
CODEX_SKIP_VENDORED_BWRAP=1 cargo build -p codex-cli --bin whale --locked
```

## 2026-07-04 R4-D issue type addendum: duplicate-read advisory loop

本轮把 `organization-json-generator` keyed rerun 中暴露的反馈层 case 收录为新的 R4-D tools 链路问题类型。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-duplicate-read-advisory-loop` | runtime recovery loop / feedback layer | validation rework 已经读取目标 artifact，projection 与 action-contract 都要求 `apply_patch` 或具体 `block_node`，但模型重复 `read_file` 同一目标；runtime 继续 advisory recovery，最后 provider/node budget hard stop | 第一条 duplicate-read recovery 保留纠错机会；第二条同类 recovery 或带 `repeated_blocked_action` 的 gate 升级为 `TaskSpaceValidationReworkDuplicateReadHardStopV1`，停止当前 turn 的 provider sampling，并保留 bounded evidence | `validation_rework_duplicate_read_hard_stops_after_one_recovery`; `validation_rework_duplicate_read_repeated_gate_hard_stops_immediately`; CoE E-085/E-086 |

边界说明：该 case 不是 tool executor 吞掉错误，也不是 feedback 语义缺失。失败语义已经通过
`TaskSpaceValidationReworkDuplicateReadRecoveryV1`、`TaskSpaceGateRecoveryV1` 和 active projection 正确传达；缺口是 runtime
把重复违反同一 patch-only gate 继续当成 advisory retry。修复只停止重复采样，不代替模型生成补丁，也不把节点伪装成业务外部 blocker。

## 2026-07-04 R4-D issue type addendum: post-edit transition gap

duplicate-read advisory loop 修复后的 keyed rerun 又暴露一个相邻 control/feedback case：模型已经产出成功
`apply_patch`，action map 也记录了 `MainToolCall actionClass=edit toolSuccess=true`，但 implement node 没有在预算边界
收束到 validation，下一轮 provider pre-dispatch 才被 hard stop 截断。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `post-edit-forced-validation-transition-gap` | runtime control loop / feedback layer | successful edit 已进入 action map，且 `node_request_count == max_model_requests_per_node`；runtime 未触发 `forced_implement_transition`，导致 validation 没有运行，open leaf 最终由 `TaskSpaceProviderBudgetHardStopV1` 截断 | snapshot budget pressure 采用真实 hard-limit 判断；成功 edit + node/profile 边界时强制 finish implement 并创建 smoke validation；未到边界不抢跑，未成功 edit 时仍由原 hard gate/feedback 处理 | `provider_budget_node_limit_force_finishes_implementation_into_smoke_test_after_edit`; `provider_budget_below_node_limit_does_not_force_finish_implementation_after_edit`; CoE E-087/E-088 |

边界说明：该 case 不是 edit feedback 缺失。工具成功语义、artifact refs 和 action map result 都存在；缺口在成功工具执行后的
phase transition guard。runtime 的职责边界仍然是状态机底线：只有在 implement node 已有成功 edit，且 provider/node
预算已经到 hard-limit 边界时，才自动桥接到 validation，避免下一次 provider 请求被硬停吞掉验证机会。

## 2026-07-04 R4-D issue type addendum: schema failure semantic truncation

post-edit transition 修复后的 keyed rerun 证明 provider/node hard stop 不再吞掉成功 edit 后的验证机会，但暴露出更细的
feedback-layer 语义保真问题：真实 `jsonschema` 输出包含所有缺失 required properties，ActionMap 中用于 rework 的
`result-9` body 却只保留 telemetry preview，截断在 statistics 错误行之前。后续 blocker 和 repair contract 因此只看到
`members`，丢失 `averageDepartmentBudget`、`totalEmployees`、`skillDistribution`、`departmentSizes`、
`projectStatusDistribution`、`averageYearsOfService`。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-schema-required-property-summary-truncated-before-action-map` | exec formatter / tool result preview / feedback layer | validator raw output 完整，但 shell_command error path 在 `ExecToolCallOutput -> FunctionToolOutput` 阶段先生成截断后的 model-visible string；ActionMap 下游 `validation_schema_repair_contract` 只能解析出截断前的 required-property 失败 | 在 exec formatter 的 `format_exec_output_str_with_ref` 截断前抽取 `TaskSpaceToolSemanticSummaryV1`，把 `missing_required_properties:` 摘要前置到 formatted output；ToolOutput preview 复用同一 helper；bounded raw preview 仍保持截断，普通非 schema 输出不新增摘要 | `exec_output_formatter_preserves_schema_summary_before_truncation`; `taskspace_preview_preserves_required_properties_from_untruncated_exec_output`; `taskspace_preview_does_not_add_schema_summary_for_plain_exec_output`; `validation_rework_projects_schema_repair_contract_from_schema_read`; CoE E-089/E-091/H-042 |

边界说明：该 case 是“语义缺失”，不是模型不服从，也不是 validation gate 判断错误。完整失败语义在 shell command
原始输出中存在，但进入 FunctionToolOutput / ActionMap 的 body 已经被 exec formatter / telemetry preview 截断。修复只提升结构化
failure summary，不扩大 raw output 窗口，也不把任意长输出直接暴露给 provider/map。

## 2026-07-04 R4-D issue type addendum: failed-edit projection dilution

schema formatter 修复后的 keyed rerun 证明 required-property repair contract 已完整进入 live projection，但下一层
`apply_patch` context mismatch 失败后，模型仍反复声明 edit 已成功并尝试 finish。runtime guard 正确拒绝 finish，
但 provider-visible projection 没有把 failed edit 作为当前最重要的 critical evidence。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `failed-edit-projection-recovery-dilution` | feedback + provider projection + control loop | failed `apply_patch` 结果存在，`TaskSpaceEditFailureRecoveryV1` 也保留失败文本，但 active projection 仍把条件性 future `finish_node after successful edit` 混进 `next_valid_actions`；failed edit 仅在 hidden refs / recovery 文本里，模型继续 `finish_node` 到 node budget hard stop | failed edit 晋升为 `critical_artifact_evidence` 的 `failed_edit_feedback signal=latest_failed_edit`；validation rework 在无成功 edit 前不暴露 immediate `taskspace_control(action=finish_node)`；allowed-actions 明确 finish blocked until successful edit，同时保留必要的同目标 refresh read | `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback`; `validation_rework`; `validation_`; `apply_patch_`; CoE E-093/H-043/E-094 |

边界说明：该 case 不是工具失败语义完全缺失，也不是状态机允许了非法 finish。状态机已经正确拒绝 finish；缺口在
projection 把“成功 edit 之后的下一步”当作当前候选动作展示，并没有把 failed edit 作为最高优先级反馈。修复保持
runtime 底线不变，只收紧 provider-visible next action 语义。

## 2026-07-04 R4-D issue type addendum: unanchored patch feedback loss after duplicate read

failed-edit projection 修复后的 keyed rerun 继续推进到 `apply_patch` grammar 层：模型提交了没有 native hunk 的
`*** Update File: generate.py` patch，action contract 正确拒绝为 `apply_patch_unanchored_update:generate.py`。随后模型重复
`read_file generate.py`，duplicate-read recovery/hard-stop 正确阻止继续 context refresh，但 bounded hard-stop excerpt
没有把 unanchored patch rejection 放到足够高优先级，模型最需要修复的 patch grammar 语义被 generic repair contract 稀释。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-duplicate-read-after-unanchored-patch-feedback-loss` | action-contract feedback + session recovery composition + control loop | action contract 已拒绝 `apply_patch_unanchored_update`，但 duplicate-read recovery 只把 `apply_patch_mixed_native_unified` / `apply_patch_native_hunk_header` 归类为 patch grammar recovery；hard-stop 摘要可能只保留 generic repair contract，模型继续重复 read/context refresh | duplicate-read recovery 将 `Most recent failed edit feedback to preserve` 提到 previous blocked feedback 之前；patch grammar preservation/advisory 覆盖 `apply_patch_unanchored_update`；当前 required behavior 明确要求立即重发 native apply_patch，并禁止 `read_file/context refresh` 作为该失败的 recovery | `validation_rework_duplicate_read_recovery_preserves_unanchored_patch_feedback`; `validation_rework_duplicate_read`; `validation_rework`; `validation_`; `apply_patch_`; CoE E-095/H-044/E-096 |

边界说明：该 case 不是 `apply_patch` executor 没有失败，也不是失败语义完全没有传给 TaskSpace。失败语义已经由
action contract 产生，但 recovery/hard-stop 的排序和分类让它在下一轮 provider-visible feedback 中不够稳定。修复不放宽
patch grammar，也不让 malformed patch 进入 executor；它只保证 action-contract 的具体失败语义在后续 duplicate-read recovery
中保持最高优先级。

## 2026-07-04 R4-D issue type addendum: read_file completeness ambiguity

unanchored patch recovery 修复后的 keyed rerun 又暴露出成功 read 反馈语义不足：validation rework 已经得到精确
`NameError: projects_by_dept is not defined` traceback，并首次读取了 `generate_organization.py`，但工具输出只是
`sed -n '1,240p'` 的正文，没有结构化说明是否已经到达 EOF。模型随后连续以 `Need full file` 为理由重复读取同一文件，
最终被 duplicate-read hard stop 截断。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-read-file-completeness-ambiguity` | read_file tool feedback + ActionMap projection + duplicate-read recovery | 成功 read 的正文存在，但 provider-visible feedback 无法区分完整小文件与截断 first-window；模型把已读结果当成可能不完整，重复 `read_file` 而不是 patch | `read_file` 保持原有 `sed -n 1,240p` / `Get-Content -TotalCount 240` bounded read 行为，同时追加 `TaskSpaceReadFileSummaryV1`，包含 `lines_read`、`eof_reached`、`max_lines`；projection/recovery 保留该 summary，`eof_reached=true` 明确 no hidden lines，`eof_reached=false` 明确 bounded_read，不放宽重复读 | `action_contract_read_file_uses_host_platform_command`; `sed_read_command_artifact_ref_ignores_read_summary_suffix`; `working_evidence_excerpt_preserves_bounded_read_summary`; `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback`; CoE E-097/H-045/E-098 |

边界说明：该 case 不是 validation failure 丢失，也不是状态机允许重复读。失败 traceback 和目标文件内容都存在，runtime 也能
hard stop 重复读；缺口在成功 read 的“完整性”没有被结构化传回模型。修复不扩大 raw file window，不把长文件伪装成完整，
只让 bounded read 明确声明 `eof_reached=true/false`。

## 2026-07-04 R4-D issue type addendum: read summary awk portability

read completeness 修复后的首轮 keyed rerun 暴露命令构造的 portability bug：Unix `read_file` summary 阶段使用
`awk ... -- <path>`，在 benchmark 容器的 awk 实现中 `--` 被当作文件名，导致每次 read 在打印正文后以 exit 2 失败。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `read-file-summary-awk-double-dash-portability` | read_file command construction / tool capability layer | `sed` 正文输出成功，但 appended `awk -- <path>` 报 `awk: cannot open "--"`，使 read_file 成功语义变成失败结果 | 保留 `sed -n 1,240p -- <path>` 作为实际读取和 artifact parser 前缀；summary 命令改为 `awk <script> <path>`；parser 忽略 `&& awk ...` suffix 后仍解析原始 artifact | direct shell smoke; `action_contract_read_file_uses_host_platform_command`; `sed_read_command_artifact_ref_ignores_read_summary_suffix`; `validation_rework`; `validation_`; CoE E-099/H-046/E-100 |

边界说明：该 case 是工具能力层命令构造错误，不是模型行为问题，也不是 read completeness contract 本身错误。修复只移除
summary `awk` 的非 portable `--`，不改变 read 窗口、不改变状态机权限。

## 2026-07-04 R4-D issue type addendum: generic fact-source concrete artifact gap

schema rename hint 修复后的 keyed rerun 证明 schema validation 链路已能跑到真实 required-property errors，但也暴露出更早的
inspect coverage 缺口：`start_task` 的 `initial_fact_sources` 写成泛化目录描述，`initial_success_criteria` 却已经明确点名
`departments.csv`、`employees.csv`、`projects.csv`。runtime 只从 fact-source record 提取 concrete artifacts，导致 inspect
只读 `schema.json` 就允许进入 implementation，模型随后按猜测字段实现。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `generic-fact-source-success-criteria-artifact-gap` | inspect coverage / fact-source feedback layer | 具体输入文件存在于 success criteria，但泛化 fact-source 描述没有展开为 required reads；实现前未读 CSV，导致字段发明和后续 schema failure | required fact-source artifacts 同时从 explicit fact_sources 和 success criteria 中抽取；排除 output-contract targets 和 generated JSON output，避免把 `organization.json` 当作输入；projection/manual finish 在命名输入未读时不暴露 implement transition | `inspect_requires_success_criteria_artifacts_when_fact_source_is_generic_directory`; `inspect_` regression suite; CoE H-067/E-141/E-142 |

边界说明：该 case 是反馈层“语义缺失”，不是 validation repair 语义扭曲。runtime 不应该在 implementation node 成功 edit
之后再允许补做输入 inspect；正确做法是在 inspect 阶段把 success criteria 中的 concrete input artifacts 纳入硬 gate。

## 2026-07-04 R4-D issue type addendum: complete-read duplicate hard-stop timing

success-criteria fact-source 修复后的 keyed rerun 证明输入覆盖已经进入 live path，但 validation rework 仍在重复读控制上过早终止。
模型第一次重复读取已经完整可见的 target artifact 时，duplicate-read rejection 才第一次明确携带：
`complete read_file context`、`eof_reached=true`、`no additional file lines are hidden`、以及 repair contract。旧策略在这条强反馈生成后
立刻 hard-stop，本轮没有给 provider 响应该反馈并改为 `apply_patch` 的机会。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-complete-read-duplicate-hardstop-too-early` | validation rework duplicate-read feedback / control loop | 完整 target read 后第一次 duplicate-read 被立即 hard-stop；强反馈没有下一轮可执行机会 | complete-read duplicate-read recovery 可恢复一次；第二次仍重复或 gate 已报告 `repeated_blocked_action` 时 hard-stop；保持重复循环上限 | `validation_rework_duplicate_read_complete_context_gets_one_recovery_before_hard_stop`; `validation_rework_duplicate_read` 7/7; `validation_rework` 20/20; CoE H-068/E-143/E-144 |

边界说明：该 case 不是状态机放宽重复读，也不是要无限重试。runtime 仍拒绝重复读；区别是第一次拒绝产生的强语义需要被发送给
provider 一轮，随后仍不 patch 才终止。

## 2026-07-04 R4-D issue type addendum: validation rework block rejection wording drift

complete-read recovery 修复后的 keyed rerun 证明第一层 hard-stop timing 已清除，但暴露了下一层反馈语义漂移。
runtime 正确拒绝了 validation rework 中的 `"action":"blocked"`，错误理由是“还需要读 `schema.json` / test expectations 才能修
`process.py`”；runtime 返回的新句式是 `dependency evidence already identifies the implementation artifact or validation rework target`。
session 旧识别只匹配 `already recorded implementation source evidence`，导致这条正确 rejection 没被结构化为
`missing_source_visibility_blocker_rejected`，后续退回 patch-only hard-stop。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-block-rejection-wording-drift` | action-contract feedback classification / validation rework recovery | runtime 已拒绝 missing-source blocker，但 session 未识别新 wording，provider 没收到结构化 `block_node rejected -> apply_patch` 反馈 | old/new missing-source rejection wording 共用 recognizer；actionable output、recent-output progress hint、tool-output summary 全部复用 | CoE H-069/E-146; `action_contract_prompt_structures_validation_rework_missing_source_blocker_rejection` |

边界说明：这不是允许 block，也不是放宽 patch-only hard-stop；正确行为是保留 hard-stop 上限，但在 hard-stop 前先把 runtime 的
block rejection 以结构化反馈传给 provider。

## 2026-07-04 R4-D issue type addendum: validation rework patch directive buried after evidence

block-rejection wording 修复后的 keyed rerun 没有复现 block path，而是进入完整 schema validation rework。runtime 已给出：
完整 `process.py` read、schema/CSV 输入证据、`missing_required_properties`、`schema_required_groups`、`member_ids->members` rename hint
和禁止重复读。但 `TaskSpaceValidationReworkPatchOnlyRecoveryV1` 把 `Current required behavior` 放在长 evidence 之后，
provider 连续两次选择 `read_file process.py`，最终按设计 hard-stop。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-patch-directive-buried-after-evidence` | validation rework recovery payload layout / actionability | 正确事实齐全，但“现在必须 apply_patch，不要 read_file”的动作指令位于长证据块后，模型在 repair loop 中继续重复读 | patch-only 和 duplicate-read recovery 均先给 `Current required behavior`，再给 previous feedback 和 evidence；保持 repair contract、complete-read evidence 和 hard-stop 上限 | CoE H-070/E-149; recovery ordering tests |

边界说明：这不是删减证据，也不是放宽 repeated read；而是把动作优先级显式前置，让模型先看到合法下一步，再用下面证据构造 patch。

## 2026-07-04 R4-D unresolved issue type: closed action-space noncompliance

patch directive 前置后的 keyed rerun 证明动作指令已经 live 可见：hard-stop excerpt 中 `Current required behavior` 出现在长 evidence
前，active projection 也明确 `next_valid_actions` 是使用完整 read result、不要重复 read/search、对 `generate_organization.py`
执行 `apply_patch`。同时 current node contract 已写明 allowed action classes 为 `edit, control(...)`，且 read/search 会被 blocked。
模型仍输出 `read_file generate_organization.py`，runtime 只能阻断并 hard-stop。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-closed-action-space-noncompliance` | capability/control layer beyond advisory feedback | action space 已闭合且反馈已前置，模型仍选择非法 read_file；runtime 只会 reject/hard-stop，不能推动 patch 产生 | 已采用 action schema narrowing：当 runtime 确认 validation rework target read 已可见且无 successful edit 时，taskspace-action-v1 `read_file` 在转换成 shell read 前被拒绝为 `validation_rework_closed_action_space_read_disallowed:read_file`，并进入 patch-only recovery；real keyed rerun pending | CoE H-071/E-152/E-153; keyed rerun `20260704-153051-437`; focused tests |

边界说明：该 case 不是语义缺失，也不是语义扭曲；是语义已正确传递后，模型仍不服从闭合动作空间。本轮修复点在能力/控制层：
非法 read 不再先落到普通 shell 工具再由 runtime gate 拒绝，而是在 action-contract schema 转换前被挡住。

## 2026-07-04 R4-D issue type addendum: closed action rejection NoAction downgrade

closed action-space narrowing 的 live rerun 证明 schema/control 层已经能阻止非法 read 进入普通 shell 工具，但同时暴露了
feedback routing 语义降级：`validation_rework_closed_action_space_read_disallowed:read_file` 是 repair-actionability rejection，
session 却把它送入泛化 `TaskSpaceNoActionRecoveryV1`。结果 provider 没收到 patch-only contract，继续重复同一非法 read，
直到 provider-node hard stop。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-closed-action-rejection-noaction-downgrade` | session recovery classification / feedback routing | action-contract 已拒绝 closed target re-read，但 rejection marker 没被识别为 implementation-needs-edit；provider-visible recovery 降成 `TaskSpaceNoActionRecoveryV1`，丢失 `apply_patch` next-action 语义 | `validation_rework_closed_action_space_read_disallowed` 归入 implementation-needs-edit、recent-output patch-only progress hint 和 validation rework patch-only recovery；第一条 closed schema rejection 保留一次 patch-only recovery，第二条同类 rejection hard-stop | keyed rerun `20260704-154904-391`; CoE H-072/E-154/E-155; `implementation_recovery_selects_patch_only_after_closed_action_space_read_reject`; `validation_rework`; `action_contract_prompt` |

边界说明：该 case 是反馈层“语义降级”，不是底层工具失败语义缺失。runtime 已经知道 read 非法；问题是这个语义进入错误 recovery
通道后不再驱动 patch。修复保持 action space 闭合，不把 read 重新放开，只把 rejection 路由到正确的 patch-only recovery。

## 2026-07-04 R4-D unresolved issue type: post patch-only noncompliance

closed-action rejection routing 修复后的 live rerun 证明 `validation_rework_closed_action_space_read_disallowed` 已正确进入
`TaskSpaceValidationReworkPatchOnlyRecoveryV1`，并在第二次同类非法 read 后 bounded hard-stop。新的 blocker 是：模型在
complete target read、schema repair contract、patch-only recovery 和 closed-action rejection 都可见后，仍不生成 patch。

| Issue type | Layer | Symptom | Next design direction | Evidence |
|---|---|---|---|---|
| `validation-rework-post-patch-only-noncompliance` | repair synthesis / model routing / bounded control loop | repair evidence 已齐全且 action space 已闭合；provider 仍重复 `read_file process_csv.py`，最终 `TaskSpaceValidationReworkPatchOnlyHardStopV1` | 已先采用通用 repair-synthesis scaffold：patch-only recovery 将 `schema_property_rename_hints`、`missing_required_properties`、traceback/test signals 转成 patch construction steps，并明确 native apply_patch grammar；模型/profile escalation 和更强 patch-plan gate 保留为下一层候选 | keyed rerun `20260704-160458-158`; CoE H-073/E-156/E-157 |

边界说明：该 case 已越过“工具反馈是否正确传递”的狭义层面，进入 repair synthesis 策略层。R4-D 负责确保失败语义不丢失、不扭曲、
不降级；H-073 focused fix 不写任务特化 patcher，只把已存在的 repair evidence 结构化为通用 patch 构造步骤。

## 2026-07-04 R4-D issue type addendum: start_task output contract downgrade

repair-synthesis scaffold 修复后的 keyed rerun 没有命中 H-073 分支，而是在更早的 start-task contract 建立阶段暴露
contract downgrade。provider 的顶层 rationale 仍包含 `organization.json` / `schema.json` 任务目标，但
`taskspace_control.start_task` args 把 success criteria 和 output contracts 写成 inspect-style discovery summary。runtime
接受该弱 contract 后，后续 `python process.py` generator-only 成功被 validation closeout 解释成任务 validation success，
即使 public validator 仍失败。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `start-task-output-contract-downgrade` | capability/contract bootstrap before tool validation | provider-supplied `start_task` args 可把 objective-level generated output/schema obligations 降级成 schema/file summary；工具成功语义因此被错误解释为任务成功 | action-contract transport 将 top-level rationale 合并进 start_task objective；runtime 从 objective、success criteria、evidence refs 推导 generated JSON output targets 和 schema/validator targets，generator-only validation closeout 不再满足弱 contract | keyed rerun `20260704-161809-385`; CoE H-074/E-158/E-159; `taskspace_action_contract_preserves_start_task_rationale_as_objective`; `start_task_derives_output_contracts_from_objective_when_model_records_inspect_outputs` |

边界说明：这是 capability/contract 层的 feedback 前置问题。工具没有“失败但没传递”；相反，工具成功被过窄的 start-task
contract 重新解释成了错误成功。R4 修复点是让状态机恢复用户目标中的产物/schema 义务，而不是允许 runtime 越过状态机直接假设所有
JSON 生成任务都必须跑某个固定 validator。

## 2026-07-04 R4-D issue type addendum: validation rework static read exception conflicts with patch-only closure

attested keyed rerun 证明 output/schema contract enforcement 已生效，但 validation rework 仍在 closed read loop 中终止。该轮
`TaskSpaceValidationReworkPatchOnlyRecoveryV1` 已包含 patch construction scaffold，`generate_org.py` 也完整读取并标记
`eof_reached=true`。provider 仍输出 `read_file generate_org.py`，其中一个原因是静态 `TaskSpaceActionContractV1`
仍保留泛化例外：“named validation rework artifact 的 read_file 可以有效”。该例外与动态 patch-only closure 冲突。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-static-read-exception-conflicts-with-patch-only` | static action-contract instruction vs dynamic recovery contract | static contract 允许 named validation rework artifact read；dynamic recovery 在 complete target read 后关闭 read/search/schema inspection；provider 继续选择 closed read | 静态 implement rule 限定 named validation rework target read 只在 target 未完整读取前有效；当 state/projection/recent feedback 提到 patch-only after target read、complete_read/eof_reached=true 或 closed-action read rejection 时，只允许 apply_patch 或 block_node | keyed rerun `20260704-163615-799`; CoE H-075/E-160/E-161; `taskspace_static_contract_closes_complete_validation_rework_reads`; `validation_rework` |

边界说明：这不是放宽 runtime，也不是把第一次 validation rework target read 禁掉；第一次 target read 仍由现有测试覆盖为合法。修复仅消除
complete-read 之后的静态/动态 contract 冲突。

## 2026-07-04 R4-D issue type addendum: failed-edit refresh reopens complete validation rework read

static read exception 修复后的 attested keyed rerun 进入了更深一层：provider 不再停在重复 read，而是在 complete target read
之后发出了 `apply_patch`。但 patch hunk verification failed 后，projection 把 same-target refresh-read 作为 failed-edit 后的合法
恢复动作重新开放。该例外对 truncated/stale read 是合理的；对 `TaskSpaceReadFileSummaryV1 eof_reached=true` 的完整 read
不合理，因为 runtime 已确认没有隐藏行。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-failed-edit-refresh-reopens-complete-read` | validation rework failed-edit recovery / projection-gate consistency | complete target read 后 patch 失败，projection 暴露 same-target refresh read；closed action gate 又拒绝 read，provider 在相互冲突的反馈中继续 retry | refresh-read 例外必须同时满足“失败 edit 在该 read 之后发生”以及“该 read 未证明 `eof_reached=true`”；完整 read 后 duplicate target read 仍拒绝，并反馈 `complete read_file context`、`eof_reached=true`、`apply_patch` | keyed rerun `20260704-164819-131`; CoE H-076/E-162/E-163; `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback`; `validation_rework`; `action_contract_prompt` |

边界说明：这是反馈/控制投影的语义边界缺失，不是语义扭曲。runtime 对 repeated read 的拒绝仍是正确的；错误在于上游 projection
把完整读取后的 failed patch 当成可 refresh 的截断/陈旧上下文处理。修复不取消 failed-edit refresh，只把它限制在未完整读取的 target
read 上。

## 2026-07-04 R4-D issue type addendum: partial-excerpt blocker wording drift

failed-edit refresh 修复后的 keyed rerun 进入 patch grammar recovery，但随后暴露新的 blocker wording 漂移。provider 在
`apply_patch_mixed_native_unified` 之后没有按 native grammar 重发 patch，而是 block：`Insufficient file content visibility`、
`only partial excerpt`、`full content is needed`、`ability to read the full file`。这些都是 missing-source blocker 的同义表达；
但 recognizer 旧词表未覆盖，runtime 接受 blocker，关闭了 repairable validation rework node。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-partial-excerpt-blocker-wording-drift` | runtime blocker classification / validation rework feedback continuity | complete target read + failed/malformed patch 后，partial-excerpt/full-content blocker 被接受，current node 关闭，后续请求退化为 `provider-context-missing` 和错误 final blocker | missing-source blocker recognizer 覆盖 partial-excerpt/full-content/read-full-file wording；complete target read 时 rejection 明确使用 existing complete evidence retry `apply_patch`，不 refresh read | keyed rerun `20260704-170158-193`; CoE H-077/E-164/E-165; `validation_rework_allows_changed_artifact_read_when_schema_failure_lacks_traceback`; `validation_rework`; `action_contract_prompt` |

边界说明：这不是 CSV/schema 真的缺失，也不是允许模型 block。正确行为是保持 validation rework node active，把该 blocker 转回
patch-only recovery，直到生成合法 patch、真实外部 blocker，或达到 bounded hard-stop。

## 2026-07-04 R4-D issue type addendum: repeated malformed patch hunks after complete read

partial-excerpt blocker 修复后的 keyed rerun 进入更深 patch recovery：当前 node 没被错误关闭，但 provider 连续输出 fragile
`Update File` hunks，多次因 expected-lines mismatch 失败，随后继续混合 native wrapper 和 unified/range hunk syntax，最后触发
`apply_patch_mixed_native_unified:process.py` 与 node request hard-stop。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-repeated-malformed-patch-hunks-after-complete-read` | apply_patch capability normalization / edit-failure recovery actionability | complete target read 后 repeated expected-lines mismatch 仍回到 fragile update hunks；live malformed wrapper 未被 normalization 覆盖；预算耗尽前没有收敛到合法 patch | complete target read + expected-lines/context mismatch 强制整文件 rewrite contract：`*** Delete File` + `*** Add File`；normalizer 跳过 misplaced `*** Begin Patch`，支持 `*** Update File` 在 wrapper 前的 live 变体 | keyed rerun `20260704-171735-273`; CoE H-078/E-166/E-167; `complete_validation_rework_expected_lines_failure_forces_full_rewrite`; `taskspace_action_contract_normalizes_misordered_begin_update_mixed_patch`; `apply_patch --lib` |

边界说明：这不是要放宽 patch grammar，也不是无限增加预算。修复仍要求 native apply_patch，只是在完整目标文件已可见且 context hunk 多次失败时，
把合法下一步从“再试一个 hunk”升级为“整文件替换”，并补齐 live wrapper 正规化能力。

## 2026-07-04 R4-D issue type addendum: patch-only schema synthesis too weak

full-rewrite recovery 修复后的 keyed rerun 没有再进入 repeated malformed patch hard-stop，但暴露了更直接的反馈层缺口：
validation 已经明确列出缺失字段，`generate_organization.py` 也完整读取，runtime 正确关闭 read/search action space；provider
仍以“需要完整内容”为理由重复 `read_file generate_organization.py`。这说明闭合语义正确，但 repair synthesis 没把缺失字段和
rename hints 提升为足够可执行的 patch 任务。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-patch-only-schema-synthesis-too-weak` | validation rework feedback / repair synthesis | complete target read + schema repair contract 已齐全，patch-only recovery 仍只泛化提示“用证据 patch”；provider 重复 closed `read_file` 并 hard-stop | patch-only recovery 在 generic scaffold 前加入 `Schema repair synthesis from current validation failure`：列出 exact `missing_required_properties`、exact `schema_property_rename_hints`，要求按 schema spelling 生成输出字段，并明确这不是再次读取理由 | keyed rerun `20260704-173608-346`; CoE H-079/E-168/E-169; `implementation_recovery_selects_patch_only_after_target_read_evidence`; `implementation_recovery_selects_patch_only_after_closed_action_space_read_reject` |

边界说明：这不是 permission 层问题，也不是 `read_file` 没有被挡住。runtime 已正确拒绝
`validation_rework_closed_action_space_read_disallowed:read_file` 并 bounded hard-stop；修复点是让反馈层把 schema 失败语义转成
可执行 patch plan，降低模型继续 discovery 的概率。

## 2026-07-04 R4-D issue type addendum: missing fact-source bootstrap does not transition

schema repair synthesis 修复后的 keyed rerun 没有到达 validation rework，而是在更早的 inspect 阶段暴露 phase-control 缺口：
provider 重复 `list_files`，runtime 已执行 `TaskSpaceMissingFactSourceBootstrapV1` 读取剩余声明 fact-source，并进一步执行 bounded
json/csv/yaml bootstrap，但没有把 inspect node 强制结束进入 implementation。模型继续 inspect，最终触发 inspect node request hard-stop。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `inspect-missing-fact-source-bootstrap-no-transition` | inspect bootstrap / phase-control bridge | missing fact-source bootstrap 已补齐事实源，但 session/runtime 把控制权还给 stuck inspect model，最终 `provider_node_request_hard_limit_exceeded node_request_count=5/5` | missing fact-source bootstrap 后若 required fact-source coverage 已清空，立即以 `inspect_missing_fact_source_bootstrap_complete` 触发 forced inspect transition；runtime 接受该 trigger 并插入 implementation node | keyed rerun `20260704-174618-510`; CoE H-080/E-170/E-171; `inspect_missing_fact_source_bootstrap_complete_forces_transition_after_coverage`; `inspect_missing_fact_sources`; duplicate transition regression |

边界说明：这不是要跳过 fact-source guard。缺失事实源未读完时，既有 guard 仍阻止 forced finish；修复只覆盖 bootstrap 已把缺口补齐后的
bridge，不再让模型继续消耗 inspect node budget。

## 2026-07-04 R4-D issue type addendum: bootstrap read classification and hard-stop convergence

missing fact-source bootstrap transition 修复后的 keyed rerun 证明同一 inspect path 仍有两个相邻 tools 链路缺口。第一，内部
bootstrap read 命令由于 awk summary 中含 `>` 被 shell action classifier 误判为 edit，导致 read-only inspect gate 拦截；第二，
当模型后续手动完成全部 fact-source 读取后，runtime 只把 `finish_node` 暴露为 next valid action，仍允许下一次 provider request
走到 node hard-stop，而不是在 pre-dispatch 阶段强制 transition。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `inspect-bootstrap-read-classification-and-hard-stop-transition-gap` | capability action classification / inspect feedback-control bridge | `TaskSpaceMissingFactSourceBootstrapV1` 的 read-only shell command 被判为 edit；完整 fact-source evidence 已存在时仍 `provider_node_request_hard_limit_exceeded node_request_count=10/10` | Unix bounded read summary 不再生成 `>`，真实 bootstrap command 分类为 `ActionClass::Read`；provider pre-dispatch hard-stop 前若 inspect progress ready，以 `inspect_hard_stop_progress_convergence` forced transition 到 implementation | keyed rerun `20260704-175447-182`; CoE H-081/E-172/E-173; `missing_fact_source_bootstrap_command_reads_bounded_declared_artifacts`; `shell_action_classifier_identifies_core_taskspace_classes`; `inspect_hard_stop_progress_convergence_forces_transition_after_coverage` |

边界说明：该修复不削弱 read-only gate。误判修复只让内部 bounded read 留在 read class；hard-stop bridge 仍依赖 inspect progress
ready，missing fact-source 或 unread referenced script 未完成时不会 forced finish。

## 2026-07-04 R4-D issue type addendum: validation required-command advisory loop

inspect bridge 修复后的 keyed rerun 进入 validation 阶段并暴露新的 feedback/control 缺口。validation gate 已经拒绝了
generator-only command，并在 `TaskSpaceGateRecoveryV1.next_valid_actions` 中给出 exact combined command：
`python generate_organization.py && python -m jsonschema -i organization.json schema.json`。`TaskSpaceValidationNeedsTestRecoveryV1`
也保留了这条命令，但它只是 developer guidance。provider 随后继续选择不可用 pytest 和 generator-only 命令，runtime 反复拒绝，
最后耗尽全局 request budget。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-output-contract-next-action-advisory-loop` | validation feedback / session runtime bridge | exact `run_test with command ...` 已存在，但只作为 advisory 返回模型；模型忽略后反复尝试不可用 pytest 或 generator-only validation，最终 hard-stop | 对 changed-artifact/output-contract coverage gate 的 exact next action，session 在同轮 runtime 中执行该命令，并把结果记录为 `ActionClass::Test`；trace 暴露 `TaskSpaceValidationRequiredCommandBootstrapV1`；普通失败或 unrelated gate 不自动执行 | keyed rerun `20260704-180719-471`; CoE H-082/E-174/E-175; `validation_required_command_bridge`; `validation_needs_test`; `validation_output_contract`; `action_contract_prompt`; `validation_rework` |

边界说明：这是反馈层“正确语义未进入控制语义”的缺口，不是 validation gate 缺失。修复不会允许 generator-only command
绕过 output contract，也不会根据文件名猜测测试；只执行 gate 自己产出的 exact legal command。

## 2026-07-04 R4-D issue type addendum: validation required-command bridge one-hop only

validation required-command bridge 的首次 keyed rerun 证明 bridge 已进入 runtime，但 staged gate 链仍有缺口。changed-artifact
coverage gate 可以先给 `python transform.py`；执行后 output-contract gate 再给更严格的
`python transform.py && python -m jsonschema -i organization.json schema.json`。旧 bridge 只执行第一跳，并把第二跳 gate rejection
当作 failed Test，导致 rework 从内部 gate rejection 而非真实 validator result 开始。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-required-command-bridge-one-hop-only` | validation runtime bridge / gate-to-gate command chain | bridge 执行 first-hop changed-artifact command 后不追 nested output-contract command；中间 gate rejection 被记录为 final Test failure | bridge 在固定 3-hop 上限内追 changed-artifact/output-contract gate 产出的 stricter exact command；中间 gate output 留在 transcript，但只把最终命令结果写回 `ActionClass::Test` | keyed rerun `20260704-182700-317`; CoE H-083/E-176/E-177; `validation_required_command_bridge`; `validation_needs_test`; `action_contract_prompt`; `validation_rework` |

边界说明：链式 bridge 只追 TaskSpace gate 自己产出的 exact command，且要求新命令不同于上一条；不会对普通 shell/test
失败做自动重试，也不会形成无限循环。

## 2026-07-04 R4-D issue type addendum: patch-only recovery tail truncation drift

链式 validation bridge 修复后的 keyed rerun 进入真实 validation rework。runtime 已完整读取 `process.py`
（`eof_reached=true`），并发出 patch-only recovery；但长 evidence 尾部仍有压缩预览，provider 以 `current projection truncated`
为理由再次 `read_file process.py`，最后触发 patch-only hard-stop。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-patch-only-tail-truncation-drift` | validation rework recovery payload layout | recovery 顶部说 complete/eof、apply_patch only，但长证据尾部的 truncated preview 诱导 provider 再读 target | patch-only recovery 在 evidence 后追加 `Final action lock`，明确 projection truncation 不是 complete target 的重读理由；下一步只能 apply_patch 或 block_node | keyed rerun `20260704-183656-438`; CoE H-084/E-178/E-179; `implementation_recovery_selects_patch_only_after_target_read_evidence`; `validation_rework`; `action_contract_prompt` |

边界说明：这不是放宽 repeated-read gate。runtime 继续拒绝 complete target re-read；修复只让反馈尾部也保持同一动作语义。

## 2026-07-04 R4-D issue type addendum: failed-edit fragile patch fallback after complete read

tail action lock 修复后的 keyed rerun 进入 apply_patch，但 patch 使用 mixed native/unified/ranged hunks 并因 expected-lines 失败；
failed edit 后 provider 又以 projection excerpt insufficient 为理由 refresh read。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-failed-edit-fragile-patch-fallback` | validation rework failed-edit recovery | complete target read 后 failed patch 仍回到 fragile hunk/read refresh | patch-only final action lock 在 expected-lines/context/mixed-hunk 失败后升级为 whole-file native replacement：`*** Delete File` + `*** Add File`；继续禁止 read/search | keyed rerun `20260704-184541-992`; CoE H-085/E-180/E-181; focused test; `validation_rework` |

## 2026-07-04 R4-D issue type addendum: schema-context blocker after patch-only recovery

failed-edit tail lock 修复后的 keyed rerun 进入 complete target read + patch-only recovery，但 provider block 为需要
`schema.json` 全量内容和 schema context，且称 current projection excerpt 不足。runtime 旧词表只覆盖一部分 `.py`/file content
表达，没覆盖 schema/output-structure/full-content/projection-excerpt-insufficient wording，于是把可恢复缺源语义当作真 blocker。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-schema-context-blocker-after-patch-only` | validation rework blocker classification / feedback continuity | complete target read + repair contract 已存在时，full schema/schema context/projection excerpt insufficient blocker 被接受，node 关闭，后续扭曲为 local infra blocker | missing-source blocker recognizer 覆盖 schema/output-structure/full-content/projection-excerpt-insufficient wording；complete target read 时 rejection 明确保留 `complete_read/eof_reached=true` 和 `apply_patch` | keyed rerun `20260704-190021-739`; CoE H-086/E-182/E-183; `validation_rework_rejects_missing_current_artifact_visibility_blocker`; `validation_rework`; `action_contract_prompt` |

边界说明：这里修的是反馈层同义词漏判，不是让 runtime 忽略真实外部 blocker。只要 blocker 指向的是“需要再看 schema/完整内容/投影不够”，且
validation rework 已有完整 target read 与 repair contract，就必须保持节点 active 并把反馈导回 patch-only。

## 2026-07-04 R4-D issue type addendum: repeated duplicate list_files no bootstrap transition

schema-context blocker 修复后的 keyed rerun 没有到 validation rework，而是在 inspect node 重复 `list_files`。第一次
`rg --files .` 已成功返回 schema/csv 文件清单；后续重复被 duplicate read/search gate 拦截，但 feedback 没进入 bootstrap control，
最终 inspect node request hard-stop。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `inspect-duplicate-list-files-no-bootstrap-transition` | inspect duplicate read/search feedback-control bridge | successful `list_files` 后重复同一 list/search，只收到 advisory duplicate recovery；没有 bounded content bootstrap，也没有 forced transition | repeated duplicate read/search 执行 `TaskSpaceRepeatedBlockedInspectBootstrapV1` 并把输出写入 ActionMap；`=====` sectioned schema/csv content 计为 input-data working evidence；bootstrap complete 后 forced transition | keyed rerun `20260704-191110-654`; CoE H-087/E-184/E-185; `inspect_duplicate_list_files_bootstrap_forces_transition_after_data_reads`; `inspect_bootstrap`; `forced_inspect_transition`; `inspect_missing_fact_sources` |

边界说明：路径列表仍只是 discovery，不直接允许 implementation；只有 bootstrap 读取到具体文件内容并通过 working-evidence 判定后，才会
让 inspect 进入 implementation。

## 2026-07-04 R4-D issue type addendum: validation rework recovery counter cross-node leak

duplicate list_files bootstrap 修复后的 keyed rerun 已进入 validation rework。新的 failure 不是工具失败、不是 validation bridge
失败，也不是 patch-only 语义缺失；它是 runtime feedback escalation 的状态作用域错误。`node-4` 已经消耗两次
patch-only recovery，后续新的 `node-6` 第一次完整读取 `processor.py` 后直接被算作第 3 次并 hard-stop。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-recovery-counter-cross-node-leak` | validation rework recovery escalation / feedback-control lifecycle | 新 validation rework node 继承旧 node 的 patch-only/duplicate-read recovery count，首次 target read 后可能直接 hard-stop | validation rework duplicate-read 与 patch-only recovery counters 以当前 provider snapshot `node_id` 为 key；换 node reset，同 node 重复违规继续 hard-stop | keyed rerun `20260704-192256-883`; CoE H-088/E-186/E-187; `validation_rework_recovery_count_resets_when_rework_node_changes`; `validation_rework` 26/26 |

边界说明：该修复不降低 repeated-read 约束。它只把 escalation lifecycle 从 turn-global 改成 node-scoped，确保新 rework
node 先收到正确 patch-only recovery，再按同节点重复违规升级。

## 2026-07-04 R4-D issue type addendum: apply_patch recovery budget drain after validation rework

H-088 修复后的 keyed rerun 已经越过 validation rework counter leak，进入真实 patch 失败路径。runtime 能保留
`TaskSpaceEditFailureRecoveryV1`、拒绝 closed read，并识别 `apply_patch_unanchored_update`，但 repeated recovery 没有自己的
hard-stop escalation，最终被 generic provider node budget 截断。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `apply-patch-recovery-budget-drain-after-validation-rework` | apply_patch/edit failure recovery escalation / feedback-control lifecycle | 同一 node 内多次 edit failure / malformed patch recovery 后继续 provider sampling，最终 `TaskSpaceProviderBudgetHardStopV1` | 新增 node-scoped apply_patch recovery counter；`TaskSpaceEditFailureRecoveryV1`、patch format/missing target/unanchored/native-hunk/intent recovery 第 4 次触发 `TaskSpaceApplyPatchRecoveryHardStopV1` | keyed rerun `20260704-193906-178`; CoE H-089/E-188/E-189; `apply_patch_recovery_hard_stops_after_repeated_same_node_failures`; `apply_patch_` 36/36 |

边界说明：该 hard-stop 是反馈层可审计收敛，不代表 patch 已修好；它防止具体 patch feedback 被 generic provider budget
hard-stop 掩盖，为后续继续优化 patch quality / rewrite strategy 留出清晰边界。

## 2026-07-04 R4-D issue type addendum: whole Python Update File replacement normalization

apply_patch recovery hard-stop 修复后的 keyed rerun 显示 provider 的下一层常见 patch intent：用 `*** Update File` 包住完整
Python 文件正文，实际想做 whole-file replacement。旧 action contract 只能拒绝为 unanchored update。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `apply-patch-whole-python-update-replacement-normalization-gap` | apply_patch capability normalization / action-contract grammar | `*** Update File: <python>` 下直接跟完整 Python source，反复被拒绝为 unanchored update 并进入 patch recovery hard-stop | 单一 Python target、无 hunk/diff/change marker、内容像 Python source 时 normalize 为 `*** Delete File` + `*** Add File`; command payload 仍拒绝 | keyed rerun `20260704-195220-438`; CoE H-090/E-190/E-191; `taskspace_action_contract_normalizes_whole_python_update_replacement`; `taskspace_action_contract_rejects_non_diff_update_payload` |

边界说明：只转换明显源码整文件替换；不会把 `python3 -c`、shell command、JSON transformation command 或任意无差异文本作为
apply_patch 执行。

## 2026-07-04 R4-D issue type addendum: validation schema feedback chain

whole Python replacement normalization 后，`organization-json-generator` 继续暴露 schema validation rework 的反馈链条问题。
这组 case 的共同点是：底层工具或 runtime 往往已经有信号，但信号进入 provider-visible recovery 时缺少字段、顺序、可见性或闭合动作。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-schema-repair-rename-hint-gap` | validation semantic summary / feedback extraction | jsonschema 报告缺 required fields，同时 raw output 暴露旧 key；feedback 只列 missing required properties，未给 rename hints | validation summary 从 offending object keys 推导 `schema_property_rename_hints`，和 missing required properties 一起进入 rework feedback | keyed rerun `20260704-201836-345`; CoE H-091/E-192; `a93391e` |
| `validation-rework-target-read-evidence-order` | ActionMap working evidence ordering | complete target read 已存在，但排在长 validation failure 后，next action 语义被稀释 | `current_main_working_evidence_summary()` 优先输出 validation rework target read，再输出长 validation failure | CoE H-092/E-193; `697ec6c`; focused tests |
| `validation-rework-complete-target-replacement-scaffold` | validation rework patch-only recovery payload | complete target read 后 recovery 仍偏向 narrow `Update File` grammar，模型可继续 discovery | complete target read 时 patch-only recovery 直接提供 whole-file replacement scaffold，允许 native `Delete File + Add File` | keyed rerun `20260704-205001-147`; CoE H-093/E-194; `7c7c892` |
| `validation-rework-complete-read-content-visibility` | target-read evidence projection / recovery truthfulness | runtime 标记 complete/eof，但 provider-visible 内容只是 compact excerpt；模型认为 full content 不可见 | rework target read evidence 增加 `content_visibility`，只有 `full_content_visible` 时才强制 full replacement；summary-only 不伪装成 full context | keyed rerun `20260704-210512-809`; CoE H-094/E-195; `44938a3` |
| `validation-rework-full-visible-patch-mismatch-recovery` | failed edit recovery / apply_patch recovery closure | full-visible target 的 expected-lines/context mismatch 后，recovery 仍允许 read refresh 或 fragile `Update File` hunk | full-visible mismatch 后 recovery replacement-only：`Delete File + Add File`，禁止 read/search/validation、`Update File` 和 placeholder hunk | CoE H-095/E-196; `dde7173`; real rerun pending |

边界说明：这不是把控制权从状态机交给 runtime。状态机仍定义 action legality；runtime/session 只负责把已发生的 tool
result、validation failure、target read 和 edit failure 转成下一轮 provider 必须看到的强语义。若语义已完整且动作空间已闭合，
runtime 可以拒绝重复 read/search 并进入 hard-stop；但不能凭空制造未执行的 edit 或把真实外部 blocker 忽略为成功。

## 2026-07-04 R4-D issue type addendum: final gate rejection reason loss

`20260704-212411-195` 的真实 run 已越过本地 schema validation，但 final response 阶段暴露 session feedback 丢语义：
`record_main_final_response()` 已返回具体 final readiness rejection reason，`Session` 也把详细 developer message 写入历史，
但 `turn.rs` 的 actionability path 用 `.is_err()` 将其压成 boolean，`last_agent_message` 只剩固定泛化句子。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `final-answer-gate-rejection-reason-loss` | session final-response feedback / provider actionability | final_answer 被 gate 拒绝后，下一轮不知道具体是 hidden orchestration wording、criteria 未满足还是 validation-after-edit 缺失；provider 转而编造 validator unavailable blocker | `taskspace-action-v1 final_answer` 和普通 assistant final-response 路径都保留 `Err(error)`；follow-up 文案包含 `Rejection reason: ...` 并要求修正具体原因后再 final_answer | keyed rerun `20260704-212411-195`; CoE H-096/E-197/E-198; focused test; `action_contract_prompt`; `final_readiness` |

边界说明：该修复不改变 final readiness gate 的判定，也不放宽 success criteria。它只保证 gate 已经产生的失败语义进入下一轮
provider-visible recovery，避免从“最终回答文案/证据 gate 不满足”扭曲成“工具不可用”。

## 2026-07-04 R4-D issue type addendum: duplicate empty Update File wrapper

`b5f2ee2` 后的 keyed rerun 没有到达 final gate，而是在 validation rework patch grammar 层停住。新增 live payload
形态是：缺少 `*** Begin Patch`，先给一个空的 `*** Update File: <path>` section，随后重复同一路径的 `*** Update File`
并夹 unified headers/hunk。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `apply-patch-duplicate-empty-update-wrapper-normalization-gap` | apply_patch capability normalization | 空 `Update File` wrapper 被包进 native patch，最终 apply_patch 报 `Update file hunk ... is empty`，消耗 recovery budget 到 hard-stop | normalizer 只删除 empty same-target duplicate `Update File` section；真实 hunk 继续走 unified/native hunk normalization 和后续 rejection checks | keyed rerun `20260704-213755-290`; CoE H-097/E-199/E-200; `duplicate_unwrapped_update_wrapper`; `apply_patch_` |

边界说明：该修复不接受任意空 patch，也不忽略目标文件。只有“当前 section 没有内容，且下一个非空 section 是同一路径的
`*** Update File`”才会折叠。

## 2026-07-04 R4-D issue type addendum: no-action recovery budget drain

`af95784` 后的 keyed rerun 未命中 duplicate wrapper，而是暴露 session recovery lifecycle 的另一类反馈层问题：
runtime 已经识别 provider 没有产生有效 TaskSpace progress，并多次插入 `TaskSpaceNoActionRecoveryV1`，但该 recovery
只有 advisory 阈值，没有专用 terminal marker。结果最终 failure 被 generic provider budget hard-stop 覆盖。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `no-action-recovery-budget-drain` | no-action recovery escalation / feedback-control lifecycle | `TaskSpaceNoActionRecoveryV1` 超过 advisory threshold 后仍继续 provider sampling，最后表现为 `TaskSpaceProviderBudgetHardStopV1` | no-action recovery 改为 snapshot-node-scoped counter；超过 node-kind cap 后记录 `TaskSpaceNoActionRecoveryHardStopV1` 并停止本 turn；hard-stop 保留上一条 recovery excerpt 但不再被分类为普通 no-action recovery | keyed rerun `20260704-214746-740`; CoE H-098/E-201/E-202; `no_action_recovery`; `action_contract_prompt`; `apply_patch_` |

边界说明：该修复不改变工具权限和状态机合法动作集合。它只把“provider 在 recovery 后仍无有效动作”的失败语义闭合在反馈层，
避免被 provider budget 语义掩盖；下一 turn 只有 TaskSpace state 改变或 provider 发出 tool/control/block-with-evidence 才能继续推进。

## 2026-07-04 R4-D issue type addendum: natural-language slash fact-source extraction

`3b6b269` 后的 keyed rerun 没有触发 no-action recovery；新的 failure 发生在 inspect fact-source coverage。runtime
已经读取了 `schema.json`、`departments.csv`、`employees.csv`、`projects.csv`，但 success criterion 中的自然语言
`departments with employees/projects` 被误识别成 artifact `employees/projects`，导致 inspect node 无法 finish。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `inspect-natural-language-slash-fact-source-false-positive` | inspect fact-source artifact extraction / coverage gate | 已读完真实 CSV/schema 后仍要求读取不存在的 `employees/projects`，最终 inspect node provider budget hard-stop | 文件扩展名 token 继续算 artifact；无扩展 slash token 只有像真实路径/目录时才算 artifact；`employees/projects` 这类关系词不进入 required fact-source coverage | keyed rerun `20260704-215805-102`; CoE H-099/E-203/E-204; `natural_language_slash`; `inspect_missing_fact_source` |

边界说明：该修复不降低“必须读取真实 fact sources 后才能进入 implementation”的约束。`schema.json`、`departments.csv`、
`employees.csv`、`projects.csv` 仍会被要求；只移除自然语言关系词造成的假路径。

## 2026-07-04 R4-D issue type addendum: required schema validator feedback distortion

`439b4e1` 后的 keyed rerun live-clear 了 separator update section 和 trailing quote 问题，但暴露 validation
feedback 的另一类语义扭曲：coverage-correct required command 已经是
`node process.js && python -m jsonschema -i organization.json schema.json`，raw shell classifier 却把它归为
`unknown`，导致 smoke_test gate 拒绝执行；随后 provider 将已读 `schema.json` 和已命名的 schema validator
扭成 `JSON schema validator tool is unavailable`。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `required-schema-validator-command-classification-and-stale-blocker` | tool action classification / validation rework blocker semantics | schema validator command 被 `smoke_test does not allow unknown` 拦截；rework block 声称未读 schema 或 validator unavailable，即使 schema/rework evidence 已存在 | `python -m jsonschema`/`python3 -m jsonschema`/`py -m jsonschema` 分类为 test；validation rework 中 stale schema visibility 和 validator unavailable blocker 被拒绝并要求 `apply_patch` | keyed rerun `20260704-223925-994`; CoE H-102/E-209/E-210; `shell_action_classifier_identifies_core_taskspace_classes`; `validation_rework_rejects_stale_schema_and_validator_unavailable_blockers`; `validation_rework` |

边界说明：该修复不把所有 `node ... && python ...` 都归为 test，只识别明确 schema validator invocation；也不否认真实
validator infrastructure failure。它只防止在已有 schema/rework evidence 的情况下，把可编辑 schema failure 重新解释成“未读
schema”或“validator 不可用”。

## 2026-07-04 R4-D issue type addendum: forced inspect bridge fact-source evidence

`9ebb998` 后的 keyed rerun 已越过 H-102，但在更早的 forced inspect transition 后停住。runtime 已接受 inspect
bridge result，且 bridge body 包含 schema/CSV evidence；implement node 仍接受“需要读取 schema/csv”的 blocker，
随后因无 active node 进入 `TaskSpaceNoActionRecoveryHardStopV1`。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `forced-inspect-bridge-fact-source-evidence-gap` | forced inspect transition / blocker evidence bridge | accepted bridge result 包含 `schema.json`、CSV 内容，但 implementation blocker 仍声称 context projection 不含这些文件 | forced inspect bridge 的 inline `artifacts=...` 进入 visible artifact refs；implementation missing-source blocker guard 计入 dependency fact-source evidence，并拒绝 stale reread blocker | keyed rerun `20260704-225618-467`; CoE H-103/E-211/E-212; `forced_inspect_transition_rejects_missing_fact_source_blocker`; `missing_source_blocker`; `action_map::runtime::tests::inspect` |

边界说明：该修复不允许 implementation 在没有 inspect 证据时跳过读取。只有 dependency inspect 或 forced-transition
bridge 已有真实 fact-source artifact 内容时，才阻止 provider 把“已读事实源”重新包装成 blocker。

## 2026-07-04 R4-D issue type addendum: failed apply_patch recovery semantics

`b2ec9b0` 后的 keyed rerun 已 live-clear forced inspect bridge stale blocker，但 validation rework 进入新的
`apply_patch` recovery failure：工具失败语义确实传回了模型，不过 recovery 文案没有强制保留失败类型、修正目标路径和下一步动作，
导致模型连续提交 mixed native/unified、expected-lines mismatch、`app/app/process.py` 目标错误，最终进入
`TaskSpaceApplyPatchRecoveryHardStopV1`。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `failed-apply-patch-recovery-critical-semantics` | apply_patch feedback recovery / path normalization | apply_patch failure 已返回，但 provider 反复重试同类错误 patch：`--- a/...`/`+++ b/...` 混入 native update、expected-lines hunk 失配、`app/app/process.py` 双 app-root | edit-failure recovery 输出 `failure_kind`、`failed_target`、`mandatory_next_action`；`b/app/...` 这类 benchmark header 在 app cwd 下规范到真实相对路径；native hunk recovery 显式禁止 unified markers | keyed rerun `20260704-230322-342`; CoE H-104/E-213/E-214; `edit_failure_recovery`; `taskspace_apply_patch`; `action_contract_prompt` |

边界说明：该修复不放宽状态机，不把失败 patch 当成功，也不静默吞掉工具错误。它只把已存在的失败反馈转成 provider
下一轮必须遵守的结构化恢复合同，并修正 terminal-bench app-root 路径前缀。

## 2026-07-04 R4-D issue type addendum: validation rework expected-lines target pollution

`4e897ff` 后的 keyed rerun 证明 H-104 部分 live-clear：`app/app/process.py` 路径漂移消失，结构化 failed-edit
contract 已进入 provider-visible rollout。但新 run 仍在 validation rework 的 repeated `apply_patch` expected-lines
failure 上 hard-stop。关键差异是：失败语义已经存在，后续恢复链在解析失败目标和 patch-only artifact 时发生污染。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-expected-lines-target-pollution` | apply_patch feedback recovery / validation rework target extraction | flattened expected-lines message 把 `generate.py: total_projects = ...` 当成 failed target；patch-only target list 被 schema/CSV refs 污染；mixed native/unified headers 过晚进入工具层 | expected-lines/context/missing-target parser 在源码扩展名处截断目标；显式 `target_artifacts` 优先于 validation artifact refs；native `*** Update File` 中出现 `--- a/...`/`+++ b/...` 时在 action contract 层拒绝为 `apply_patch_mixed_native_unified:<target>` | keyed rerun `20260704-231827-396`; CoE H-105/E-215/E-216; `validation_rework_patch_only_prefers_explicit_target_artifacts`; `mixed_native_unified`; `expected_lines_target`; `taskspace_apply_patch`; `action_contract_prompt` |

边界说明：该修复不提升模型 patch 能力本身，也不把 failed patch 当成功。它只保证 feedback 层把同一个失败事实继续以正确目标、
正确 artifact 集合和正确 grammar 错误类别传回模型，避免恢复链把 `generate.py` 扭成带源码片段的伪路径，或把 schema/CSV
输入误当成可 patch 目标。

## 2026-07-04 R4-D issue type addendum: validation rework stale schema-knowledge blocker wording

`42d9777` 后的 keyed rerun 已证明 mixed native/unified patch 在 action-contract 层提前拒绝，且 patch-only recovery
target 保持为 `process.py`。新的 failure 是 blocker 语义覆盖不足：validation rework 已有 schema/CSV evidence 和
`process.py` complete target read，provider 仍用 `Cannot apply a valid patch without knowing the schema definition`
这类等价说法关闭 rework。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-stale-schema-knowledge-blocker` | validation rework blocker semantics / feedback predicate coverage | 已有 schema/fact-source evidence 和 complete target read 后，`without knowing schema` 这类 blocker 绕过 missing-source guard，被接受后触发 patch-only hard-stop | missing-source/schema blocker predicate 覆盖 `without knowing`、`without schema knowledge`、`lack/lacking schema knowledge`、`need schema definition`；complete target read 后拒绝该 blocker 并要求 `apply_patch` | keyed rerun `20260704-233803-895`; CoE H-106/E-217/E-218; `validation_rework_rejects_missing_current_artifact_visibility_blocker`; `validation_rework_rejects_stale_schema_and_validator_unavailable_blockers` |

边界说明：该修复不允许 runtime 无条件忽略真实 schema 缺失。只有当 validation rework 已经有 dependency schema/fact-source
evidence，并且当前 target 已完整读取时，才把这类 blocker 识别为 stale missing-source 语义并拒绝。

## 2026-07-04 R4-D issue type addendum: full-visible mixed native hunk recovery drift

`39caa76` 后 keyed rerun 已 live-clear H-106：stale schema-knowledge blocker 没有复现，模型继续进入 patch 路径。
新的 failure 是 native-hunk recovery 的 actionability 不够闭合：在 `process.py` 已完整可见后，provider 连续多次把
unified headers/range hunks 放进 native `*** Update File` section。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `apply-patch-full-visible-mixed-native-hunk-recovery-drift` | apply_patch grammar feedback / validation rework recovery actionability | full-visible target read 后，`TaskSpaceApplyPatchNativeHunkRecoveryV1` 仍允许 `Update File + @@` 作为主路径，模型重复 mixed native/unified grammar 到 hard-stop | full-visible validation rework target 下，native-hunk recovery 强制 whole-file replacement：`*** Delete File` + `*** Add File`；显式禁止 `*** Update File` 和 unified headers/range hunks | keyed rerun `20260704-234927-306`; CoE H-107/E-219/E-220; `native_hunk_recovery`; `mixed_native_unified`; `apply_patch_recovery`; `validation_rework`; `taskspace_apply_patch` |

边界说明：该修复不把 malformed patch 当成功，也不重新允许 mixed grammar 进入工具层。它只在目标文件已完整可见时把反馈动作空间收窄为
whole-file replacement，避免继续消耗请求预算在同一种 grammar 错误上。

## 2026-07-05 R4-D issue type addendum: replacement-only recovery enforcement gap

`7409c30` 后 keyed rerun 证明 H-107 的 forced replacement recovery 已进入 provider-visible feedback：
hard-stop excerpt 明确包含 `whole-file native replacement`、`Delete File + Add File`、`Do not emit Update File`。
但 provider 仍连续发 `Update File` mixed native/unified patch，说明仅靠文案不足，需要 action-contract enforcement。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `apply-patch-replacement-only-recovery-enforcement-gap` | apply_patch action-contract enforcement / recovery-state semantics | replacement-only recovery 已可见，provider 仍发 `*** Update File` + unified headers/range hunks，runtime 继续走 generic `apply_patch_mixed_native_unified` 到 hard-stop | 已实现：full-visible replacement-only recovery 激活后，针对 active validation rework target 的 `*** Update File` 直接拒绝为 `apply_patch_replacement_required:<target>`，并回到强制 `*** Delete File` + `*** Add File` recovery | keyed rerun `20260705-000330-979`; CoE H-108/E-221/E-222; focused tests `requires_replacement`, `mixed_native_unified`, `native_hunk_recovery`, `validation_rework`, `taskspace_apply_patch` |

边界说明：该问题不是 H-107 文案未到达模型，而是文案到达后缺少状态化强制。当前 focused fix 已把 recovery-state
落到 action-contract 约束；真实 keyed rerun 仍需确认 live 链路是否完全闭合。

## 2026-07-05 R4-D issue type addendum: terminal blocked fact-source contradiction

`fc7cae1` 后 keyed rerun 越过 replacement-only hard-stop，但暴露新的反馈层问题：系统已通过 `rg --files`、
`read_file schema.json` 和 missing fact-source bootstrap 读取 CSV/schema，最终 terminal `blocked` 却声称这些文件不在
workspace，导致任务以 false local infrastructure blocker 结束。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `terminal-blocked-observed-fact-source-contradiction` | terminal blocked gate / feedback truthfulness | 已观察 required fact sources 后，`blocked node_id:null` 仍可声明这些文件缺失并终止任务 | 已实现：terminal `blocked` 接入 observed required fact-source gate；若 blocker 声称已观察文件 missing/not present/not found，则拒绝并反馈继续基于现有证据推进 | keyed rerun `20260705-002052-730`; CoE H-109/E-223/E-224; focused tests `terminal_blocker_rejects_missing_fact_sources_after_bootstrap_read`, `missing_fact_source`, `missing_source_blocker`, `action_contract_prompt` |

边界说明：这不是文件工具失败，也不是 runner 没提供 CSV/schema；日志证明文件已列出并读取。问题是 terminal blocker
路径绕过了普通 `block_node` 的证据矛盾校验。

## 2026-07-05 R4-D issue type addendum: non-sticky replacement-required state

`4aeb22f` 后 keyed rerun 证明 H-109 的 false terminal blocker 不再终止任务，但 active validation rework target
仍进入 patch recovery hard-stop：第一次 mixed `Update File` 被拒绝为 `apply_patch_replacement_required:process.py`，后续
其他 `Update File` 形态又落回 generic unanchored/mixed feedback。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `apply-patch-replacement-required-non-sticky-update-file` | apply_patch action-contract / replacement-required state | active rework target 已 replacement-required，但后续 `*** Update File` old/new、unanchored、normalized update 仍走 generic recovery 并 hard-stop | 已实现：active validation rework target 的任何 `*** Update File` 在 normalize 前后都优先返回 `apply_patch_replacement_required:<target>`；非 rework target 保持 generic feedback | keyed rerun `20260705-003821-682`; CoE H-110/E-225/E-226; focused tests `requires_replacement`, `keeps_generic_unanchored`, `mixed_native_unified`, `unanchored_update`, `validation_rework`, `taskspace_apply_patch` |

## 2026-07-05 R4-D issue type addendum: replacement-required recovery marker distortion

`b3d31ec` 后 keyed rerun 证明 H-110 action-contract sticky 状态已生效：同一 active rework target 的后续
`*** Update File` 都被拒绝为 `apply_patch_replacement_required:generate_organization.py`。但 feedback 层又暴露
语义扭曲：这些 replacement-required rejection 之后插入的 recovery marker 和 hard-stop excerpt 仍显示
`TaskSpaceApplyPatchNativeHunkRecoveryV1`，把“禁止 Update File，必须 whole-file replacement”的状态语义重新包装成
native hunk grammar 修复。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `apply-patch-replacement-required-recovery-marker-distortion` | feedback recovery marker / observability / hard-stop audit | classifier 已返回 `apply_patch_replacement_required:<target>`，但 provider-visible recovery、warning 和 hard-stop excerpt 仍标为 `TaskSpaceApplyPatchNativeHunkRecoveryV1` | 已实现：新增 `TaskSpaceApplyPatchReplacementRequiredRecoveryV1`；replacement-required recovery 不再 alias native-hunk marker；advisory/special warning、apply-patch recovery accounting、implement recovery accounting、duplicate-read preserve 和 hard-stop excerpt 全链路识别该 marker | keyed rerun `20260705-005608-072`; CoE H-111/E-227/E-228; focused tests `replacement_required`, `native_hunk_recovery`, `unanchored_update`, `action_contract_prompt`, `validation_rework`, `taskspace_apply_patch` |

边界说明：该修复不放宽 `apply_patch` 工具，也不把 malformed patch 当成功。它只保证已存在的
`apply_patch_replacement_required` 失败语义在反馈层、恢复计数和审计摘要中不再被改名为 native-hunk recovery。

## 2026-07-05 R4-D issue type addendum: replacement gate blocks actionable normalized patch

`23a25bd` 后 keyed rerun 证明 H-111 live-clear：replacement-required recovery marker 已正确显示为
`TaskSpaceApplyPatchReplacementRequiredRecoveryV1`。但同一 run 暴露能力层/反馈层边界问题：item_58 的
`Update File` patch 被 replacement-required gate 拦截；复制到诊断目录后，只需执行现有 normalizer 等价操作
（去掉 unified file headers、把 range hunk 改成 native `@@`）即可 apply，且 schema validation 通过。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `apply-patch-replacement-required-overblocks-actionable-update` | apply_patch action-contract / capability normalization / feedback boundary | active rework target 的可机械归一化 mixed `Update File` 被 `apply_patch_replacement_required` 无条件拒绝，错失可执行修复 | 已实现：active rework target 先尝试 normalize；归一化后无 malformed/mixed/unanchored 问题则 dispatch apply_patch；不可执行 update 才保留 replacement-required | keyed rerun `20260705-011054-226`; diagnostic `target/r4-h112-patch-diagnostic/item_58`; CoE H-112/E-229/E-230; focused tests `rework_target`, `replacement_required`, `mixed_native_unified`, `unanchored_update`, `validation_rework`, `taskspace_apply_patch` |

边界说明：这不是撤销 replacement-required 语义。无锚点、placeholder、仍带 mixed marker 或 malformed header 的 rework
`Update File` 仍会被拒绝为 replacement-required；只有归一化后可执行的 patch 进入工具层。

## 2026-07-05 R4-D issue type addendum: schema type mismatch repair semantics gap

`646edd8` 后 keyed rerun 证明 H-112 live-clear：active rework target 的 mechanically actionable `Update File`
已经实际进入 `apply_patch` 并产生 `file_change`。新的 failure 转入 validation feedback：`jsonschema` 明确报告
`skillDistribution` 不是 `object`，public validator 也因 `departmentSizes` 仍是 list 而失败，但 recovery 链没有把
“expected object”组织成 schema repair 事实，模型后续漂移到 CSV parsing、metadata 和 stale blocker。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-schema-type-mismatch-repair-semantics-gap` | tool semantic summary / validation rework repair contract / implementation recovery synthesis | validator 已给出 `skillDistribution expected object`，但反馈层只结构化 missing required properties 和 rename hints，未把 type mismatch 转成 patch-only 修复要求 | 已实现：tool summary、validation failure excerpt、validation repair contract 均输出 `schema_type_mismatches`；implementation recovery 对 `expected object` 明确要求输出 object/map，而不是 array of objects | keyed rerun `20260705-012516-669`; CoE H-113/E-231/E-232; focused tests `schema_type_mismatch`, `type_mismatch`, `validation_rework`, `action_contract_prompt` |

边界说明：该修复不绕过 validator，也不凭空改 schema。它只把 validator 已经输出的类型事实保真传给下一轮 patch 构造，
避免“类型不匹配”被压缩成普通失败文本后丢失操作语义。

## 2026-07-05 R4-D issue type addendum: unlocated array item type mismatch repair gap

`8451089` 后 keyed rerun 证明 H-113 对 statistics object-map 的修复已 live-clear：`skillDistribution`、
`departmentSizes`、`projectStatusDistribution` 均生成 object map，public validator 的 statistics test 通过。新的 failure
来自 `members`：schema 要求 `members` 是 string array，但 validator 只输出多个 dict value
`is not of type 'string'`，没有 bracket path，runtime 没有把它映射回 `members.items.type=string`。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-schema-array-item-type-mismatch-repair-gap` | validation rework repair contract / schema-backed feedback synthesis | validator 输出 unlocated object-not-string item failures；provider 未收到 `members expected string items`，误判成排序问题并写坏缩进 | 已实现：从已读 schema 抽取 array item type expectations；将 unlocated type mismatch 与 schema array item 定义合并为 `schema_type_mismatches=members expected string items`；recovery 明确 expected string items 要输出 string array | keyed rerun `20260705-024255-572`; CoE H-114/E-233/E-234; focused tests `validation_rework_projects_schema_repair_contract_from_schema_read`, `array_item_type`, `type_mismatch`, `validation_rework` |

边界说明：该修复不是 hard-code `members`。它只在已有 schema read 和 validation type mismatch 同时存在时，把未定位的 primitive item
type failure 连接到 schema 中的 array field，作为下一轮 patch 构造事实。

## 2026-07-05 R4-D issue type addendum: type mismatch path pollution and placeholder range leakage

`e182c9b` 后 keyed rerun 未能验证 H-114 live-clear，因为更早的 validation rework patch recovery 失败。该轮暴露两个
反馈/能力边界问题：type mismatch extractor 把普通数据 list 中的 `['RedBull']` 误当 schema path；rework update
gate 又把 `@@ -... +... @@` placeholder range hunk 归一化后送进 apply_patch。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-schema-type-mismatch-data-bracket-path-pollution` | tool semantic summary / validation failure excerpt | `schema_type_mismatches` 出现 `RedBull expected string` 等普通数据值，污染 repair contract | 已实现：bracket path 解析只接受 jsonschema 的 `schema[...]` / `instance[...]` 路径；普通 Python repr list 不再作为 schema path | keyed rerun `20260705-025939-670`; CoE H-115/E-235/E-236; focused tests `data_lists`, `type_mismatch` |
| `apply-patch-rework-placeholder-range-hunk-leakage` | apply_patch action-contract / capability normalization | replacement-required 后，`@@ -... +... @@` placeholder hunk 被 normalize 成 native-looking hunk并进入 apply_patch，最终 expected-lines hard-stop | 已实现：mechanically actionable rework update 在 normalize 前后拒绝 placeholder range hunk，回到 `apply_patch_replacement_required:<target>` | keyed rerun `20260705-025939-670`; CoE H-116/E-235/E-236; focused tests `placeholder_range`, `rework_target`, `mixed_native_unified` |

边界说明：该修复不撤销 H-112 的 actionable patch normalization。带具体行号和真实 old/new context 的 range hunk 仍可归一化执行；
只有 `...` placeholder range 被视为不可执行。

## 2026-07-05 R4-D issue type addendum: generated output target advertised as unread fact source

`c78e8fc` 后 keyed rerun 证明 H-115/H-116 没有复现：普通数据值不再污染 `schema_type_mismatches`，
placeholder range hunk 也未进入 apply_patch。但该轮暴露更早的 inspect feedback 问题：runtime 把
`organization.json` 同时作为 output contract 和 declared fact-source artifact，导致模型反复读取尚未生成的输出文件，
最终 `inspect_code_context` 节点耗尽 provider node budget。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `inspect-output-contract-as-fact-source-false-positive` | inspect fact-source extraction / context projection / duplicate-read recovery | projection 和 recovery 要求 `read_file declared fact-source artifact organization.json`，但 `organization.json` 是生成输出，不是输入证据；模型无法进入 implement_solution | 已实现：从 fact_sources 抽取 required artifact 时过滤已声明生成输出；输出目标集合排除 schema/validator contracts，确保 `schema.json` 仍是输入/验证依赖 | keyed rerun `20260705-031706-550`; CoE H-117/E-237/E-238; focused tests `inspect_fact_source_extraction_ignores_declared_generated_output_targets`, `inspect_fact_source`, `output_contract`, `inspect_duplicate_read` |

边界说明：该修复不放松 CSV/schema 输入读取要求。`departments.csv`、`employees.csv`、`projects.csv`、`schema.json`
仍会作为 inspect coverage；只有已由 output contract 声明的生成物不再被要求在实现前读取。

## 2026-07-05 R4-D issue type addendum: missing-source blocker rejection overcounts patch-only hard-stop

`f2c31e4` 后 keyed rerun 证明 H-117 live-clear：inspect 不再要求读取生成物 `organization.json`，
并进入 implementation / validation rework。新的 blocker 出现在 feedback accounting：provider 发出
“缺 schema/source，无法修复”的 block_node，runtime 已正确拒绝为 `missing_source_visibility_blocker_rejected`，
但随即计入 patch-only repeated non-edit hard-stop，导致模型没有机会消费这条更强反馈。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-missing-source-blocker-rejection-hardstop-overcount` | validation rework patch-only recovery accounting / feedback loop | runtime 拒绝 missing-source blocker 后立即 `TaskSpaceValidationReworkPatchOnlyHardStopV1`，没有给 provider 下一轮 apply_patch 机会 | 已实现：`missing_source_visibility_blocker_rejected` 与 closed-action rejection 一样有一次 patch-only recovery grace；重复无效 blocker 仍 hard-stop | keyed rerun `20260705-032858-986`; CoE H-118/E-239/E-240; focused tests `validation_rework_patch_only_allows_one_missing_source_blocker_rejection_recovery`, `validation_rework_patch_only`, `action_contract_prompt` |

边界说明：该修复不是放宽 patch-only 约束。read/list/search/schema inspection 仍无效；只是把“被 runtime 反驳的 block_node”
作为新的强语义反馈交给模型一次，要求下一步必须基于已读 target 和 validation failure 进行 apply_patch。

## 2026-07-05 R4-D issue type addendum: generic CSV input fact-source undercoverage

`c8d2359` 安装后 keyed rerun 证明 H-118 live-clear：没有再出现
`missing_source_visibility_blocker_rejected` 后立即 hard-stop。新的 blocker 前移到 inspect coverage：
start_task 把用户要求压缩成“Read existing CSV files and schema.json”，runtime 只强制读取了 `schema.json`，
没有把 list_files 发现的 `departments.csv`、`employees.csv`、`projects.csv` 升级为必须读取的输入证据。
provider 随后凭猜测写 `process.py`，在 `projects.csv` 上触发 `KeyError: 'id'`。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `inspect-generic-csv-input-fact-source-undercoverage` | inspect fact-source expansion / forced inspect transition / input-data capability boundary | 需求说 `CSV files` 且 list_files 已发现具体 CSV，但 forced inspect transition 仍在未读 CSV 内容时进入 implementation | 已实现：generic CSV requirement 会把发现的具体 `.csv` 输入加入 required fact-source reads；`===== file.csv` bootstrap sections 满足 coverage；`*.csv` glob 不会成为 required artifact | keyed rerun `20260705-034521-738`; CoE H-119/E-241/E-242; focused tests `inspect_generic_csv_requirement_expands_discovered_csv_inputs`, `inspect_missing_fact_source`, `forced_inspect_transition` |

边界说明：该修复不把所有 repo CSV 都无条件变成任务输入。只有任务要求中出现 CSV input/data/files/source 语义，
且 inspect 已发现具体 `.csv` 文件时，才把这些具体输入加入必读 fact-source coverage；生成输出过滤仍保留。

## 2026-07-05 R4-D issue type addendum: successful validation closeout reported as blocked

`1b1ddf9` 安装后 keyed rerun 证明 H-119 live-clear：TaskSpace 在 inspect 中读取了
`schema.json`、`departments.csv`、`employees.csv`、`projects.csv`，随后生成并修复 `organization.json`，
public validation 和 hidden oracle 都通过。但最终用户可见消息仍是 terminal `blocked`，理由声称 schema
validation 被 local infrastructure 阻塞。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `closed-validation-success-final-blocked-false-positive` | feedback + terminal closeout / state priority | `TaskSpaceForcedValidationCloseoutRecoveryV1` 已要求 final_answer，且验证结果已成功；旧 blocked validation/local infra evidence 仍驱动 provider 输出 `blocked` | 已实现：successful validation evidence 优先于旧 blocker evidence；成功后不再注入 closed-validation/tool-runtime blocker contract；无 active node 且任务已验证完成时，terminal `blocked` 被转换为 `final_answer` | keyed rerun `20260705-035754-438`; CoE H-120/E-243/E-244; focused tests `closed_validation_blocker_is_suppressed_after_successful_validation`, `completed_task_final_answer_conversion_includes_blocked_action`, `action_contract_prompt`, `forced_validation_closeout` |

边界说明：旧 blocker evidence 不会被删除，仍保留在 replayable state 和 evidence trail 中。但一旦同一 active
map 已有 accepted successful validation result，终态反馈必须表达“验证已通过，可以 final_answer”，不能再把旧 blocker
提升为用户可见 blocked。

## 2026-07-05 R4-D issue type addendum: missing fact-source bootstrap root path and fallback transition gap

`fc5c7a8` 安装后 keyed rerun 未能再次到达 H-120 final closeout，而是在 inspect 早期 hard-stop：
runtime 试图自动读取缺失 CSV fact sources，但把 `/employees.csv`、`/departments.csv`、`/data/projects.csv`
当作 shell 绝对路径执行，导致读失败。后续 fallback 虽然读到了相对 CSV sections，但没有立即 forced transition，
provider 又重复 schema/list 直到 node budget hard stop。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `inspect-bootstrap-root-path-transition-gap` | feedback + capability + phase gate | missing fact-source bootstrap 用 `/employees.csv` 绝对路径读失败；fallback 读到 CSV sections 后仍未进入 implementation | 已实现：bootstrap read path 将 workspace-root-style `/...` 归一化为相对路径；relative `=====` sections 可满足 root-style required refs；fallback bootstrap 读完后立即尝试 forced inspect transition | keyed rerun `20260705-041253-955`; CoE H-121/E-245/E-246; focused tests `missing_fact_source_bootstrap_command_uses_workspace_relative_paths`, `inspect_missing_fact_sources_accept_relative_sections_for_root_refs`, `inspect_missing_fact_source`, `forced_inspect_transition` |

边界说明：该修复不把任意宿主绝对路径开放给工具。TaskSpace artifact ref 在 benchmark sandbox 中是工作区相对语义；
自动 bootstrap 只把这种 root-style workspace artifact 转成相对读取命令，仍通过普通 sandboxed shell 读取。

## 2026-07-05 R4-D issue type addendum: generic CSV duplicate basename overcoverage

`6b7debf` 安装后 keyed rerun 证明 H-121 的 root-path bootstrap failure 已 live-clear：自动 bootstrap
读取了相对路径 `schema.json`、`departments.csv`、`employees.csv`、`projects.csv`。新的 blocker 是
generic CSV discovery 同时把 root CSV 与 `data/` 下同 basename 副本都升级成 required fact sources；bounded
bootstrap 读完 canonical root CSV 后，`data/*.csv` 仍残留为 missing fact-source，导致 inspect 重复到 node budget hard stop。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `inspect-generic-csv-duplicate-basename-overcoverage` | inspect fact-source expansion / feedback coverage / phase gate | generic `CSV files` 需求下，同 basename root/data CSV 副本都被要求读取，canonical root CSV 已读仍无法 forced transition | 已实现：仅对 discovery-derived generic input refs 按 basename canonicalize，优先保留 shallower/root-level path；显式 concrete fact-source refs 不被静默去重 | keyed rerun `20260705-042157-236`; CoE H-122/E-247/E-248; focused tests `inspect_generic_csv_requirement_expands_discovered_csv_inputs`, `inspect_missing_fact_source`, `forced_inspect_transition` |

边界说明：该修复不是忽略 `data/` 目录，也不是把所有同名文件合并。它只作用于“泛化 CSV input”
由文件发现派生出来的 required input set，避免同一业务输入的重复副本把反馈层变成无限 missing-source。

## 2026-07-05 R4-D issue type addendum: read-summary path telemetry pollution

`a808190` 安装后 keyed rerun 证明 H-122 的 duplicate basename overcoverage 已不再作为主 blocker：
第一次 missing fact-source bootstrap 成功读取了 root-level `departments.csv`、`employees.csv`、`projects.csv`。
新的 blocker 是 read-summary telemetry 被 artifact extractor 当成路径：`TaskSpaceReadFileSummaryV1: path=departments.csv`
生成了 synthetic required artifact `path=departments.csv`，后续 bootstrap 读取 literal `path=*.csv` 失败。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `inspect-read-summary-path-telemetry-artifact-pollution` | tool semantic summary / artifact extraction / feedback coverage | 正确的 read summary `path=foo.csv` 被解释成新文件 `path=foo.csv`，导致 missing fact-source 和 bootstrap command 都读错路径 | 已实现：artifact ref normalization 剥离 telemetry 前缀 `path=`，让 `path=foo.csv` 等价于 `foo.csv`；generic CSV bootstrap 测试覆盖 summary path 字段 | keyed rerun `20260705-043055-329`; CoE H-123/E-249/E-250; focused tests `inspect_generic_csv_requirement_expands_discovered_csv_inputs`, `inspect_missing_fact_source`, `forced_inspect_transition` |

边界说明：该修复不删除 `TaskSpaceReadFileSummaryV1`，也不降低 artifact coverage gate。它只把工具摘要中的
`path=` 键值字段还原为真实 artifact ref，避免 telemetry 格式污染能力层。

## 2026-07-05 R4-D issue type addendum: inspect hard-stop transition attempt guard gap

`601bc74` 安装后 keyed rerun 证明 H-123 已 live-clear：`path=*.csv` synthetic artifact 不再出现。
新的 blocker 是 session control：inspect 已成功读取 `departments.csv`、`schema.json`、`employees.csv`、
`projects.csv`，但 `node_request_count=5/5` 时直接 provider budget hard-stop，没有先让 runtime 尝试
`inspect_hard_stop_progress_convergence`。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `inspect-hard-stop-transition-attempt-guard-gap` | session provider-budget control / phase transition bridge | required fact-source coverage 已完整，hard-stop 前仍未进入 implement；session 额外 readiness predicate 阻止 runtime convergence 判断 | 已实现：inspect node pre-dispatch hard-stop 先调用 runtime forced transition；runtime 仍负责拒绝 missing fact-source 或弱 evidence，不满足才继续 terminal hard-stop | keyed rerun `20260705-043735-552`; CoE H-124/E-251/E-252; focused tests `inspect_hard_stop_progress_convergence_forces_transition_after_coverage`, `provider_budget`, `forced_inspect_transition`, `inspect_missing_fact_source` |

边界说明：这不是提高 provider budget，也不是无条件从 inspect 跳 implement。session 只移除过窄的前置判断；
是否可 transition 仍由 ActionMap runtime 根据成功 read/search、unread scripts、missing fact-source 等 gate 决定。

## 2026-07-05 R4-D issue type addendum: validation blocker supersession gap

`e0a17fc` 安装后 keyed rerun 证明 H-124 已 live-clear：inspect 不再停在 hard-stop，而是进入 implementation
和 validation。新的 blocker 是 final readiness feedback：第一次 validation 失败生成 `result-10` blocker，
后续 rework 和 validation closeout 已完成，但 final gate 仍以 `result-10 still unreviewed` 拒绝 final answer，
导致 provider 在 `phase=unknown/node_kind=unknown` 下重开 inspect，直到 provider budget hard-stop。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-blocker-supersession-final-gate-gap` | validation recovery lifecycle / final readiness feedback | downstream rework validation 成功后，旧 validation blocker 仍作为 unresolved unreviewed result 阻断 final；provider 收到错误 next action 语义后重试/重开 inspect | 已实现：successful validation closeout 沿 dependency/origin rework chain 找到被覆盖的 blocked validation blocker，并标记为 `invalid`；active rework 期间旧 blocker 仍允许保持 unreviewed | keyed rerun `20260705-044510-605`; CoE H-125/E-253/E-254; focused tests `validation_closeout_invalidates_superseded_rework_blocker_for_final_answer`, `validation_closeout`, `validation_rework`, `action_contract_prompt` |

边界说明：该修复不是放宽 final gate 对普通 unreviewed result 的要求。它只处理已被后续 accepted validation
结果覆盖的旧 validation blocker，将其从“待 review 的活跃阻塞”转换为“被成功 rework 废止的失败证据”。

## 2026-07-05 R4-D issue type addendum: validation rework schema rediscovery hard-stop timing gap

`75de79f` 安装后 keyed rerun 未复现 H-125 的 stale final-gate blocker，但 validation rework 进入新的
patch-only feedback timing gap：runtime 已把 schema 缺失字段、rename hints 和完整 target source 都放进 recovery；
provider 仍尝试读取 `schema.json`，runtime 正确拒绝，却立即 hard-stop，模型没有下一轮消费“schema 已摘要、只能 patch”的
更具体反馈。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `validation-rework-schema-rediscovery-patch-only-grace-gap` | validation rework feedback timing / patch-only hard-stop accounting | schema repair contract 已可见且 target 已读后，provider 重读 `schema.json` 被拒绝；patch-only hard-stop 立即触发，截断一次可行动反馈 | 已实现：schema repair synthesis 存在时 recovery 带 `schema_repair_rediscovery_grace=true`，并允许一次额外 recovery；普通无 schema repair 的 patch-only 路径仍一次后 hard-stop | keyed rerun `20260705-050333-167`; CoE H-126/E-255/E-256; focused tests `validation_rework_patch_only_schema_repair_gets_one_extra_recovery_before_hard_stop`, `validation_rework_patch_only_without_schema_repair_still_hard_stops_after_one_recovery`, `validation_rework`, `action_contract_prompt` |

边界说明：这不是允许 validation rework 重新 discovery。`read_file/search schema.json` 仍然会被拒绝；区别只是当
schema repair contract 已经完整投影时，拒绝本身会成为一次可消费的反馈，而不是直接进入 hard-stop。没有 schema repair
synthesis 的 patch-only case 不获得这次 grace。

## 2026-07-05 R4-D issue type addendum: replacement-required recovery budget loop

`efb0faf` 安装后 keyed rerun 证明 H-126 live-clear：schema rediscovery hard-stop 未复发，流程进入 apply_patch。
新的 blocker 是 replacement-required feedback：目标文件已 full-visible，action contract 正确拒绝 `*** Update File`
和 `*** Context Lines` 风格的伪 native patch，但 recovery 仍过于通用并反复 advisory，最终被全局 provider budget
hard-stop 覆盖。

| Issue type | Layer | Symptom | Resolution contract | Evidence |
|---|---|---|---|---|
| `apply-patch-replacement-required-recovery-budget-loop` | apply_patch feedback / replacement-only enforcement / provider budget interaction | `apply_patch_replacement_required:<target>` 重复出现；recovery 没有给具体 Delete/Add target scaffold，且重复非 replacement 未先 hard-stop | 已实现：replacement-required recovery 读取 working evidence；full-visible target 时输出具体 `Delete File`/`Add File` scaffold；重复同节点 replacement-required rejection 转为 apply-patch recovery hard-stop，不再落到 provider budget | keyed rerun `20260705-051421-876`; CoE H-127/E-257/E-258; focused tests `replacement_required`, `apply_patch_recovery`, `validation_rework`, `action_contract_prompt` |

## 2026-07-05 R4-D issue type addendum: validation node budget bootstrap gap

`d37a7f1` 后 keyed rerun live-clear 了 replacement-required loop，但暴露 provider budget 与 validation feedback 的交界问题：
successful implementation edit 后 runtime 创建了 `node-7 kind=smoke_test`，此时总 provider request 已到 `20/20`，下一次
pre-dispatch 直接 `TaskSpaceProviderBudgetHardStopV1`，导致 fresh validation node 没有任何 test/build 证据。

| issue type | 层级 | 本质 | 修复 | 证据 |
|---|---|---|---|---|
| `validation-node-provider-budget-bootstrap-gap` | session provider-budget control / validation feedback bootstrap / action-map evidence | coverage-correct validation command 已可由 runtime 确定，但 fresh validation node 在 provider hard-stop 前未执行 test，反馈停在 budget hard-stop 而非验证结果 | 已实现：runtime 暴露 fresh validation bootstrap command；session pre-dispatch hard-stop 前运行本地 validation bootstrap；成功则 forced validation closeout，失败则记录 Test 失败证据；已有 test/build result 后不再 bootstrap | keyed rerun `20260705-053314-639`; CoE H-128/E-259/E-260; focused tests `validation_node_blocks_generator_only_command_for_schema_output_contract`, `validation_required_command_bridge`, `provider_budget`, `validation_closeout` |

边界说明：这不是提高 provider budget，也不是绕过状态机。bootstrap 只在 provider 已拒绝下一次模型请求、当前节点是
`smoke_test/regression_test`、且 runtime 能从已知 local validator 或 changed artifact + output contract 推导出确定命令时运行。
没有确定命令或节点已有 test/build 证据时仍按原 provider hard-stop/validation closeout 逻辑处理。

## 2026-07-05 R4-D issue type addendum: validation missing-command blocker false positive

`5ee9b5c` 后 keyed rerun 没有命中 fresh validation provider-budget hard-stop，而是在更早阶段暴露 validation blocker
语义漏洞：实现 edit 已成功，`smoke_test` 节点已创建，但 provider 用 `blocked` 声称“没有 validator command / 需要 inspect
test harness”，runtime 接受后终态变成 false infrastructure blocker。

| issue type | 层级 | 本质 | 修复 | 证据 |
|---|---|---|---|---|
| `validation-missing-command-visibility-blocker-false-positive` | validation blocker gate / feedback actionability / action-map evidence | fresh validation node 无 test/build result，且 runtime 可从 changed artifact + output contract 推导命令时，仍接受“缺少 validator/test command 可见性”的 blocker | 已实现：`block_main_node` 在 fresh validation node 上先查 deterministic bootstrap command；命中时拒绝 missing-command visibility blocker，并返回 exact `run_test` 命令；真实 access/sandbox/network 外部 blocker 不被改写 | keyed rerun `20260705-055257-224`; CoE H-129/E-261/E-262; focused tests `validation_node_blocks_generator_only_command_for_schema_output_contract`, `block_validation_node` |

边界说明：这不是禁止 validation node block。只有当 blocker 的理由是 validator/test command 不可见、不可发现或伪 shell
不可用，且 runtime 已能推导确定命令时才拒绝；已经有具体 failed test/build result 或真实外部访问阻塞时仍按原 validation
block/rework 规则处理。

## 2026-07-05 R4-D issue type addendum: start-task alias semantic loss

`f0d6c47` 后 keyed rerun 证明 H-129 live-clear：missing-command blocker 没有复发，runtime recovery 能执行验证命令。
新的 blocker 是更早的 start_task 语义丢失：provider 发送 `task_description`、`initial_criteria`、
`initial_contracts`、`first_node_kind`、`first_node_description` 等自然别名，但 tool/parser 没有完整 canonicalize，
导致 action map 的 objective 退化成 `TaskSpace task`，output contract 退化成泛化用户请求。后续 bootstrap 只执行
`python generate_organization.py`，把 generator exit=0 误记为 validation pass，外部 public validator 才暴露
`members` 和 `averageDepartmentBudget` 缺失。

| issue type | 层级 | 本质 | 修复 | 证据 |
|---|---|---|---|---|
| `start-task-natural-alias-semantic-loss` | taskspace_control capability layer / action-contract normalization / validation feedback precondition | 工具接受 start_task action，但关键需求字段因别名未归一化而静默丢失，后续 validation feedback 缺少 output/schema contract | 已实现：native handler 与 action-contract session 层共同接受 `task_description`、`initial_criteria`、`initial_contracts`、`first_node_kind/initial_node_kind`、`first_node_description` 等别名；start_task initial sections 支持 string/array/single-object | keyed rerun `20260705-060117-936`; CoE H-130/E-263/E-264; focused tests `start_task_accepts_natural_task_payload_aliases`, `start_task_wraps_single_initial_section_objects`, `taskspace_action_contract_canonicalizes_natural_start_task_aliases`, `taskspace_control` |

边界说明：这不是把自然语言固定模板化，也不是降低状态机要求。修复只发生在工具参数正规化层：
模型仍必须走 Agent/TaskSpace 路径；runtime 仍根据规范化后的 objective、output contracts、fact sources 和 validation
results 执行原有 gate。

边界说明：该修复不放宽 apply_patch grammar。相反，它把 replacement-only 从泛化建议强化为目标明确的 feedback
和重复失败控制：第一次给模型可执行 replacement scaffold，第二次仍不遵守则用专门 hard-stop 暴露该工具链失败。

## 2026-07-05 R4-D issue type addendum: validation rework patch feedback budget cliff

`933085f` 安装后 keyed rerun 证明 H-130 的关键下游后果已清除：runtime 不再只跑
`python generate_organization.py`，而是执行了 `python generate_organization.py && python -m jsonschema -i organization.json schema.json`。
新的 blocker 出现在 validation rework 恢复反馈的送达边界：runtime 已经插入
`TaskSpaceValidationReworkPatchOnlyRecoveryV1`，但全局 provider request 已到 `20/20`，下一轮直接
`TaskSpaceProviderBudgetHardStopV1`，模型没有机会消费“只能 patch/block”的反馈。

| issue type | 层级 | 本质 | 修复 | 证据 |
|---|---|---|---|---|
| `validation-rework-patch-feedback-budget-cliff` | provider budget gate / validation rework feedback delivery / runtime capability boundary | runtime 生成的 patch-only feedback 被插入在普通 budget grace 已消耗之后，语义存在但无法送达模型 | 已实现：pre-dispatch gate 增加窄条件 `provider_validation_rework_patch_feedback_grace`；仅 validation rework implement 节点、目标 artifact 已知、无成功 edit、全局预算耗尽、普通 grace 已用、节点请求数恰为 1 且节点预算未耗尽时允许一次 | keyed rerun `20260705-061558-109`; CoE H-131/E-265/E-266; focused test `taskspace_active_budget_allows_validation_rework_patch_feedback_grace`; regressions `provider_budget`, `validation_rework` |

边界说明：这不是提高普通 provider budget，也不是让状态机放宽 hard-stop。状态机仍提供事实：
当前是否 validation rework、目标 artifact 是否已知、是否已有 edit；runtime 只在这些事实满足时允许一次“反馈送达”
请求，第二次同节点请求或缺少 validation rework artifact 仍按原 hard-stop。

## 2026-07-05 R4-D issue type addendum: apply_patch placeholder ellipsis hunk leakage

`6a4ec2e` 安装后 keyed rerun 没有复发 provider budget hard-stop，patch-only feedback 已能被模型消费。
新的 blocker 是 apply_patch 语法反馈层：provider 在 validation rework 中发出 `*** Update File: process.py`
加 `@@ ... @@` 占位 hunk；旧检测只覆盖 `@@ -... +... @@`，导致该 patch 进入 edit tool 并记录泛化
`tool_failure`，而不是在 action contract 层返回 replacement-required。

| issue type | 层级 | 本质 | 修复 | 证据 |
|---|---|---|---|---|
| `apply-patch-placeholder-ellipsis-hunk-leakage` | action-contract apply_patch grammar gate / validation rework feedback | `@@ ... @@` 占位 hunk 不是可机械应用 patch，但未被 placeholder detector 识别，失败语义从 grammar/replacement 扭成 generic edit failure | 已实现并 live-cleared：placeholder hunk detector 纳入 `@@ ... @@` / `@@...@@`；validation rework target 上返回 `apply_patch_replacement_required:<target>`，不再执行 edit tool | failing keyed rerun `20260705-063230-012`; solved keyed rerun `20260705-064634-577`; CoE H-132/E-267/E-268/E-269; focused test `taskspace_action_contract_requires_replacement_for_rework_target_placeholder_ellipsis_hunk`; regressions `action_contract_prompt`, `validation_rework`, `taskspace_apply_patch` |

边界说明：这不是禁止所有含 `...` 的文件内容。检测只在 patch hunk header 行本身是 `@@ ... @@`/`@@...@@`
或已有 placeholder range hunk 时触发；普通文件内容里的 `...` 仍按原 patch normalization 处理。

## 2026-07-05 R4-D issue type addendum: path-correction recovery budget drain

E3 targeted `multi-source-data-merger` 诊断证明路径纠错已经从“反馈缺失”推进到“反馈已送达但恢复循环无界”：
`/data` 失败后，`/data/source_a/users.json` 和 `/data/source_b/users.csv` 被
`path_correction_retry_forbidden` 拒绝在 shell dispatch 之前；但专用
`TaskSpacePathCorrectionRecoveryV1` 不计入 generic no-action hard-stop，最终仍落到
`TaskSpaceProviderBudgetHardStopV1 node_request_count=7/6`。

| issue type | 层级 | 本质 | 修复 | 证据 |
|---|---|---|---|---|
| `path-correction-recovery-budget-drain` | action-contract path correction / feedback recovery accounting / provider budget control | 路径失败语义已传到 tool boundary，但 provider 重试绝对 workspace alias 后，专用 path-correction recovery 可反复 advisory，直到 generic provider budget hard-stop | 已实现：新增 `TaskSpacePathCorrectionHardStopV1`；同一 node 允许一次 path-correction recovery prompt，第二次仍重复确定性绝对路径拒绝时停止本 turn；hard-stop excerpt 不再被误识别为 recovery item | targeted run `20260706-003657-482`; CoE H-148/E-298/E-299; focused tests `path_correction_recovery_hard_stops_after_one_retry_prompt`, `path_correction`, `provider_response_actionability` |
| `path-correction-stale-feedback-after-successful-relative-read` | path-correction recovery lifecycle / actionability state cleanup | provider 已从 `/data` 改为成功的 `rg --files .`，但旧 path-correction state 未清除，导致 `TaskSpacePathCorrectionHardStopV1` 误把成功相对读之后的恢复判断当成重复绝对路径违规 | 已实现：成功 `list_files`/`read_file`/`search` 且没有新的 path-not-found correction 时清除 path-correction feedback；edit/control success 不清除 | targeted run `20260706-004757-758`; CoE H-149/E-300/E-301; focused test `path_correction_feedback_clears_after_successful_read_surface_action`; regressions `path_correction`, `taskspace` |

边界说明：这不是扩大 `/data` 访问权限，也不是把失败读当成成功证据。runtime 仍拒绝绝对 alias
并提供 workspace-relative suggestion；修复只把重复确定性拒绝从高成本 provider budget hard-stop 收敛为专用、可审计的
path-correction hard-stop。
