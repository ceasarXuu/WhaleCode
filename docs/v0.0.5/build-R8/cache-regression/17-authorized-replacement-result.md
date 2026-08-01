# R8 获批替代 revalidation 运行结果

- 时间：2026-08-01 13:05
- Record：`WAR-20260801-130559-CACHE-REGRESSION-DDFF3293`
- Subject HEAD：`0490facf13bbef0cc3f75909bccdc9f8271b63be`
- Proposal：`CBP-30AA55032E53B239`
- Authorization：`CBA-20260801-REPLACEMENT-30AA55032E53B239`
- 状态：`partial`，runner exit `3`

## 1. 实际路径

1. binary attestation preflight 已通过，证明首次运行暴露的本机构建问题已修复。
2. Standard 首臂创建了隔离 provider boundary，但 Whale 在读取配置时退出；map-request 按失败即停规则未运行。
3. 运行未自动重试，容器、网络和 host secret 均验证为空。
4. boundary 与 Whale wire trace 完整对账为 0 请求，因此本次 input、cached input、output token 和估算费用均为 0。

## 2. 两个独立阻塞

| 问题 | 直接表现 | 已确认根因 | 影响 |
|---|---|---|---|
| provider 路由 | Whale 报 `deepseek` 是不可覆盖的保留 provider | benchmark 注入 `model_providers.deepseek.base_url`，与内置 provider 合同冲突 | Agent 未启动，无法测缓存 |
| RunId 身份链 | 外层 runner 报 RunId 对应 0 个目录 | 新显式 RunId 被时间戳目录创建器覆盖 | 自动账本最初看不到实际已生成的证据 |

实际 artifact 位于：

`target/cache-hit-regression/WAR-20260801-130559-CACHE-REGRESSION-DDFF3293/single-file-fast-fix/20260801-130559-657`

## 3. 工程判断

不能通过放开 `model_providers.deepseek` 覆盖来修复，也无需为产品新增 `deepseek_base_url`。进一步追踪确认，现有
custom provider 已能在 benchmark 内声明一个 `deepseek-boundary` 传输别名，并完整复现内置 DeepSeek 的名称、认证变量、
Responses wire、重试、超时和 WebSocket 字段；唯一差异是本地 provider ID 和有意指向隔离代理的 base URL。

离线探针证明内置 DeepSeek 与该别名生成的模型可见上下文均为 16,336 bytes，SHA-256 完全相同。修复应仅修改
benchmark，并在 artifact 中同时记录 `logical_provider_id=deepseek` 与
`transport_provider_id=deepseek-boundary`。全局 final-wire 门禁继续保护生产 `provider_id=deepseek`，不得为测试别名
放宽。参考：[Codex config schema](https://github.com/openai/codex/blob/main/codex-rs/core/config.schema.json)、
[DeepSeek Agent 集成配置](https://api-docs.deepseek.com/quick_start/agent_integrations/deepcode)。

当前实现已通过 `ConfigToml` 解析、provider 字段等价、Docker provider boundary 和内置 DeepSeek final-wire payload
回归，但对抗性审查指出这些证据尚未覆盖精确 PowerShell 参数经过真实 CLI/完整 `Config` 的入口，也未直接执行 alias
normal final-wire，正式证据与 ledger 也没有绑定 resolved transport identity。因此工程状态回退为 validating；缓存门禁和
发布继续阻断。详见 `vs_review/2026-08-01-r8-provider-boundary-alias-review.md`。

RunId 保持更简单的机械合同：传入显式 ID 时，新目录就使用该 ID；未传入时才生成时间戳。当前修复已覆盖非法路径、
已存在目录和默认时间戳，并通过 cache regression 与 E3 guardrails 回归，没有引入模糊扫描 fallback。

## 4. 证据边界

本次只证明 binary preflight 已恢复，以及两个 harness 阻塞的根因。Standard 与 map-request 都没有形成可比较的缓存数据，
不得晋升 accepted baseline，也不能用于判断两臂缓存收益。两次一次性授权均已消费；修复后真实复验需要新的用户预算。
