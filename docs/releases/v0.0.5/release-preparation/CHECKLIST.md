# WhaleCode v0.0.5 发布准备清单

## 机器已验证

- [x] 发布身份门禁：WhaleCode `v0.0.5`，Codex substrate `rust-v0.149.0`
- [x] 分发身份门禁：活动安装、更新和 npm 路由均归属 Whale
- [x] 品牌身份门禁：用户可见产品面统一为 Whale
- [x] release guard 单元测试通过
- [x] `cargo metadata --locked --no-deps` 通过
- [x] `cargo build -p codex-cli --bin whale --locked` 通过
- [x] workspace 隔离二进制输出 `whale 0.0.5`
- [x] npm 元包离线 staging 与 `npm pack --dry-run` 通过：`@ceasarxuu/whalecode@0.0.5`
- [x] npm 元包只包含 `README.md`、`bin/whale.js`、`package.json`
- [x] npm 元包引用六个平台版本，未包含 OpenAI/Codex 分发目标
- [x] 发布前 registry 快照仅有 `0.0.0-dev*`、`0.0.1-dev*`，不存在 `0.0.5*`
- [x] cache-sensitive index gate 通过（fingerprint `9c817b9f59426efa097be43988d4731a2e7ba412bad63bdf69a466fcbcaaaced`）
- [x] main worktree 引用门禁通过，未引用其他本地 worktree
- [x] Whale 根仓库已建立手动触发、只读权限、六平台并行的未签名原生候选 workflow
- [x] workflow 静态门禁确认不含 tag trigger、npm publish、GitHub Release、写权限或 vendor release workflow 调用
- [x] 每个平台输出 npm staging 兼容归档、SHA-256 和 Whale 制品合同，并汇总六平台 manifest
- [x] run `32495900738` 的六个平台构建、打包和上传全部成功；本地隔离聚合合同校验通过
- [x] 同一 run 已离线生成七个 npm tarball，并记录 SHA-256；Linux x64 实机返回 `whale 0.0.5`

## 发布者必须人工核验

### 版本与内容

- [x] 确认产品 tag 为 `v0.0.5`，不是 `rust-v0.149.0`
- [x] 确认 GitHub/npm 发布标题与说明使用 WhaleCode `v0.0.5`
- [x] 通读 `RELEASE_NOTES.md`，确认主要内容为 TaskSpace、DeepSeek 支持和 Codex 0.149 特性同步，且未作过度声明
- [x] 最终发布候选固定为 main 提交 `77d6bf09360b55ebb929653bf73def7f91ef46e6`

### npm 账号与版本

- [x] 发布者已在 Chrome 的 `@ceasarxuu/whalecode` access 页面确认当前账号具备 owner/maintainer 发布权限，包为 public scoped package
- [x] `npm whoami` 返回有权发布 `@ceasarxuu/whalecode` 的账号 `ceasarxuu`
- [x] `npm view @ceasarxuu/whalecode versions --json` 确认 `0.0.5` 及六个平台后缀版本尚未存在
- [x] npm CLI 发布通过 Chrome 安全密钥验证；凭据未写入仓库
- [x] 根包使用 `latest`，平台包使用 `linux-x64`、`linux-arm64`、`darwin-x64`、`darwin-arm64`、`win32-x64`、`win32-arm64` dist-tag
- [x] scoped 包发布命令全部带 `--access public`

### 原生制品

- [x] 选择 WhaleCode 原生构建 run `32495900738`；未使用 OpenAI/Codex vendor release run
- [x] 已获用户授权并手动触发 `whale-native-artifacts`：最终候选来源为 main `77d6bf093`，输入版本为 `0.0.5`
- [x] 六个平台制品齐全：Linux x64/arm64、macOS x64/arm64、Windows x64/arm64
- [x] 六平台草稿 npm 包均在对应原生 runner 的隔离前缀执行成功，`--version` 返回 `whale 0.0.5`
- [x] 记录最终七个 npm tarball 的 SHA-256，并核对 staging 没有混入本机路径、凭据、日志或额外文件
- [x] Linux、macOS、Windows 双架构均完成隔离 prefix 安装 smoke，且未生成或覆盖 `codex` 命令（run `32521520611`）

### 发布与回滚授权

- [x] 用户已明确授权创建/推送 `v0.0.5` tag、发布七个 npm 版本并公开 GitHub Release
- [x] GitHub Release 已公开，目标为候选提交 `77d6bf093`，包含七个 npm tarball、manifest 和 SHA256SUMS
- [x] 已按六个平台包先发布、根元包最后发布的顺序执行
- [x] 发布前已执行失败停止检查：平台齐全、版本无冲突、完整性一致且六平台安装 smoke 通过
- [x] 回滚由仓库发布维护者负责：修正 npm dist-tag、deprecate 错误版本、撤下 GitHub Release；npm 已发布版本不可覆盖或删除后复用

## 发布结果

- [x] npm 已发布 `0.0.5` 根包与六个平台版本，`latest` 和六个平台 dist-tag 均已通过 registry 回读核验。
- [x] 从 npm registry 隔离安装 `@ceasarxuu/whalecode@0.0.5`，实机返回 `whale 0.0.5` 且未生成 `codex` 命令。
- [x] 未签名注释 tag `v0.0.5` 已推送，peeled commit 为 `77d6bf09360b55ebb929653bf73def7f91ef46e6`。
- [x] GitHub Release 已于 `2026-08-21T20:28:29Z` 公开：<https://github.com/ceasarXuu/WhaleCode/releases/tag/v0.0.5>。
- [x] GitHub Actions 继续默认关闭；六平台 workflow 仅允许人工按需触发，本地验证为默认路径。

## 当前禁止

- 不得运行或复用 vendor 内 `rust-release.yml`。
- 不得把 `whale-native-artifacts` 的未签名候选误称为正式签名发行物。
- standalone、Homebrew、Desktop、R2、WinGet、SDK 和网站发布保持禁用。
- 后续版本仍须取得明确授权后才可创建/推送 tag 或发布 GitHub/npm 资产。
- 不得使用 PATH 上的全局 Whale 或其他 worktree 的构建产物替代本 worktree 候选。
