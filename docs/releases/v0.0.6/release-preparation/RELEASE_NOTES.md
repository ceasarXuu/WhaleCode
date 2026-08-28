# WhaleCode v0.0.6（发布说明草稿）

本版本重点补齐多 Provider 使用链路，并更新 DeepSeek V4 Responses 能力。

## 主要更新

- **新增多 Provider 切换**：通过 `/provider` 在 OpenAI 订阅、OpenAI API 与 DeepSeek 之间切换；切换按 turn 边界原子生效，并同步更新模型、提示词、工具、命令、压缩策略和历史投影。
- **统一跨 Provider 模型选择**：`/model` 按访问方式分组展示当前可用模型，支持直接跨组选择；选择会记忆到当前 session，并成为后续新 session 的完整默认路由。
- **隔离三路凭据**：OpenAI 订阅、OpenAI API key 与 DeepSeek API key 可安全共存，登录和登出只影响目标访问方式。
- **完善 DeepSeek V4 Responses**：内置 `deepseek-v4-flash`、`deepseek-v4-pro` 和实验性的 `deepseek-v4-flash-vision-exp`，补齐 Vision 输入、thinking effort、工具边界与 SSE 生命周期处理。
- **增强 Provider 恢复语义**：resume、fork、rollback、subagent 和跨 Provider 历史投影可恢复逐 turn 路由，失败切换保留原 Provider。

## 安装

发布后可使用：

```bash
npm install -g @ceasarxuu/whalecode@0.0.6
```

WhaleCode 产品版本为 `v0.0.6`；Codex `0.149.0` 仅表示本版本采用的底层 substrate 版本。

## 已知限制

- 原生候选仍为未签名制品。
- `deepseek-v4-flash-vision-exp` 是实验模型，服务端行为和模型可用性可能变化。
- DeepSeek hosted web search 的官方能力与当前模型目录展示尚需在发布前统一核验。
- 本说明是候选草稿；六平台制品、npm registry 安装 smoke、tag 与 GitHub Release 尚未执行。
