# Problem P-001: 0.147 rebase 后缓存验收缺失 Provider wire trace
- Status: fixed
- Created: 2026-08-21 05:07
- Updated: 2026-08-21 05:35
- Objective: 恢复真实缓存验收对 Standard 与 TaskSpace Provider wire trace 的完整采集和结算。
- Symptoms:
  - Standard 真实运行 CLI exit 0、Provider 边界计数 7，但 runner 因找不到 `pair-001/left/artifacts/provider-wire-trace.jsonl` 判定失败。
- Expected behavior:
  - 每个真实 arm 都产出可关联的 Provider wire trace，runner 完成 usage、缓存和业务结果验证。
- Actual behavior:
  - Standard arm 已完成 Provider 调用，但证据收集阶段抛出 FileNotFoundError，map-request 按停止条件未启动。
- Impact:
  - Codex 0.147 rebase 的缓存敏感面无法晋升基线，分支不能完成最终合入收口。
- Reproduction:
  - 在提交 `d7e9ded3cf5614c58fa4373a712347bb2c95a775` 上运行提案 `CBP-E38AD62811CCDD3A`，记录 `WAR-20260821-050539-CACHE-REGRESSION-E2F72457`。
- Environment:
  - Linux；分支 `whalecode-codex`；workspace-local Whale SHA256 `f5338c5ed4c9c001dc6fb6a14d5f6c0748b0f5332727fee6304f479673242f74`；模型 `deepseek-v4-flash`。
- Known facts:
  - CLI exit code 为 0，Provider 边界请求计数为 7，缺失的是 runner 指定路径下的 trace 文件。
  - 当前授权零重试，失败后没有启动 map-request。
  - benchmark 已向容器注入 `WHALE_PROVIDER_WIRE_TRACE_PATH=/artifacts/provider-wire-trace.jsonl`。
  - 迁移提交 `f881e71b2` 删除 `provider_wire_trace.rs` 及其 ModelClient 接线；当前源码没有该环境变量的消费者。
- Ruled out:
  - `--taskspace` help 入口缺失已修复；本次失败发生在 Standard arm 完成后。
- Fix criteria:
  - 根因经产物目录、配置/代码路径和离线定向复现至少两类证据确认。
  - 修复后无 Provider 的定向测试通过，并在新授权真实运行中 Standard 与 map-request 均产出完整 trace、usage 与业务成功证据。
- Current conclusion: 根因是 0.147 迁移删除了 Provider wire trace producer 与 ModelClient 接线，但保留了 benchmark consumer 和环境变量合同。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Resolution basis:
  - H-002 confirmed by E-002/E-003 and repaired by commit `93ef626dda6254ca94cb39ad266baeef5415f5d4`; E-004/E-005 validate the restored producer offline and against the real DeepSeek Responses route.
- Close reason:
  - Standard and TaskSpace arms both emitted complete provider wire traces with `trace_coverage=1.0` and complete usage. The TaskSpace arm's separate execution failure is tracked independently and does not invalidate the trace repair.

## Hypothesis H-001: 0.147 改变了 trace 产物路径而 collector 仍读取旧路径
- Status: refuted
- Parent: P-001
- Claim: Whale 已生成等价 trace，但文件名或目录迁移，cache runner 仍硬编码旧的 `artifacts/provider-wire-trace.jsonl`。
- Layer: root-cause
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - CLI 成功且其他边界证据存在，rebase 同时改造了上游 exec/app-server 路径。
- Falsifiable predictions:
  - If true: run root 内存在包含 Provider request/wire 事件的其他文件，或 0.147 写入代码指向不同路径。
  - If false: run root 中完全没有等价 trace，且写入配置未启用或写入器报错。
- Diagnostic evidence plan:
  - Prediction or clause under test: run root 内应存在等价 trace 或新路径标记。
  - Signal: 完整产物文件清单、文件名搜索、事件 schema 搜索及 producer/collector 代码路径。
  - Capture method: 只读 `find`/`rg` 与代码检查，不启动模型。
  - Event name or marker:
    - `provider-wire-trace`
    - `taskspace-provider-request-event-v1`
  - Correlation keys:
    - `WAR-20260821-050539-CACHE-REGRESSION-E2F72457-CACHE-001`
  - Differentiates from:
    - H-002
    - H-003
  - Supports if:
    - 发现等价事件位于 collector 未读取的新路径。
  - Refutes if:
    - run root 没有任何等价 trace，且 producer 没有写入成功证据。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: run root 中不存在等价 wire trace；不是路径迁移。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: none
- Blocker:
  - none
- Close reason:
  - E-002 refuted

## Hypothesis H-002: exec 入口没有启用 0.147 trace writer 所需配置
- Status: confirmed
- Parent: P-001
- Claim: benchmark 传入的 trace 配置在 0.147 exec/app-server 配置层未生效，导致运行成功但从未创建 trace 文件。
- Layer: interaction
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - rebase 后 exec 改为 in-process app-server，配置传递边界发生变化。
- Falsifiable predictions:
  - If true: `whale-argv.json` 含 trace override，但 resolved config/运行事件缺失对应启用状态，producer 创建条件为 false。
  - If false: resolved config 明确启用 writer，且运行日志显示 writer 已创建或尝试写入。
- Diagnostic evidence plan:
  - Prediction or clause under test: CLI 参数到 Config 再到 writer 的启用值是否完整传递。
  - Signal: whale argv、resolved config、manifest、writer 构造条件和相关事件。
  - Capture method: 只读比较运行证据与 Rust/PowerShell 代码。
  - Event name or marker:
    - `provider_wire_trace`
  - Correlation keys:
    - `WAR-20260821-050539-CACHE-REGRESSION-E2F72457-CACHE-001`
  - Differentiates from:
    - H-001
    - H-003
  - Supports if:
    - 参数存在但配置在 app-server 边界丢失，或 producer 明确未启用。
  - Refutes if:
    - writer 配置和实例均已启用。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
  - E-003
  - E-004
- Conclusion: 参数成功传入容器，但迁移删除 producer 和 ModelClient 接线，导致配置无人消费。
- Repair design readiness: ready
- Next step: 将历史已验证 wire trace producer 适配到 0.147 ModelClient 请求与终止边界。
- Blocker:
  - none
- Close reason:
  - Fixed by `93ef626dda6254ca94cb39ad266baeef5415f5d4` and validated by E-004/E-005.

## Hypothesis H-003: trace writer 已启用但写入或收尾失败且错误未传播到 CLI
- Status: refuted
- Parent: P-001
- Claim: writer 已创建，但目录、权限、flush 或 shutdown 失败仅记录为非致命日志，导致 CLI exit 0 而文件不存在。
- Layer: interaction
- Factor relation: single
- Depends on:
  - none
- Rationale:
  - 成功退出与证据缺失也可能由 best-effort telemetry 写入失败造成。
- Falsifiable predictions:
  - If true: stderr、rollout 或事件日志存在创建/写入/flush 错误，或目录权限与 writer 目标冲突。
  - If false: 没有 writer 实例或目标路径实际迁移。
- Diagnostic evidence plan:
  - Prediction or clause under test: writer 生命周期是否产生失败信号。
  - Signal: stderr、容器日志、事件流、目录权限与 shutdown 代码。
  - Capture method: 只读日志搜索和 writer 错误传播代码检查。
  - Event name or marker:
    - `provider_wire_trace_write_failed`
    - `flush`
  - Correlation keys:
    - `WAR-20260821-050539-CACHE-REGRESSION-E2F72457-CACHE-001`
  - Differentiates from:
    - H-001
    - H-002
  - Supports if:
    - 捕获到与目标文件相关的写入或 flush 失败。
  - Refutes if:
    - writer 未启用或等价 trace 已在其他路径成功写入。
  - Instrumentation status: none
  - Instrumentation lifecycle:
    - none
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-003
- Conclusion: 当前源码没有 writer 实例，因而不是已启用 writer 的写入或 flush 失败。
- Repair design readiness: blocked until Status is confirmed and Evidence gate is satisfied
- Next step: none
- Blocker:
  - none
- Close reason:
  - E-003 refuted

## Evidence E-001: 真实 Standard 运行成功但证据物化失败
- Related hypotheses:
  - H-001
  - H-002
  - H-003
- Direction: neutral
- Type: reproduction
- Source: `benchmarks/cache-regression/results/WAR-20260821-050539-CACHE-REGRESSION-E2F72457.json`
- Prediction or plan link:
  - P-001 症状复现边界；尚不能区分三项假设。
- Matched signal:
  - CLI exit 0、Provider request count 7、指定 trace 路径 FileNotFoundError。
- Correlation keys:
  - `WAR-20260821-050539-CACHE-REGRESSION-E2F72457-CACHE-001`
- Raw content:
  ```text
  status=failed; exit_code=0; provider_boundary_request_count=7;
  evidence_error=FileNotFoundError: .../pair-001/left/artifacts/provider-wire-trace.jsonl
  ```
- Interpretation: Agent 业务进程没有失败；故障位于 trace 生成或消费链路，现有事实不足以选择具体根因。
- Time: 2026-08-21 05:07

## Evidence E-002: run root 无等价 wire trace 但环境变量已正确注入
- Related hypotheses:
  - H-001
  - H-002
- Direction: supports
- Type: environment
- Source: `target/cache-hit-regression/WAR-20260821-050539-CACHE-REGRESSION-E2F72457/.../pair-001/left/artifacts/container-inspect-agent.json` 与完整文件清单
- Prediction or plan link:
  - H-001/H-002 的产物路径与配置传递预测。
- Matched signal:
  - 容器环境包含目标路径；目录内没有该文件或其他等价 provider wire trace，request facts 的 trace event count 为 0。
- Correlation keys:
  - `WAR-20260821-050539-CACHE-REGRESSION-E2F72457-CACHE-001`
- Raw content:
  ```text
  WHALE_PROVIDER_WIRE_TRACE_PATH=/artifacts/provider-wire-trace.jsonl
  provider_request_event_count=0
  expected path .../left/artifacts/provider-wire-trace.jsonl does not exist
  ```
- Interpretation: collector 路径与注入合同一致，H-001 被否定；缺失发生在 producer 侧。
- Time: 2026-08-21 05:10

## Evidence E-003: 迁移提交删除 producer 和全部 ModelClient 接线
- Related hypotheses:
  - H-002
  - H-003
- Direction: supports
- Type: code-location
- Source: `git show f881e71b2 -- third_party/codex-cli/codex-rs/core/src/provider_wire_trace.rs` 与当前源码搜索
- Prediction or plan link:
  - H-002/H-003 的 writer 构造与生命周期预测。
- Matched signal:
  - `f881e71b2` 删除 902 行 producer；旧 client 的 `from_env`、`begin_logical_request`、`record_request`、`record_terminal` 接线均不在当前 client 中。
- Correlation keys:
  - commit `f881e71b2f79bdcafbe800b1bfd9500edc940e58`
- Raw content:
  ```text
  core/src/provider_wire_trace.rs | 902 ---------------------
  delete mode 100644
  current rg WHALE_PROVIDER_WIRE_TRACE_PATH: no Rust consumer
  ```
- Interpretation: H-002 的具体机制被直接代码证据确认；H-003 不成立，因为没有 writer 可发生写入失败。
- Time: 2026-08-21 05:11

## Evidence E-004: 0.147 trace producer 与完整 CLI 离线验证通过
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source: 当前工作树定向 Rust 测试、CLI 编译和缓存控制面测试
- Prediction or plan link:
  - H-002 修复方向：producer 恢复并接入 0.147 Responses HTTP/WebSocket dispatch 与 terminal 边界后应可编译、写出 v11 request/terminal 事件且不破坏 collector 合同。
- Matched signal:
  - `cargo test -p codex-core provider_wire_trace --lib` 通过 23 项；`cargo check -p codex-cli` 通过；`python3 -m unittest discover -s scripts/cache-regression -p 'test_*.py' -v` 通过 232 项。
- Correlation keys:
  - repair worktree after commit `9f4949924`
- Raw content:
  ```text
  provider_wire_trace: 23 passed, 0 failed
  codex-cli: cargo check passed
  cache-regression: 232 tests passed
  ```
- Interpretation: producer、0.147 类型适配与 collector 控制面在无 Provider 条件下闭合。
- Time: 2026-08-21 05:21

## Evidence E-005: 真实 Standard 与 TaskSpace 均恢复完整 wire trace
- Related hypotheses:
  - H-002
- Direction: supports
- Type: fix-validation
- Source: `benchmarks/cache-regression/results/WAR-20260821-052740-CACHE-REGRESSION-8AF3D2BC.json`
- Prediction or plan link:
  - P-001 修复标准：两个 arm 都必须产出可关联 trace 与完整 usage。
- Matched signal:
  - Standard 8 个请求、TaskSpace 1 个请求，两侧 `trace_coverage=1.0`、`cache_usage_missing_count=0`；总 usage 为 input 75481、cached 64256、uncached 11225、output 1992。
- Correlation keys:
  - `WAR-20260821-052740-CACHE-REGRESSION-8AF3D2BC`
- Raw content:
  ```text
  standard: provider_requests=8 trace_coverage=1.0 cache_usage_missing_count=0 business_success=true
  map-request: provider_requests=1 trace_coverage=1.0 cache_usage_missing_count=0 business_success=false
  ```
- Interpretation: wire trace 故障已修复；map-request 的业务失败来自独立的 TaskSpace response finalize/dispatch 时序问题。
- Time: 2026-08-21 05:35
