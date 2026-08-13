# U9：恢复 DeepSeek 长上下文压缩合同

- 日期：2026-08-14
- 上游 substrate：Codex CLI `rust-v0.147.0`
- 结论：`verified`
- 真实模型请求：0

## 1. 实施边界

本单元恢复三个相互依赖的运行时合同：

1. DeepSeek V4 Flash 与 Pro 的上下文窗口为 1,000,000 tokens，自动压缩阈值为 755,000 tokens；
2. 普通 Flash 对话保持 Flash，生成 compaction checkpoint 时使用 Pro；
3. 继续使用 Codex 0.147 的通用 checkpoint prompt、history replacement 和持久化生命周期。

模型条目在本单元保持 `hide`，只为 core 提供正确运行时元数据；Flash 默认和 Flash/Pro 的用户可见性仍由 U6 收口。本单元不修改 TaskSpace projection、resume/fork 状态，也不增加 Debug/Create/Multi-Agent 专用摘要提示词。

## 2. 最小实现

- 在现有 local compaction 路径增加一个 DeepSeek-only 采样模型选择：源模型不是 Pro 时，使用 `deepseek-v4-pro` 构造采样 `TurnContext`；原始 turn context 继续承担 checkpoint 安装、事件和会话状态。
- 在 0.147 bundled catalog 恢复 Flash/Pro 隐藏元数据，包括 `standard` 默认 reasoning、原生 Responses 对应的 freeform `apply_patch`、不发送 `reasoning.summary`、1M 上下文和 755K 阈值。
- 将 context-window 阈值判断提取为纯函数，并锁定 0、755K 前一 token 和 755K 边界行为。
- 复用上游通用 checkpoint prompt。它已经覆盖进度、用户决定、约束、待办和关键证据，因此不恢复旧 Whale prompt appendix，避免把 TaskSpace、Debug 和 Create 产品语义侵入通用 compaction。

## 3. 验证结果

| 验证 | 结果 |
| --- | --- |
| bundled DeepSeek catalog | passed；Flash/Pro 均为 1M/755K、hidden、无 reasoning summary 参数 |
| sampling model unit | passed；Flash 选择 Pro，Pro 不重复切换 |
| threshold / short job | passed；0 和 754,999 不触发，755,000 触发 |
| mock final request | passed；首个普通请求为 Flash，compact 请求为 Pro 并携带 checkpoint prompt |
| mock 线程栈说明 | 默认测试线程栈在该集成夹具上溢出；`RUST_MIN_STACK=16777216` 后通过，未发生真实网络请求 |
| models-manager / core compaction 回归 | 48 passed / 17 passed |
| sync replay / metadata 门禁 | 42 tests passed；inventory/replay/metadata checks passed；当前 overlay 27 路径 |
| cache regression index gate | passed；指纹 `dfc5237fbd21b8abb0e4e35f13a7bf874b57b9f89c767da50538e547e9aca1a3`；当前指纹未变，最近一次 live 回归仍为失败 |
| 真实网络/API 请求 | 0 |

## 4. 结论

U9 已在 Codex 0.147 的现有 compaction 生命周期中恢复 DeepSeek 长上下文与 Flash→Pro 压缩请求，没有复制旧 compaction 状态机，也没有触及 TaskSpace。下一工作单元为 U10：恢复缓存与 final-wire 免费证据并执行强制 index gate。
