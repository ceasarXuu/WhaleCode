# U18：最新 main rebase 与 R8 TaskSpace 语义迁移

- 日期：2026-08-21
- 分支：`whalecode-codex`
- 工作空间：`/home/zhangxu/whalecode-codex`
- rebase 目标：`main@df5da4d3944448a9ae877d601f8c8045c415d983`
- Codex vendor substrate：`rust-v0.147.0` / `be6e8eac029b183056b7e4402879f15d2c85f61b`
- 真实模型/API 请求：0

## 1. 目标与边界

本单元把既有 0.147 融合成果重放到更新后的项目 `main`，解决的不只是 Git 文本冲突，还包括 TaskSpace R8 与旧 v2 实现之间的迁移号、状态模型、RPC 和最终请求组合冲突。

本轮没有改变已确认的产品决策：DeepSeek Flash 仍为默认，正式版 Pro 保持可用；Standard 仍是默认运行模式；OpenAI hosted catalog、remote plugin sharing、Bedrock、Guardian、Windows/TUI 完整矩阵和 live cache baseline 继续延期。旧 `ext/taskspace` v2 代码保留为未激活的历史层，本轮不以删除旧模块扩大范围。

## 2. 关键迁移结果

### Git 与代码边界

- 119 个本地提交已成功重放到目标 main，merge-base 与 `origin/main` 对齐。
- 0.147 vendor provenance 不变；本次变化是 Whale overlay 在新项目 main 上的语义重放，不伪装成新的 Codex tag。
- R8 TaskSpace 继续使用 core/app-server/state 的现有组合点，没有创建第二套状态库、运行时框架或 provider wire 分支。

### 状态迁移

- `0047_taskspace_canonical_store.sql` 保持历史字节和 checksum 不变，已安装旧 Whale 数据库不会因 checksum 漂移而失效。
- 新增 `0048_taskspace_relational_store.sql`：先移除冲突索引名，再将旧 v2 JSON 表重命名为 `taskspace_v2_*` archive，最后创建 R8 relational tables。
- 旧 v2 原始行无损保留，但不会被启发式转换或自动激活。这样避免把无法证明等价的旧状态升级为当前 canonical map。
- fresh、已知旧 Whale、部分或未知 migration 历史均有明确测试；未知形态继续 fail-closed。

### 生产组合路径

`thread_fork_inherits_taskspace_through_production_extensions` 现在通过真实 app-server composition、typed RPC、SQLite 和 mock Responses server 验证：

1. Standard turn 发出不带 TaskSpace 的最终 Responses body；
2. typed mode/policy RPC 启用 TaskSpace，模型通过 `taskspace_exec` 初始化并执行嵌套工具；
3. typed read RPC 读取 canonical map；
4. 普通 `thread/fork` 继承绑定；
5. app-server graceful shutdown 后重新启动并 resume fork；
6. 重启后的 typed read 仍返回同一 map，后续最终 Responses body 包含 TaskSpace tool/world-state。

该链路共消费 5 个本地 mock HTTP 请求，真实 API 费用为 0。

### 0.147 schema 兼容

0.147 image/web 扩展的 JSON Schema 使用小数 `minimum`。旧 overlay 将该字段收窄为 `i64`，会在工具注册时触发 `invalid number`。现已改为 `serde_json::Number`，整数 builder 保持兼容，TaskSpace validator 同时支持整数与小数比较，并增加小数 round-trip 测试。

另将 OpenAI image-generation runtime gate 测试夹具显式绑定 OpenAI provider，避免 ChatGPT 测试凭据错误复用 Whale 的 DeepSeek 默认 provider；这只修正测试前提，不改变产品逻辑。

## 3. 验证证据

| 验证 | 结果 |
| --- | --- |
| `codex-state --lib` | 190 passed |
| `codex-tools --lib` | 106 passed |
| `codex-core taskspace --lib` | 72 passed |
| image generation extension | 10 passed |
| web search extension | 8 passed |
| production TaskSpace fork/restart/final-wire | 1 passed |
| image/web app-server schema 注册定向用例 | 各 1 passed |
| OpenAI provider runtime-gate 测试夹具 | 1 passed |
| 隔离矩阵 | 4,881 tests 完成；非全绿 |
| cache index gate | blocked：敏感面与旧 accepted baseline 不一致，等待专用真实回归 |
| 真实模型/API 请求 | 0 |

隔离矩阵中的新增批量 image/web `invalid number` 回归已修复。高并发矩阵中一条 image-edit 用例未命中 mock，隔离精确复跑 1/1 通过。其余失败继续落在既有清单的 OpenAI hosted/remote catalog/plugin sharing、Bedrock、Guardian、DeepSeek-only 模型目录和 sandbox 临时目录假设；本单元不为追求全绿恢复已明确关闭的产品面。历史逐项分类仍见 [`failure-manifest.md`](../../v0.0.5/codex-upstream-sync/evidence/u17-closure-4f4f5d4c5/failure-manifest.md)。

## 4. 收口结论

最新 main 的 rebase 与冲突处理已经完成到行为层：DeepSeek 默认链未被 OpenAI 测试假设反向侵入；TaskSpace 使用唯一 relational store，旧 v2 数据可恢复查看但不被误激活；fork/restart 后的生产 final-wire 有端到端证据。同步元数据已经通过校验，但缓存 index gate 因当前敏感面与旧 accepted baseline 不一致而正确阻断。必须取得专用真实回归预算、使用专用 runner 形成与当前指纹一致的证据并完成 baseline 晋升后，才能提交和精确 lease 推送。
