# R4 organization-json-generator 工具链收敛补充记录

日期：2026-07-02

本记录补充 `organization-json-generator` public sample 的真实现场。该样本用于验证 R4 工具调用链路在长流程数据生成任务中的表现，不能只看最终 solved/wrong，需要拆分工程链路是否前进、工具反馈是否进入模型上下文、以及剩余失败是否属于模型补丁质量。

## 1. 修复项：validation coverage gate 进入专用 recovery

失败现场：

```text
Before:
C:\WhaleRunCache\r4-public10-20260702\actual\organization-json-generator-v1\runs\terminal_bench__organization-json-generator\20260702-055624-750\pair-001

outcome_standard=solved
outcome_taskspace=agent_exec_timeout
taskspace_exec_timed_out=true
taskspace_wall_ms=900041
nodes=5
edges=455
accepted_results=0
unreviewed_results=20
open_leaf_nodes=2
```

根因：

```text
ActionMap 已经给出 TaskSpaceGateRecoveryV1:
reason=validation_test_missing_changed_artifact_coverage
next_valid_actions=run_test with command `python generate_organization.py`

但 session 层没有把该 gate reason 归入 TaskSpaceValidationNeedsTestRecoveryV1，
因此模型可见恢复指导落入 generic no-action recovery，容易继续验证错误对象。
```

修复：

```text
third_party/codex-cli/codex-rs/core/src/session/turn.rs

1. taskspace_message_hit_validation_needs_test(...) 识别：
   - validation_test_missing_changed_artifact_coverage
   - validation_test_missing_local_validator_coverage

2. build_taskspace_validation_needs_test_recovery_item(...) 保留原始 TaskSpaceGateRecoveryV1 payload，
   包括 next_valid_actions。
```

验证：

```text
cargo fmt --all -- --check
PASS

cargo test -j1 -p codex-core validation_changed_artifact_coverage_recovery_preserves_next_action --lib
PASS

cargo test -j1 -p codex-core validation_needs_test_recovery_blocks_discovery_loop --lib
PASS

cargo test -j1 -p codex-core no_action_recovery_preserves_recent_gate_recovery_context --lib
PASS

cargo build -j1 --profile dev-small -p codex-cli --bin whale
PASS
```

## 2. 真实复验收益

复验现场：

```text
After:
C:\WhaleRunCache\r4-validation-coverage-org-json-20260702\runs\terminal_bench__organization-json-generator\20260702-223232-599\pair-001

outcome_standard=solved
outcome_taskspace=engineering_unclean
taskspace_exec_timed_out=true
taskspace_wall_ms=420117
tool_call_count=15
nodes=6
edges=5
accepted_results=3
unreviewed_results=17
open_leaf_nodes=1
changed_paths_taskspace=generate.py
```

真实收益判断：

| 维度 | 修复前 | 修复后 | 判断 |
|---|---:|---:|---|
| agent 执行时长 | 900041 ms | 420117 ms | 明显降低，但仍超时 |
| graph edges | 455 | 5 | 工程链路爆炸已收敛 |
| accepted results | 0 | 3 | TaskSpace 状态推进有效 |
| open leaf nodes | 2 | 1 | 未完全收敛 |
| 是否复现旧 coverage gate loop | 是 | 否 | 原问题已修复 |
| 最终业务结果 | timeout | timeout | 未达到 utility parity |

结论：

- 本修复有真实工程收益：旧的 validation coverage gate loop 没有复现，图结构从 455 edges 收敛到 5 edges。
- 本修复没有证明业务成功率收益：TaskSpace 仍未在 420 秒内完成该样本。
- 该样本仍属于 R4 未完成项，不能计入最终通过。

## 3. 新暴露的未收敛点

复验中已经确认工具反馈进入模型上下文：

```text
result-14:
python generate.py
IndentationError: unexpected indent

result-18:
python generate.py
IndentationError: unindent does not match any outer indentation level

TaskSpaceImplementNeedsEditRecoveryV1 inserted
TaskSpaceApplyPatchUnanchoredUpdateRecoveryV1 inserted
TaskSpaceEditFailureRecoveryV1 inserted
```

新的卡点：

```text
模型收到失败 patch / unanchored patch recovery 后，仍继续输出不可执行或上下文不匹配的 patch。
同时 provider request trace 显示：
request_count=19->20 max=20
state=compact_checkpoint_required->over_profile_hint
```

后续修复方向：

1. 分析 `max_requests=20` 是否仍以硬上限形式影响开放式任务收敛，和“profile 只能作为起始复杂度估算”的要求是否冲突。
2. 针对失败 patch 后的恢复，优先复用 standard 模式的 apply_patch 失败反馈语义，避免 TaskSpace 自己用过强的抽象提示替代原始失败。
3. 对 organization-json-generator 继续做单样本复验，验收标准不是一次性 solved，而是逐层证明：不复现旧 loop、失败 patch 能恢复、最终进入 validator lifecycle。

## 4. 2026-07-03 Linux 接手复验

接手后在 Linux 主机上重新物化 pinned Terminal-Bench source：

```text
source: https://github.com/laude-institute/terminal-bench
branch: dataset/terminal-bench-core/v0.1.x
commit: 91e10457b5410f16c44364da1a34cb6de8c488a5
task: tasks/organization-json-generator
```

本次没有得到 utility 复验结果；先暴露并修复了 harness 前置问题：

| 层 | 失败 | 修复 |
|---|---|---|
| materialization | `WindowsIdentity.GetCurrent()` 在 Linux 不支持 | 非 Windows stale ACL repair no-op；source guard 记录 `windows_acl_unavailable` |
| uv cache | 下载器硬编码 `curl.exe` | 选择 `curl.exe` 或平台 `curl` |
| validation launcher | validator 通过 `cmd.exe /c` 启动 | 直接启动 validator process，stdout/stderr 流式写入 |
| execution alias | `subst` 在 Linux 不存在 | 非 Windows 使用 direct repo dir |
| rollout lookup | `$env:USERPROFILE` 为空导致 Join-Path null | 候选 home 改为非空 `WHALE_HOME` / `USERPROFILE` / `HOME/.whale` |

验证：

```text
test-terminal-bench-uv-cache-harness.ps1 PASS
test-external-wrapper-harness.ps1 PASS
test-oracle-runner-harness.ps1 PASS
test-terminal-bench-adapter-harness.ps1 PASS
test-metrics-extractor-harness.ps1 PASS
test-harness.ps1 PASS

Plan-only:
target/r4-org-json-plan-20260703/.../20260703-053756-940
PromptInvalid=False
PromptManualReview=False
```

真实复验当前阻塞：

```text
target/r4-org-json-real-20260703e/.../20260703-054112-829

agent:
Missing environment variable: DEEPSEEK_API_KEY

validator:
Docker build failed at RUN pip install jsonschema because proxy resolution failed.
```

后续修复：

```text
run-taskspace-benchmark.ps1
provider-credential-preflight-health.json
stable_code=provider_credential_missing
```

复验：

```text
target/r4-org-json-real-20260703f/.../20260703-054256-842
exit_code=3
abort_phase=provider_credential_preflight
```

当前结论：
- H-035/H-036 已由聚焦单测排除；本轮没有进入 patch recovery utility 层。
- Linux harness 已能完成物化和前置诊断，不再被 Windows primitive 阻断。
- 继续该样本前必须先配置 `DEEPSEEK_API_KEY`，并修复 Docker build 的 Python package/proxy 环境；否则任何 TaskSpace utility 结论都不成立。
