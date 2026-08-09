# Phase B5 VA-02 首次生产结构验证

- Date: 2026-08-09
- Status: Failed at first structural gate / no retry
- Record: `WAR-20260809-195732-CACHE-REGRESSION-4E4DA2D5`
- Subject commit: `9e847e626581efced23194f15d672f741d1d061a`
- Observability repair: `cca76e921`

## 1. 授权与实际消耗

本次授权仅覆盖 `single-file-fast-fix × map-request × repeat=1`，使用 `deepseek-v4-flash`，最多 2 个 Provider
请求且不允许重试。任一结构、业务、usage 或预算异常立即停止。

| 指标 | 实际值 |
|---|---:|
| Sample run | 1 |
| Provider request | 1 |
| Input token | 11,715 |
| Cached input token | 0 |
| Uncached input token | 11,715 |
| Output token | 108 |
| 墙钟时间 | 40.347 秒 |
| 冻结价格估算 | USD 0.00167034 |
| 重试 | 0 |

容器、网络和临时 secret 清理均为 `verified_absent`。第二个请求没有发生，剩余上限不作为可复用重试预算。

## 2. 实际响应路径

最终 Provider 请求中：

- TaskSpace capability identity 为
  `2e5e1e38fd7caa185aed6d1b8a85f804eab3acd0d6e69efbaa6c5c1adf5ba73d`；
- 顶层 Tool 数为 2，`tool_choice=auto`；
- Tool declaration 为 21,182 bytes，约 5,296 tokens；
- active Map projection 尚不存在，符合首次请求事实。

目标模型没有生成顶层 `taskspace_exec`，而是生成了未声明的顶层 client call：

```json
{
  "name": "exec_command",
  "arguments": {
    "cmd": "ls -la /workspace && cat /workspace/README* 2>/dev/null | head -100",
    "node_id": "root"
  }
}
```

`exec_command` 只作为 `taskspace_exec.calls[]` 的内部能力向模型说明，并未由 Runtime 作为顶层 Tool 暴露。模型将内部能力
名称提升成了顶层 Function Call。Runtime 按产品合同拒绝该响应：

```text
TaskSpace response contains forbidden top-level client Tool `exec_command`
```

拒绝发生在任何 client Tool 或 Map 副作用之前：命令未执行、Map 未初始化、节点与 Action 数均为 0、工作区无业务修改。
因此 Runtime 硬门行为正确，但 VA-02 的“目标模型可生成正式生产合同”验收没有通过。

## 3. 观测链发现

首次 runner 结算还暴露了一个独立的 I07 工程缺口：Rust producer 已输出
`provider-chat-wire-trace-v11`，而 canonical request facts 及若干 active consumer 仍只接受 v10，导致运行结束时错误报告
`wire_schema_unsupported`，token 与费用一度无法结算。

提交 `cca76e921` 已将 active producer、canonical parser、直接消费者和 fixture 统一到 v11，并修正 TaskSpace Exec observer
对 `canonical + local_attempt` 的真实消费声明。原始不可变 trace 随后通过现有派生器离线重建，得到 100% trace coverage、
0 usage gap 和上表 token/费用。账本仍保持 `failed`，结果仍保持 `partial`，并明确保存原始解析错误和离线重分析路径；没有
把失败运行改写为成功。

## 4. 当前归因边界

已经坐实：

1. 最终生产 Router 没有把普通 client Tool 作为顶层 Tool 暴露；不是 Runtime Tool 可见性泄漏。
2. 模型看到了内部能力名称和参数合同，但没有遵守外层调用形状。
3. Runtime 的拒绝符合不可绕过硬约束，没有执行、补全、重排或猜测 Agent 动作。
4. 当前请求的 developer 内容是通用工作说明；零基线删除旧协议说明后，没有一份活动的 TaskSpace 工作协议系统地说明
   “client/Map 动作只能写入 `taskspace_exec.calls[]`，不得把内部能力作为顶层调用”。

尚未坐实：

- 仅补充最小 TaskSpace 工作协议后，目标模型是否能稳定生成 outer Exec；
- 问题是否还包含 DeepSeek 对嵌套 Function 能力表达的独立遵循限制；
- 三种 projection 的业务效果与成本。首次请求未进入 Map 工作闭环，不能用于这些结论。

## 5. 停点与候选方向

VA-02 已触发 Phase B5 的“首个结构性失败即停”。VA-03 四臂测量不得开始。

建议的最小下一步是恢复一段 TaskSpace 专用、Agent 可见的工作协议，只说明外层调用模型和首次行动方式：client/Map 动作
必须作为 `taskspace_exec.calls[]` 条目；不得顶层调用内部能力；首次可在同一个 outer Exec 中初始化 Map 并执行工作。硬合同
继续由 Tool schema 和 Runtime preflight 负责，协议不复制 Tool 参数、不解释结果、不替 Agent 选择动作。

这是 Agent 输入与产品工作方式的变更，会触及缓存敏感面。实施前需要用户确认；实施后必须先通过缓存门禁，再另行申请
一次新的真实 Provider 预算。不得用强制 `tool_choice`、Runtime 自动包裹非法调用或接受顶层 client Tool 代替该决策。

## 6. 证据

- Result: `benchmarks/cache-regression/results/WAR-20260809-195732-CACHE-REGRESSION-4E4DA2D5.json`
- Ledger: `benchmarks/whale-agent-run-ledger.json`
- Authorization: `benchmarks/cache-regression/authorizations/CBA-20260809-B5-VA02-46EACBE1F82A5403.json`
- Original evidence: `benchmarks/cache-regression/evidence/WAR-20260809-195732-CACHE-REGRESSION-4E4DA2D5/`
- Offline v11 reanalysis: `benchmarks/cache-regression/evidence/WAR-20260809-195732-CACHE-REGRESSION-4E4DA2D5/WAR-20260809-195732-CACHE-REGRESSION-4E4DA2D5-CACHE-001-OFFLINE-REANALYSIS-V11/`

## 7. 后续修复状态

用户确认继续参考最新 Codex 主线收敛协议后，VA-02R 已完成离线实现。修复没有向 base instructions 再增加一份
TaskSpace 调用说明，而是让 `taskspace_exec` 自身的 model-visible description 成为唯一外层操作合同；具体证据见
[`25-phase-b5-protocol-authority-repair.md`](25-phase-b5-protocol-authority-repair.md)。本文仍保留首次失败事实，不把离线修复
改写成真实 Provider 已通过。VA-02 只有在新预算下复验成功后才能关闭。
