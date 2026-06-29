# R3 工程层收口状态

本文记录 2026-06-30 对 build-R3 的工程层 100% 收口结果。这里的“工程层收口”只表示 R3 plan 中要求的代码、脚手架、门禁、可执行证据和当前 HEAD 证明已经补齐；不把人工审计、formal E3 得分、速度/成本发布结论伪装成已完成。

## 1. 收口边界

R3 工程层必须满足：

1. R3-A/B/C 的 provider-visible context replacement、bundle contract、cache plan 证明在当前 HEAD 通过。
2. R3-D 的 graph closeout 和 lifecycle 收敛问题有代码修复和回归测试。
3. R3-E 的 timing attribution、request phase attribution 和 observability 门禁通过。
4. R3-F 的 non-agent gates、code-complete marker、release/start gate 脚手架完整，并且不能绕过 explicit user approval。
5. 之前文档中写过的工程缺口不能停留在“人工手写 fixture”或“临时 AllowStale”状态。

不属于工程层自动完成的内容：

1. `v005-user-approval.json`：这是用户授权 marker，不能由工程脚本自动伪造。
2. formal E3 aggregate utility：必须在用户授权后运行完整 E3，并经过人工审计和 release decision。
3. 速度、成本、cache-hit 的发布级收益结论：需要 formal E3 或等价规模证据。

## 2. 本轮补齐的工程缺口

| 缺口 | 根因 | 修复 |
|---|---|---|
| `WhaleBinaryHealth` 依赖 `whale.exe` mtime，Cargo no-op build 后会误判 stale | 只比较源码变更时间和二进制时间，缺少构建产物身份 | 增加 `whale.exe.build-attestation.json`，记录 binary sha、repo root、Codex source commit；health gate 可识别 attested stale mtime |
| code-complete / user-approval marker 只能靠测试手写 fixture | marker 不是一等脚本产物，无法稳定复用到 start/release gate | 新增 `write-v005-markers.ps1`，统一生成 `v005-code-complete.json`，并只在显式 `-ApproveFullE3` 时生成 user approval |
| current-HEAD non-agent gates 未覆盖新脚手架 | release/start gate 只检查旧 fixture，无法证明 wrapper 和 marker writer 可用 | `build-v005-non-agent-gates.ps1` 增加 `external_wrapper_fixture` 和 `marker_writer_fixture` |
| `active_context_replacement` gate 测试契约落后于 lifecycle normalization | 测试仍读取旧字段 `result`，生产路径已经归一为 `result_summary` | 更新 Rust 测试断言，保持真实 lifecycle contract 和 gate 一致 |

相关提交：

```text
6336a26e9 close r3 benchmark marker and binary health gaps
4adadcc94 fix taskspace lifecycle normalization test contract
```

## 3. 当前 HEAD 证据

```text
HEAD = 4adadcc94
TaskList = target\r3-engineering-closeout-formal-task-list.jsonl
task_list_hash = f603bd25c787f7142a756994e2b773f73ac36ad99141f2d18018462e6a4950fa
sample_set_id = terminal-bench_E3-P0_3_5
profile_hash = 53dc5d28741f87ad36b5a714d7971a471da6ff83f98e8ede6e0b82efad376861
source_version = 1a6ffa9674b571da0ed040c470cb40c4d85f9b9b
```

当前 HEAD non-agent gates：

```text
Artifact = target\r3-engineering-closeout-non-agent-gates-head-4adadcc\v005-non-agent-gates.json
Status = pass

provider_request_hook = pass
runtime_budget_response = pass
budget_quality_impact = pass
active_context_replacement = pass
state_commit_displacement = pass
spawn_node_budget = pass
request_phase_attribution = pass
release_decision_fixture = pass
start_gate_fixture = pass
external_wrapper_fixture = pass
marker_writer_fixture = pass
```

code-complete marker：

```text
Artifact = target\r3-engineering-closeout-markers\v005-code-complete.json
Status = pass
```

E3 start gate：

```text
Artifact = target\r3-engineering-closeout-start-gate\e3-start-gate.json
Status = blocked_for_full_e3

disk_preflight = pass
docker_storage = pass
task_list = pass
v005_non_agent_gates = pass
v005_code_complete = pass
v005_user_approval = blocked
stable_code = v005_user_approval_missing
full_e3_allowed = false
```

这说明 R3 工程层已经可以生成当前 HEAD 的完整 code-complete 证据，但 formal E3 仍被 explicit user approval marker 阻断。这个阻断是 R3-F 设计要求，不是工程代码缺口。

## 4. 已执行验证

```text
powershell -File scripts\taskspace-benchmark\test-v005-marker-writer.ps1 = PASS
powershell -File scripts\taskspace-benchmark\test-v005-non-agent-gates-builder.ps1 = PASS
powershell -File scripts\taskspace-benchmark\test-external-wrapper-harness.ps1 = PASS
powershell -File scripts\taskspace-benchmark\test-e3-start-gate.ps1 = PASS
powershell -File scripts\taskspace-benchmark\test-release-decision.ps1 = PASS
cargo test -p codex-core active_context_replacement --lib -- --nocapture = PASS, 121 tests
cargo build -p codex-cli --bin whale --profile dev-small = PASS
```

真实 runner preflight 也已验证不再需要 `-AllowStaleWhaleBin`：

```text
RunRoot = target\r3-engineering-closeout-attested-planonly
WhaleBin = D:\BuildCache\whalecode\cargo-target\dev-small\whale.exe
Health = target\r3-engineering-closeout-attested-planonly\whale-binary-preflight-health.json
status = pass
stale_for_codex_source = false
build_attestation_status = pass
```

## 5. 工程层结论

R3 工程层 code complete 为 true。

具体含义：

1. R3 plan 中写到的工程目标、脚手架、门禁和可执行证据已经补齐到当前 HEAD。
2. 旧的 `AllowStaleWhaleBin` 诊断依赖已被 attestation 机制替代。
3. marker 不再靠测试手写，code-complete 可以由正式脚本生成。
4. start gate 正确阻断 full E3，因为用户授权 marker 缺失。

剩余事项不是工程层代码缺口：

1. 用户确认是否允许生成 `v005-user-approval.json`。
2. 授权后运行 formal E3。
3. E3 后进行人工 audit 和 release decision。
