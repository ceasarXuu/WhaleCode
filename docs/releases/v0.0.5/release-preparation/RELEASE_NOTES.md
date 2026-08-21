# WhaleCode v0.0.5

本版本重点增强复杂任务协作与 DeepSeek 使用体验，并将底层能力追赶至 Codex CLI 0.149。

## 主要更新

- **新增 TaskSpace**：提供可持久化的任务状态、任务分支与恢复能力，支持复杂编码任务在多线程协作中持续推进。
- **完善 DeepSeek 支持**：补齐 DeepSeek provider、模型选择、Responses API、推理流式输出、长上下文压缩和缓存用量统计。
- **追赶 Codex 0.149 新特性**：同步新版 CLI、会话与多 Agent 基础能力，同时保留 WhaleCode 的产品身份和 DeepSeek/TaskSpace 行为。

## 安装

```bash
npm install -g @ceasarxuu/whalecode@0.0.5
```

WhaleCode 产品版本为 `v0.0.5`；Codex `0.149.0` 仅表示本版本采用的底层 substrate 版本。

本版原生二进制未签名；发布制品附带 SHA-256 校验文件。
