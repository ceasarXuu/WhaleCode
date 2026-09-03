# WhaleCode v0.0.7

- 状态：规划中
- 创建日期：2026-08-31

本目录集中维护 WhaleCode v0.0.7 的版本主题。当前已建立：

- [Codex CLI 0.151 主线追赶](codex-upstream-sync/plan.md)：从当前官方 `rust-v0.149.0` vendor 追赶到稳定版 `rust-v0.151.0`，优先吸收权限、安全、稳定性和执行效率修复，同时保护 v0.0.6 多 Provider、DeepSeek Responses 与 TaskSpace 语义。
- [开发版与 Release 完全隔离 PRD](dev-release-isolation-prd.md)：固定全局 `whale` 为 release、全局 `whale-dev` 为按 cwd 路由的开发入口，并让多个 worktree 的 binary 与所有可写状态完全隔离。
- TaskSpace projection policy 回归修复：在 `whale-dev` 恢复 `/map-request`、`/map-append`、`/map-always`，选择即时生效并写入 workspace 隔离配置；未显式选择时仍默认 `map-request`。

本目录的建立不代表 v0.0.7 已进入发布准备，也不授权自动触发 GitHub Actions、真实 Whale Agent 请求或外部发布。
