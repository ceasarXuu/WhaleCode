# Phase B4 VA-01 固定离线验收

- 日期：2026-08-09
- 代码基线：`3601dcf0b`
- 状态：通过
- 真实 Whale Agent / Provider 请求：0

## 1. 验收范围

本轮只执行 Phase B3/B4 已有的冻结检查，不新增测试框架，也不使用真实 API Key。覆盖 Docker benchmark 镜像、
TaskSpace Exec、关系化 Map Store、Action 结算与恢复、Standard final wire、CLI/TUI Viewer、App Server Protocol、
workspace 构建、zero-base 和缓存门禁。

## 2. 结果

| 检查 | 结果 |
|---|---|
| Docker benchmark 镜像构建 | PASS；产出 `sha256:55a8ac...e0ca`，全部复用既有缓存层 |
| `DEEPSEEK_API_KEY=test cargo test -q -p codex-core --lib` | 1856 passed / 3 ignored |
| `cargo test -p codex-core taskspace_exec --lib` | 57 passed |
| `cargo test -p codex-core taskspace_action_settlement --lib` | 11 passed |
| `cargo test -p codex-state --lib` | 134 passed |
| `cargo test -p codex-core --test all cache_final_wire` | 1 passed |
| `cargo test -p codex-cli --test debug_taskspace_map` | 5 passed |
| `cargo test -p codex-tui action_map -- --test-threads=1` | 4 passed |
| `cargo test -p codex-app-server-protocol --lib` | 183 passed |
| `cargo check --workspace --all-targets` | PASS；既有 warning 不变 |
| TaskSpace zero-base gate | 6 tests + repository scan PASS |
| cache regression gate，`source=index` | PASS；fingerprint `b966785b...7cbdb` |

缓存门禁当前仍将已知 policy 变化保持为发布阻断，等待后续获批的真实缓存验证；这不影响 VA-01 的离线工程结论。

## 3. 验收中发现并修复的问题

TUI 的 App Server Viewer 集成测试最初失败，包含两个陈旧测试前提：

1. fixture 没有把 projection policy 写入 Embedded App Server 实际重新读取的临时 `config.toml`，因此 Core 按产品合同
   正确拒绝了 TaskSpace 激活；
2. 同一重型 App fixture 只在 Windows 使用 8 MiB 测试线程，Linux 默认测试栈在当前依赖规模下发生栈溢出。

提交 `3601dcf0b` 仅修复测试：fixture 显式写入合法的 `map-request` policy，并在所有平台复用已有的大栈包装。生产 API、
TaskSpace 模式、Tool、Map 和缓存输入均未改变。修复后 Viewer 页面合同和 App Server 真实路由共 4 项全部通过。

Core 全库首轮另有 9 项 Guardian/模型刷新测试失败，日志均显示缺少 `DEEPSEEK_API_KEY`，尚未进入各自的本地 wiremock。
按 B3 冻结口径使用虚拟值 `test` 重跑后 9 项全部通过；没有读取 `.env.local` 的真实 key，也没有外部 Provider 请求。

## 4. 结论

当前生产链、持久化事实、消费者和 Standard final wire 均通过固定离线验收。VA-01 完成；后续 VA-04A 已基于当前
源码和确定性证据重新映射 R8 的 I01～I10，结果见
[`23-phase-b4-issue-remap-result.md`](23-phase-b4-issue-remap-result.md)。
