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
- [x] 2026-08-21 只读 registry 快照仅有 `0.0.0-dev*`、`0.0.1-dev*`，尚无 `0.0.5*`（发布前仍须重新查询）
- [x] cache-sensitive index gate 通过（fingerprint `9c817b9f59426efa097be43988d4731a2e7ba412bad63bdf69a466fcbcaaaced`）
- [x] main worktree 引用门禁通过，未引用其他本地 worktree

## 发布者必须人工核验

### 版本与内容

- [ ] 确认产品 tag 为 `v0.0.5`，不是 `rust-v0.149.0`
- [ ] 确认 GitHub/npm 发布标题与说明使用 WhaleCode `v0.0.5`
- [ ] 通读 `RELEASE_NOTES.md`，确认功能描述、已知边界和“不宣称 upstream 全量测试全绿”准确
- [ ] 确认最终提交就是获准发布的 main HEAD，并记录 commit SHA

### npm 账号与版本

- [ ] `npm whoami` 返回有权发布 `@ceasarxuu/whalecode` 的账号
- [ ] `npm view @ceasarxuu/whalecode versions --json` 确认 `0.0.5` 及六个平台后缀版本尚未存在
- [ ] 确认 granular token/OTP 可用、发布时 2FA 策略明确，凭据未写入仓库或用户级共享配置
- [ ] 确认根包使用 `latest`，平台包使用 `linux-x64`、`linux-arm64`、`darwin-x64`、`darwin-arm64`、`win32-x64`、`win32-arm64` dist-tag
- [ ] 确认 scoped 包发布命令全部带 `--access public`

### 原生制品

- [ ] 选择并批准一个 WhaleCode 仓库的原生构建 run URL；不得使用 OpenAI/Codex vendor release run
- [ ] 六个平台制品齐全：Linux x64/arm64、macOS x64/arm64、Windows x64/arm64
- [ ] 每个平台归档内的可执行文件名为 `whale` 或 `whale.exe`，且 `--version` 返回 `whale 0.0.5`
- [ ] 记录最终七个 npm tarball 的 SHA-256，并核对 staging 没有混入本机路径、凭据、日志或额外文件
- [ ] 至少在 Linux、macOS、Windows 各完成一次全新 prefix 安装 smoke；同时确认已有官方 `codex` 命令未被覆盖

### 发布与回滚授权

- [ ] 明确批准本次外部副作用：创建/推送 `v0.0.5` tag、发布七个 npm 版本，以及是否创建 GitHub Release
- [ ] 确认发布顺序为六个平台包先发布，根元包最后发布
- [ ] 确认失败停止条件：任一平台缺失、版本冲突、完整性不符或安装 smoke 失败时，不发布根元包
- [ ] 确认回滚负责人和操作：修正 npm dist-tag、deprecate 错误版本、撤下 GitHub Release；npm 已发布版本不可覆盖或删除后复用

## 尚待外部输入

- 原生六平台 Whale 构建 run 尚未选定和批准。
- npm 发布账号、token/OTP 与 2FA 状态只能由发布者在发布环境确认。
- `release.json` 保持 `status=preparing`、`publish_authorized=false`，直到上述人工项签核且用户明确授权实际发布。

## 当前禁止

- 不得运行或复用 vendor 内 `rust-release.yml`。
- standalone、Homebrew、Desktop、R2、WinGet、SDK 和网站发布保持禁用。
- 未明确授权前不得创建/推送 tag，不得发布 GitHub/npm/WinGet/R2 资产。
- 不得使用 PATH 上的全局 Whale 或其他 worktree 的构建产物替代本 worktree 候选。
