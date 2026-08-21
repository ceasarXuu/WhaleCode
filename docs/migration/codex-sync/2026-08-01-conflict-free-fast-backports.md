# 2026-08-01 Codex 无冲突快速 Backport

> 历史执行报告：本文中的工作单元标签只用于追溯当时的提交，不属于当前计划编号。当前唯一计划见 [Codex CLI 主线融合执行计划](../../v0.0.5/codex-upstream-sync/plan.md)。

## 范围

- 前一 Codex vendor 基线：`fed0a8f4faa58db3138488cca77628c1d54a2cd8`
- 新 Codex vendor 基线：不变；本次不是整仓刷新
- 上游稳定目标：`rust-v0.146.0`
- 导入方式：六个上游提交按原始 hunk 回移，分别提交、验证和推送
- 文档角色：已完成工作的历史执行证据

## 已 Backport

| 工作单元 | 上游提交 | Whale 提交 | 结果 |
| --- | --- | --- | --- |
| W1 | `2e598df6fcd30717cfdcd2a898746a84d365ca23` | `44e48e210` | `git -C` 不再自动批准 |
| W2 | `9deb4f9c86426c40ba1e189831d7bc3634dd7b94` | `4162a91c9` | Windows URL prefix 改为大小写不敏感 |
| W3 | `6ec8c4a6ecb17bc3ab10d0c5edf75494b50cab7e` | `a5670f9a6` | Git metadata 命令禁用 repository fsmonitor |
| W4 | `36912ce3de1c039f7faaddd509d0465ff644e6c1` | `00b9d9006` | Windows paste burst interval 与其他平台统一为 8ms |
| W5 | `5d7e6a2503fc71f09cea71bfca9e193e0c3fd215` | `595ff6d37` | 外部 borrowed slice 不再执行无效 pointer offset |
| W6 | `c86b1be3cdbe12307843bcc9e7a44c1904ddcdf1` | `2c81aca14` | diff render 借用或移动 `FileChange`，不再 clone |

每个提交 body 均记录完整 `Upstream-Commit` 和官方 patch SHA-256。目标文件实施前与 `fed0a8f4` 基线一致，六个官方 patch 的 `git apply --check --directory=third_party/codex-cli` 均通过。

## 保留的 Whale 差异

- 未修改 DeepSeek provider、payload、model、reasoning 或缓存前缀；
- 未修改 TaskSpace、ActionMap、protocol、state schema 或 replayable state；
- 未导入 permission profiles、MCP/Skills/Plugins/Code Mode 或 network proxy 架构；
- 未修改 `Cargo.toml`、`Cargo.lock`、公共 schema 或生成物；
- Codex vendor 基线标识保持 `fed0a8f4`，避免把 selective backport 误报为 vendor refresh。

## 验证结果

通过项：

- W1 focused tests 2/2；`codex-shell-command` 130/130；
- W2 Linux `codex-shell-command` 130/130；新增用例受 `#[cfg(windows)]` 控制，当前环境未执行；
- W3 fsmonitor marker 1/1、`get_has_changes` 5/5、`codex-git-utils` 36/36；
- W4 paste burst 8/8；
- W5 external slice 1/1、wrapping 45/45；
- W6 diff render 49/49、diff gallery snapshots 3/3；
- `cargo fmt --all -- --check`；
- `cargo check -p codex-cli --bin whale --locked`；
- 缓存 index gate：`PASS d3484073e6702d853c9e9ddd86c6a5f7499fdcb4a6590fe30133ff1303bdff2e`，当前指纹未变。

全量 `codex-tui` 使用 `RUST_MIN_STACK=33554432` 运行：1843 passed、33 failed、1 ignored。失败集中于当前 Whale 快照、ActionMap route mode 断言和既有 TUI 状态展示，与本批三个 TUI 目标的 focused tests 不重叠；因此本批代码已回移，但全量测试门禁仍为非绿色。

## 调试与操作经验

1. 默认 Rust 测试线程栈会让 `attach_live_thread_for_selection_rejects_empty_non_ephemeral_fallback_threads` 稳定栈溢出；同一用例设置 `RUST_MIN_STACK=33554432` 后通过。TUI 全量回归应显式设置该值，避免把测试基础设施限制误判为产品回归。
2. Insta 失败默认生成 `.snap.new`。首次运行生成的 32 个临时文件已移入系统回收站，未接受或提交；复跑使用 `INSTA_UPDATE=no`，避免污染工作树。
3. 缓存门禁脚本按当前工作目录解析仓库根。必须从仓库根运行 `python3 scripts/cache-regression/check_cache_regression_gate.py --source index`；从 `codex-rs` 通过相对路径启动会错误查找 `codex-rs/benchmarks/`。
4. 上游 patch SHA-256 是官方原始 patch 的追溯标识。部分提交的上游 parent 晚于 Whale vendor 基线，因此本地 `git diff` 的 blob index 和 hunk 行号可以不同；实施内容保持官方 hunk，不能把本地 diff 摘要误写成官方 patch 摘要。

## 未关闭风险

- W2 的 Windows mixed-case URL 测试尚未在 Windows 执行；
- W4 尚未完成 VS Code integrated terminal 和原生 Windows terminal 的输入/粘贴 smoke；
- `codex-tui` 全量仍有 33 个既有失败，进入下一 TUI 大批次前应单独治理基线；
- 缓存 index gate 同时提示“最近一次 live 回归失败”；本批没有真实 Whale Agent run，也不据此宣称 live cache 回归恢复。

## 回滚

六个提交互相独立。若平台验证发现回归，使用 `git revert <Whale 提交>` 单项回滚；不使用 `git reset --hard`，也不覆盖整个 vendor 目录。
