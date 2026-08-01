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

不能通过放开 `model_providers.deepseek` 覆盖来修复。上游 Codex 明确把内置 provider ID 设为不可覆盖，并对内置
OpenAI 单独提供顶层 `openai_base_url`；这表明“保留 provider 身份 + 专用 endpoint 配置”是现有架构模式，而非
把内置 provider 伪装为用户自定义 provider。

首选方向是为 Whale 内置 DeepSeek 增加同构的 `deepseek_base_url`，默认仍为官方
`https://api.deepseek.com`，benchmark 仅在隔离容器内把它设置为 provider boundary。这样不改变 provider ID、认证变量、
Responses wire 或模型选择。参考：[Codex config schema](https://github.com/openai/codex/blob/main/codex-rs/core/config.schema.json)、
[DeepSeek Agent 集成配置](https://api-docs.deepseek.com/quick_start/agent_integrations/deepcode)。

RunId 则应保持更简单的机械合同：传入显式 ID 时，新目录就使用该 ID；未传入时才生成时间戳。修复必须覆盖非法路径、
已存在目录、resume 和 force，避免为找回证据引入模糊扫描 fallback。

## 4. 证据边界

本次只证明 binary preflight 已恢复，以及两个 harness 阻塞的根因。Standard 与 map-request 都没有形成可比较的缓存数据，
不得晋升 accepted baseline，也不能用于判断两臂缓存收益。两次一次性授权均已消费；修复后真实复验需要新的用户预算。
