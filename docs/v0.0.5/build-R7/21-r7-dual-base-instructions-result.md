# R7 双基础提示词实施结果

- 日期：2026-07-20
- 实现提交：`5ecfefbcd`
- 观测提交：`de7a8f547`
- 状态：单组 Docker 诊断验收通过

## 1. 结果

WhaleCode 现在按会话工作方式选择一份完整 `base_instructions`。Standard 使用保留 Codex 成熟框架、只替换
品牌暴露的完整 base；TaskSpace 使用同一框架并把 Map 方法有机融合进工作方式。旧的极简 base 与独立
TaskSpace developer protocol 已退出生产路径。

模式、base 和计划工具来自同一次 SessionState 快照。Standard 保留 `update_plan`，TaskSpace 使用
`taskspace_control`；启动预热、resume 和模式切换使用相同选择器。provider wire trace v5 逐请求记录 base
profile、版本、哈希、位置和估算成本。

## 2. 验证

通过的工程门禁：

- `cargo test -p codex-core base_instructions --lib`：9 passed。
- `cargo test -p codex-models-manager`：35 passed。
- `cargo test -p codex-core --test all taskspace_terminal_contract`：2 passed。
- `cargo test -p codex-core provider_wire_trace --lib`：17 passed。
- 双 base 合同、成本观测、性能报告自测通过。
- `cargo build -p codex-cli --bin whale --locked` 通过。

Docker 使用 `deepseek-v4-flash`、`map-request`、同一二进制和 hard boundary：

| 模式 | 结果 | Requests | Tools | 时间 | Input | Cached | Uncached | Output | Req2+ cache | Map N/E |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Standard | solved | 6 | 8 | 13.471s | 65,949 | 54,656 | 11,293 | 1,430 | 97.46% | 0/0 |
| TaskSpace | solved | 7 | 7 | 22.244s | 95,545 | 81,536 | 14,009 | 2,855 | 96.95% | 5/4 |

两侧公开与隐藏验证均通过，TaskSpace 最终没有开放节点。

## 3. Base 身份

| 模式 | 请求覆盖 | Profile | Version | SHA-256 | 估算 tokens/请求 |
|---|---:|---|---|---|---:|
| Standard | 6/6 | `standard` | `1.0.0` | `7c27bcb...f19` | 5,313 |
| TaskSpace | 7/7 | `taskspace` | `1.0.0` | `95f6cc4e...d55` | 5,503 |

每个请求的 identity count 都为 1，合同匹配率为 100%，没有旧 developer protocol 或 profile 混用。
TaskSpace base 比 Standard 多约 190 estimated tokens/request。TaskSpace 总输入为 Standard 的 1.45 倍，
但差值同时包含多 1 次请求、更大的工具 schema、控制反馈和自然历史，不能归因为 base 固定成本。

## 4. 证据边界

本轮只有一个简单样本、每侧一次，证据等级为 E2-candidate。它足以证明双 base 接线、身份日志、工具路径和
基本行为可用，不足以判断长期质量收益或总体成本。后续三种 projection policy 的正式多次对照继续由 Phase
E/G 承担。

机器结果：`benchmarks/taskspace/r7/dual-base-instructions-v1.0.0-result.json`。
