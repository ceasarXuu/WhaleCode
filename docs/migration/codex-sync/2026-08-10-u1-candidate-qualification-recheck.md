# U1：Codex 0.146 候选资格增量复核

- 执行日期：2026-08-10
- 候选：`rust-v0.146.0` / `e363b08c9175ac1cbe5893615dd2cb9ddf95043b`
- 结论：U1 已完成；0.146 候选继续 no-go，U2 不启动
- 生产 vendor：未修改
- 模型请求：0

## 复核结论

初次资格审查的四个失败已逐项归因：

| 初始失败 | 归因 | 修正或处置 |
| --- | --- | --- |
| CLI `--locked` 在解析前失败 | runner 约束过严；发布 lock 中本地 crate 版本仍为 `0.0.0` | 临时树改用 `--offline`，允许本地版本规范化且禁止联网 |
| core 代理继承断言失败 | runner 继承宿主大小写代理变量 | qualification 环境删除 HTTP/HTTPS/ALL/NO proxy 的全部大小写变体 |
| app-server 找不到 `codex-code-mode-host` | package-scoped nextest 未构建运行时 sibling binary | 在 app-server 测试前增加独立、可审计的 helper build 命令 |
| TUI 更新提示为 `0.0.0` / `0.146.0` | 上游发布 tag 的不可变 snapshot 与 release version 不一致 | 不修改候选、不接受 snapshot，保留为 no-go 证据 |

上述前三项均通过不修改候选源码的 focused probe 得到验证；TUI focused probe 在清理环境后仍复现完全相同的差异。

## 修正后的资格矩阵

| ID | 命令 | 结果 |
| --- | --- | --- |
| 01 | `cargo fmt --all -- --check` | passed |
| 02 | `cargo check -p codex-cli --bin codex --offline` | passed |
| 03 | `cargo nextest run --no-fail-fast -p codex-core` | failed |
| 04 | `cargo build --offline -p codex-code-mode-host --bin codex-code-mode-host` | passed |
| 05 | `cargo nextest run --no-fail-fast -p codex-app-server` | failed |
| 06 | `cargo nextest run --no-fail-fast -p codex-tui` | failed |

最终为 3/6 passed、3/6 failed。core 和 app-server 已不再出现原始代理继承、缺 helper 两个签名，但完整包测试进一步暴露当前主机无法提供的嵌套 sandbox/network 能力及其他 fixture 失败；TUI 除已确认的 release-version snapshot 外还有其他 snapshot/IPC 失败。因此 V1 的“官方入口可重复完成”门槛未满足，不能把 focused 修复误报成候选通过。

机器结果见 [`upstream-candidate.json`](../../v0.0.5/codex-upstream-sync/upstream-candidate.json)，完整规范化日志位于 `docs/v0.0.5/codex-upstream-sync/evidence/rust-v0.146.0/`，调试证据链位于 `coe/2026-08-10-03-16-u1-candidate-qualification.md`。

## 范围与后续

- qualification runner 只增加环境隔离、官方 local nextest 参数及显式 helper build，没有加入业务逻辑。
- `third_party/codex-cli/` index tree 保持不变；未修改 DeepSeek、TaskSpace 或缓存敏感面。
- 0.146 候选保持 `direction-rejected`；按唯一计划的停止条件，U2 及后续 cutover 单元保持未执行。
- 若要继续主线融合，需要先由用户决定改选新的上游候选，或明确授权针对 0.146 的上游 fixture/sandbox 资格策略；这不属于 U1。
