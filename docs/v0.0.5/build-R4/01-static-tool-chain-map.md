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
