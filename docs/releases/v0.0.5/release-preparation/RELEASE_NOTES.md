# WhaleCode v0.0.5 发布说明（草稿）

> 状态：准备中，尚未授权发布。

WhaleCode v0.0.5 是以 DeepSeek 为核心的终端 coding agent 候选版本。该版本基于 OpenAI Codex CLI `0.149.0` substrate，但 Whale 产品版本、CLI 版本和产品 tag 均为 `0.0.5` / `v0.0.5`。

## 主要内容

- DeepSeek provider、模型目录、Responses API、reasoning stream 与缓存记账。
- TaskSpace relational state、execute/fork/reload/final-wire 路径及相关可观测性。
- workspace 隔离 bootstrap、二进制 attestation 和本地开发安全门禁。
- Codex 0.149 upstream 同步、overlay provenance 与隔离回归入口。

## 已知边界

- 当前发布准备面向 Whale + DeepSeek + TaskSpace 主路径。
- OpenAI/ChatGPT 登录、OpenAI remote catalog/plugin sharing、Bedrock 和相关上游专属测试不属于当前产品发布合同。
- npm、WinGet、R2 和网站部署渠道尚未确认；vendor 内上游发布工作流不得直接用于 Whale 发布。
- 全量 upstream suite 存在已分类的非产品面失败，不能描述为全绿。

## 版本身份

- WhaleCode：`v0.0.5`
- Codex upstream substrate：`rust-v0.149.0`

两者不得互换登记。
