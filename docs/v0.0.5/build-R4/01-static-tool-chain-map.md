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
