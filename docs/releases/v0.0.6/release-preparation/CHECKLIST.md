# WhaleCode v0.0.6 发布清单

## 工程与候选证据

- [x] Rust workspace、lockfile、npm 元包与工作流默认版本统一为 `0.0.6`。
- [x] 产品版本 `v0.0.6` 与 substrate `rust-v0.149.0` 保持独立身份。
- [x] 多 Provider Phase 0–5 完成，14 项验收行为均有自动化测试或可复现证据。
- [x] provider 受影响六 crate 隔离矩阵 9286/9286、app-server protocol 293/293 通过。
- [x] DeepSeek 三模型静态目录、Responses SSE 与 Vision 输入具备离线合同；Vision 单请求真实 smoke 通过。
- [x] 发布说明不宣称三模型 live matrix 完全通过；Flash/Pro usage 证据限制已保留。
- [x] v0.0.6 不向用户展示或宣称 DeepSeek hosted web search；模型目录保持 `supports_search_tool=false`。
- [x] 本地 release preflight 27/27 通过：identity、distribution、brand、npm candidate、native/npm workflow、manual Actions 与 release 单测。
- [x] `cargo metadata --locked --no-deps` 与本地 `whale` 构建通过，二进制输出 `whale 0.0.6`。
- [x] cache-sensitive index gate 通过（fingerprint `3b83125448b51052fe8972cf20c5a5bab818f683a91214dd2eb99ab99eff8c8f`）。
- [x] 最终候选固定为 `78c46f4fc8dba9bda406cd7f7b2e16ca3797b8b4`。

## 原生制品与安装验证

- [x] `whale-native-artifacts` run `33202591530` 在候选提交上完成六平台构建、打包与 manifest 汇总。
- [x] 六个平台齐全：Linux x64/arm64、macOS x64/arm64、Windows x64/arm64。
- [x] 七个 npm tarball、`native-manifest.json` 与 `SHA256SUMS` 已生成并通过完整性回读。
- [x] tarball 清单未混入本机路径、凭据、日志或额外文件。
- [x] release smoke run `33206670227` 在六个平台原生 runner 全部通过，且未生成 `codex` 命令。
- [x] 草稿 smoke 所需临时 `contents: write` 已在验证后恢复为 `contents: read`。

## npm 发布

- [x] 用户明确授权发布 `v0.0.6` tag、七个 npm 版本与公开 GitHub Release。
- [x] 发布前 registry 不存在任何 `0.0.6*` 版本。
- [x] npm Trusted Publisher 已绑定 `.github/workflows/npm-publish.yml`，发布使用 GitHub Actions OIDC，不保存长期 npm 写 token。
- [x] OIDC 工作流保持 `workflow_dispatch`，仅授予 `actions: read`、`contents: read` 和 `id-token: write`。
- [x] npm publish run `33225232511` 验证构建 run 与候选 SHA，校验已存在版本后按平台版本优先、根包最后的顺序完成发布。
- [x] 根包 `0.0.6` 与六个平台后缀版本全部存在；`latest` 和六个平台 dist-tag 均通过 registry 回读。
- [x] 从 npm registry 隔离安装 `@ceasarxuu/whalecode@0.0.6`，输出 `whale 0.0.6`，且只生成 `whale` 命令。

## Tag 与 GitHub Release

- [x] 未签名注释 tag `v0.0.6` 已推送，peeled commit 为 `78c46f4fc8dba9bda406cd7f7b2e16ca3797b8b4`。
- [x] GitHub Release 目标提交与 tag 一致，包含七个 npm tarball、manifest 和 SHA256SUMS 共 9 个资产。
- [x] GitHub Release 于 `2026-08-29T01:06:03Z` 公开：<https://github.com/ceasarXuu/WhaleCode/releases/tag/v0.0.6>。
- [x] GitHub API 回读确认该版本不是 draft/prerelease，并为当前 latest release。
- [x] GitHub Actions 继续默认关闭；所有根 workflow 仅允许人工 `workflow_dispatch`。

## 保持禁用

- 不运行或复用 vendor 内 `rust-release.yml`。
- 不把未签名候选误称为正式签名发行物。
- standalone、Homebrew、Desktop、R2、WinGet、SDK 和网站发布保持禁用。
- npm 已发布版本不可覆盖、删除后复用；异常恢复只允许修正 dist-tag、deprecate 错误版本或撤下 GitHub Release。
- 不使用 PATH 上的全局 Whale 或其他 worktree 的构建产物替代本候选。
