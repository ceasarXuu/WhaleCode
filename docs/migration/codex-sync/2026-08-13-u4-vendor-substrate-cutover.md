# U4：Codex 0.147 vendor substrate 切换

- 日期：2026-08-13
- 上游版本：`rust-v0.147.0`
- 上游 commit：`be6e8eac029b183056b7e4402879f15d2c85f61b`
- 上游 tree：`3828c818254d4c756585f5b59fe46b6fa3634765`
- 结论：`verified`
- 真实模型请求：0

## 1. 实施结果

`third_party/codex-cli/` 已由旧导入基线机械替换为通过 U2 资格验证的 0.147 Git archive，并只重放 U3 已验证的最小 Whale substrate overlay：

- `whale` 主二进制和顶层 CLI identity；
- `WHALE_HOME` / `~/.whale` 状态隔离；
- direct auth 与 encrypted secrets 的 Whale keyring namespace；
- `remote_plugin=false`、`plugin_sharing=false` 默认值保护。

本单元没有重放 DeepSeek、cache 或 TaskSpace，也没有恢复剩余品牌、helper、安装和发布逻辑。它们仍由 U5–U17 的闭环单元负责。

## 2. 来源与差异证明

| 检查 | 结果 |
| --- | --- |
| tag commit | `be6e8eac029b183056b7e4402879f15d2c85f61b` |
| commit tree | `3828c818254d4c756585f5b59fe46b6fa3634765` |
| `LICENSE` SHA-256 | `d17f227e4df5da1600391338865ce0f3055211760a36688f816941d58232d8dc`，与官方 archive 一致 |
| archive 与 U3 candidate 的源码差异 | `UPSTREAM.md` + 8 个 U3 修改路径 |
| 当前 overlay inventory | 11 路径：8 个 U3 修改 + 3 个被 Git `export-ignore` 排除的 `.vscode` 文件 |
| 历史 upstream delta | 4,666 路径，继续表示初始导入基线到 0.147 的变化 |

`.vscode/extensions.json`、`.vscode/launch.json`、`.vscode/settings.json` 不在官方 Git archive 中，因此当前 inventory 把它们记录为删除；这不是 Whale 产品逻辑或手工裁剪。

切换后，15 条历史 selective backport 已由 0.147 vendor 基座承接，账本状态统一更新为 `superseded_by_vendor`。元数据生成器据此拆分两个基线：历史 delta/ledger 继续使用初始导入 commit，当前 overlay/replay 使用 0.147 commit，避免把整棵上游树误判成 Whale overlay。

## 3. 验证结果

| 验证 | 结果 |
| --- | --- |
| `cargo fmt --all -- --check` | passed；仅 stable rustfmt 的已知 unstable-option warning |
| `cargo test -p codex-utils-home-dir` | 6 passed |
| `cargo test -p codex-login auth::storage::tests` | 25 passed |
| `cargo test -p codex-secrets` | 9 passed |
| feature default 定向测试 | 3 passed |
| `cargo build --offline -p codex-cli --bin whale` | passed；使用已校验的官方 rusty-v8 150.4.0 工件 |
| `whale --version` | `whale 0.147.0` |
| `whale --help` | passed |
| 隔离 smoke | `WHALE_HOME` 创建 `tmp/arg0`；指定的 `CODEX_HOME` 保持空 |
| sync metadata 单测 | 42 passed |
| delta / replay / metadata 门禁 | passed |
| cache contract/gate 单测 | 53 passed（由独立 U4a 提交完成合同迁移） |
| free final-wire 矩阵 | passed：prompt caching、cache key、MCP tool cache、`codex-api` |
| cache regression index gate | passed；指纹 `437c9afd92cdf50428c353d78b1268950e9e1852392bd0118a1d787a59978f8f` |

Cargo 构建会把官方 `Cargo.lock` 中 workspace 包的发布占位版本从 `0.0.0` 重写为 `0.147.0`；该构建副作用已恢复，提交中保留官方 archive 的锁文件内容。

## 4. 阶段结论

U4 的源码替换、常规验证和强制缓存门禁均已完成，Phase B 可以收口。首次门禁揭示的合同与产品面混批问题已通过独立 U4a 提交治理，没有降低门禁、修改产品源码或伪造 live 结果。

accepted live baseline 仍保留最近一次 `live_regression_failed`，本次免费验证不构成发布放行。真实缓存回归继续推迟到 DeepSeek/TaskSpace 被测闭环恢复后另行申请预算；不得直接晋升基线。
