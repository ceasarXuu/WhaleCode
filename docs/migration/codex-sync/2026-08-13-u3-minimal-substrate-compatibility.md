# U3：Codex 0.147 最小 Whale substrate 兼容边界验证

- 日期：2026-08-13
- 候选：`rust-v0.147.0` / `be6e8eac029b183056b7e4402879f15d2c85f61b`
- 候选 tree：`3828c818254d4c756585f5b59fe46b6fa3634765`
- 结论：`verified`
- 生产 vendor：未修改
- 真实模型请求：0

## 1. 验证目标

在一次性导出的官方 0.147 tree 上验证：

1. Whale 主二进制身份可以通过很薄的 overlay 建立；
2. `WHALE_HOME`、默认 `~/.whale`、文件 auth 和 keyring auth 不与官方 Codex 状态共址；
3. 不需要 DeepSeek 或 TaskSpace stub 即可构建最小 Whale CLI；
4. 0.147 新增能力不会未经既有产品约束静默改变默认权限、远程插件或 MCP 行为。

本单元只验证兼容 seam，不替换 `third_party/codex-cli/`，也不完成最终发行文案、helper 命名或安装包重放。

## 2. 一次性 overlay

临时树中修改 6 个生产文件和 2 个专用测试文件：

| 路径 | 最小变化 |
| --- | --- |
| `codex-rs/cli/Cargo.toml` | 主二进制和 `default-run` 改为 `whale` |
| `codex-rs/cli/src/main.rs` | 顶层 CLI identity、bin name 和 usage 改为 Whale |
| `codex-rs/utils/home-dir/src/lib.rs` | 读取 `WHALE_HOME`，默认 `~/.whale`；拒绝 `.codex` 和与 `CODEX_HOME` 相同的目录 |
| `codex-rs/login/src/auth/storage.rs` | direct auth keyring service 改为 `Whale Auth` |
| `codex-rs/login/src/auth/storage_tests.rs` | 锁定 direct auth keyring service 的 Whale namespace |
| `codex-rs/secrets/src/lib.rs` | encrypted secrets keyring service 改为 `whale` |
| `codex-rs/features/src/lib.rs` | `remote_plugin`、`plugin_sharing` 默认值改为 false |
| `codex-rs/features/src/tests.rs` | 锁定上述两个默认值 |

没有复制旧版 CLI、provider 或 TaskSpace 源文件，也没有新增兼容框架。内部 `Codex*` 类型名保持上游原样；只有产品 identity、状态目录和 keyring namespace 发生变化。

## 3. 验证结果

| 验证 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | passed；仅 stable rustfmt 的已知 unstable-option warning |
| `cargo test -p codex-utils-home-dir` | 6 passed |
| `cargo test -p codex-login auth::storage::tests` | 24 passed |
| `cargo test -p codex-secrets` | 8 passed |
| Whale auth service 定向测试 | login 1 passed；secrets 1 passed |
| remote plugin 默认值定向测试 | 1 passed |
| 官方 sandbox V8 环境下 `cargo build --offline -p codex-cli --bin whale` | passed |
| `whale --version` | `whale 0.147.0` |
| `whale --help` | 标题和 usage 使用 `Whale CLI` / `whale` |
| 隔离环境 `whale login status` | 返回 `Not logged in`；只在 `WHALE_HOME` 创建临时运行目录，指定的 `CODEX_HOME` 保持空 |

构建沿用 U2 已验证的 OpenAI `rusty-v8-v150.4.0`、`ptrcomp_sandbox_release`、`x86_64-unknown-linux-gnu` 资产合同，没有回退到本机源码构建。

## 4. 新能力默认面审计

| 能力 | 0.147 事实 | Whale 处理 | 结论 |
| --- | --- | --- | --- |
| `--approve-for-me` | 显式 CLI flag，`default_value_t=false`；使用时设置 auto-review、`on-request` 和 `workspace-write` | 不修改；未传 flag 时不生效 | explicit opt-in |
| MCP 2026-07-28 | `mcp_2026_07_28` 为 under-development，默认 false | 保持上游默认 false | default-safe |
| portable / remote Agent Plugins | `plugins` 默认 true；0.147 同时把 `remote_plugin`、`plugin_sharing` 设为 stable + 默认 true | 保留当前 Whale 已有的本地 `plugins=true`；把新增 remote/sharing 默认值设回 false | protected by existing feature seam |
| thread sections | app-server 提供显式 `threadSection/*` RPC；只有支持 SQLite 的 store 才可执行，未调用时不会创建 section 或移动 thread | 不增加 Whale 专用禁用层；作为显式客户端请求能力保留 | explicit API action |

审计中唯一需要修正的默认变化是 remote plugin 与 plugin sharing。该修正直接使用上游现有 feature seam，只改两行默认值，符合既定“本轮不启用新增产品能力”的约束，不构成新的产品决定。

## 5. 边界与后续

- U3 证明的是最小 identity/home/auth/default seam 可行，不代表所有 0.147 用户文案已经完成 Whale 化。
- U4 机械替换 vendor 时只允许重放本报告声明的最小生产 patch；不得顺带带入 DeepSeek 或 TaskSpace 实现。
- npm/bun/brew 更新命令、helper binary、安装包和剩余用户可见品牌属于既有 Whale release overlay，必须在后续对应的 brand/release 闭环中按当前产品合同重放，不能指向 `@openai/codex` 或官方 Codex 安装路径。
- 0.147 的 `plugins=true` 与当前 Whale 一致；`remote_plugin=false`、`plugin_sharing=false` 是 cutover 必须锁定的默认值。
- thread sections 是新增的显式 app-server 协议能力；若未来要在 Whale UI 或 TaskSpace 中主动暴露，需在对应产品单元另行决定。本次不做 UI 接入。

临时源码树和 smoke home 在取证后删除；生产 vendor index tree 全程未变化。
