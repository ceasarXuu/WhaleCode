# R5-K2.0 无行为 archive 基建结果

- 日期：2026-07-13
- 状态：COMPLETE
- Candidate code commit：`36beb138f33b5eb4e0ad114512fe34a9807a4969`
- Live artifact：`target/r5-map-compression/K2.0-parity-r2`

## 1. 实施结果

新增 dormant `ProjectionArchivePayload` codec，使用 `output-ref://sha256/<sha256>` 内容寻址。codec 对 node、edge、
result 和 node event 做确定性排序，校验重复 ID、ownership、entry/internal/exit edge 边界和 payload hash。

本阶段没有从 `ActionMapRuntime`、projection renderer、session 或 tool handler 调用 codec；provider projection、tool
schema、状态推进和 replay 路径均无新增分支，strategy activation 为 0。

## 2. 验证

| 验证 | 结果 |
|---|---|
| 100/1,000/10,000 nodes encode -> decode -> re-encode | 3/3 PASS，bytes/ref/hash 稳定 |
| empty/duplicate/hash corruption/invalid boundary | 全部显式拒绝 |
| PowerShell super-run 计划自检 | 18/18 arm tasks 正确，repeat 顺序轮换 |
| simple live STD/B0/C | 3/3 business + validator PASS |
| complex live STD/B0/C | 3/3 business + validator PASS |

首次 live 启动因外层进程未读取 `.env.local`，6 个 arm 全部在 provider preflight 前以
`provider_credential_missing` 退出，没有发出模型请求。runner 已改为仅在进程环境缺 key 时读取 `.env.local` 的指定
credential 键；值不写入日志、manifest 或命令行。第二次运行通过。

## 3. 成本解释

单次 live 中 C 相对 B0 的 requests 为 simple `13/7`、complex `18/9`，不能作为 K2.0 行为回归归因：

- codec 在 production call graph 中没有调用点；
- simple C 的 Agent 额外创建了 2 个重复节点，随后因自己遗留 open nodes 收到 3 次 state-machine reject；
- complex C 比 B0 多执行 13 个普通工具调用，control failure 均为 0；
- 所有 arm 最终均通过 validator。

因此，本阶段只声明“正常路径正确、provider 行为没有代码级新增分支”，不声明随机成本等价。后续 S1 的正式门禁
仍按 3 repeats；K2.0 的单次波动不会被当作 S1 收益或损失。

## 4. 工程经验

1. 不应把 binary 身份绑定到之后的纯文档/runner commit；super-run 分离 `orchestrator_commit` 与最后一次
   `codex_source_latest_commit`，同时严格校验 binary SHA。
2. 冻结 B0 因源码继续演进会触发内部 stale 检查。外层必须先验证 tracked manifest 中的 B0 binary/attestation
   SHA，才允许 child runner 使用 `AllowStaleWhaleBin`。
3. live 单次轨迹可用于发现故障和解释动作，不能代替 dormant 代码的确定性 call-graph 证据。
