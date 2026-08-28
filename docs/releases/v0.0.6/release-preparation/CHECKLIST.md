# WhaleCode v0.0.6 发布准备清单

## 已有工程证据

- [x] Rust workspace 与 lockfile 中 Whale 自有 package version 已统一为 `0.0.6`。
- [x] 多 Provider Phase 0–5 已完成，14 项验收行为均有自动化测试或可复现证据。
- [x] provider 受影响六 crate 隔离矩阵 9286/9286、app-server protocol 293/293 通过。
- [x] DeepSeek 三模型静态目录、Responses SSE 与 Vision 输入具备离线合同；Vision 单请求真实 smoke 通过。
- [x] provider final-wire cache 基线已接受，当前 accepted manifest 有效。
- [x] GitHub Actions 发布工作流仅支持 `workflow_dispatch`，无自动发布触发器和写权限。
- [x] 2026-08-28 复核 DeepSeek 官方模型表：Flash、Pro、Vision Exp 均列为支持 Responses API。

## 本轮候选准备

- [x] 登记产品版本 `v0.0.6` 与 substrate `rust-v0.149.0` 的独立身份。
- [x] 发布门禁、六平台候选和 release smoke 的默认版本更新为 `0.0.6`。
- [x] 发布说明草稿覆盖多 Provider、DeepSeek Responses、凭据隔离与恢复语义。
- [x] 本地 release preflight 全部通过：identity、distribution、brand、npm candidate、native workflow、manual Actions 与 24 项 release 单测。
- [x] `cargo metadata --locked --no-deps` 与本地 `whale` 构建通过，二进制输出 `whale 0.0.6`。
- [x] cache-sensitive index gate 通过（fingerprint `3b83125448b51052fe8972cf20c5a5bab818f683a91214dd2eb99ab99eff8c8f`）。
- [ ] 固定最终候选 commit，并回填 `release.json`。

## 发布前必须完成

- [ ] 决定 DeepSeek hosted web search 的最终用户可见合同，并统一 provider 与模型目录能力表达。
- [ ] 如需宣称“三模型 live matrix 完全通过”，先取得真实运行预算并补齐 Flash/Pro usage 证据；否则保持发布说明的有限声明。
- [ ] 手动触发 `whale-native-artifacts`，确认六平台构建、打包与 manifest 全部通过。
- [ ] 从候选制品完成 Linux、macOS、Windows 双架构隔离安装 smoke，且不生成 `codex` 命令。
- [ ] 核验 npm 账号权限，并确认 `0.0.6` 根包与六个平台版本尚未存在。
- [ ] 记录七个 npm tarball 的 SHA-256，检查无本机路径、凭据、日志或额外文件。
- [ ] 通读并批准最终 `RELEASE_NOTES.md`。

## 外部发布授权

- [ ] 用户明确授权创建并推送 `v0.0.6` tag。
- [ ] 用户明确授权发布七个 npm 包与公开 GitHub Release。
- [ ] 发布后回读 npm dist-tag、registry 安装结果、tag peeled commit 与 GitHub Release 资产。

当前 `release.json` 必须保持 `status=preparing`、`publish_authorized=false`。准备工作不授权 tag、npm publish、GitHub Release 或远端 Actions 运行。
