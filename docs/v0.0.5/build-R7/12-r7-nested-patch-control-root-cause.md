# R7 嵌套 Patch Control 错误根因

## 1. 判定

```text
Status: Root Cause Confirmed / Production Repair Not Implemented
Scope: shared taskspace_control patch carrier
Runtime state integrity: preserved
Feedback integrity: preserved
```

失败的不是 patch 语法，也不是 `apply_patch` 执行。Agent 生成的整个 `taskspace_control.arguments`
在进入 Runtime 前已经是非法 JSON，典型表现是在大型 patch 字符串结束后多一个 `}`。Runtime 的严格
parser 正确拒绝该调用并保持 `state_commit=false/partial_commit=0`。

根因是 provider 对“多分支 TaskSpace control schema + 复合 continuation + 大型多行 patch 字符串”
的参数结构闭合和正文保真不稳定。它不是 R7 新增 completion action 的局部问题：历史 54 次真实
`patch_then_actions` carrier 中有 15 次 JSON 非法，其中 13 次明确发生在旧
`transition_node(bind)`，本次 `complete_then_continue` 只是再次暴露共用缺陷。

## 2. 失败链

complex 样本的真实顺序是：

1. request 6 生成 2,128 字节 `complete_then_continue` arguments，末尾多一个 `}`；
2. Runtime 在第 2,128 列报告 trailing characters，零状态提交；
3. request 7 直接执行 `apply_patch` 并成功；
4. request 8 执行 pytest，request 9 读取文件；
5. request 10 才生成空 `{}` control，再次被零提交拒绝；
6. request 11 和 12 分别以合法 handoff 和 terminal action 收口。

因此空 `{}` 不是失败后的即时重试，也不是 Agent 没收到错误。首次 Runtime 失败输出与 request 7
对应 tool message 的 SHA-256 完全一致，并持续存在到 request 11。反馈层没有丢失、裁剪、改写或
call id 错配；空调用属于 Agent 在完整上下文下的后续参数生成错误。

## 3. 对照实验

probe 使用生产 `taskspace_control` schema、DeepSeek V4 Flash、`stream=false`，不经过 Whale SSE
assembler，也不执行 patch。每个 arm 运行 6 次，只落盘长度、哈希和 verdict。

| 形态 | JSON 合法 | trailing | patch 逐字节一致 | 结论 |
|---|---:|---:|---:|---|
| 当前大型嵌套 carrier | 4/6 | 2/6 | 0/6 | 真实缺陷可复现 |
| 少一层 `arguments` | 5/6 | 1/6 | 0/6 | 不能根治 |
| 当前短 patch carrier | 6/6 | 0/6 | 0/6 | 结构稳定不代表正文保真 |
| 独立大型 `apply_patch` | 6/6 | 0/6 | 6/6 | 唯一同时过两道门禁 |
| continuation 直接 `patch_input` | 6/6 | 0/6 | 0/6 | 一次正文仅 3 字节 |
| control 顶层 `patch_input` | 6/6 | 0/6 | 0/6 | 只改善可解析性 |

大型 patch 本身不是充分原因，因为独立 `apply_patch` 6/6 完整。对象嵌套层数也不是充分原因，
因为减少一层仍失败，而更扁平的两个 control 形态虽然可解析，正文仍被稳定改写或截断。问题边界是
provider 在复杂 control tool 中同时选择生命周期 variant、组织状态字段并生成长 patch 正文时的整体
参数保真能力。

## 4. 不应采用的修复

- Runtime 删除尾随括号或猜测 JSON：会把协议失败改写成 Runtime 主观语义，违反零提交底线。
- 只把 patch 字段挪到浅层：实验只证明 JSON 更容易闭合，没有证明正文忠实。
- 修改 projection 或增加错误提示：失败反馈已经完整进入上下文，不是反馈层问题。
- 把问题归因于 SSE：非流式 provider 响应已经独立复现。
- 直接启用 provider strict beta：当前 schema 并不满足其全部对象字段 required 等约束，既有项目 probe
  也出现复杂 schema 返回空参数；这不是可直接切换的局部开关。

## 5. 修复方向

优先候选是让 patch 继续使用 provider 已证明稳定的独立 `apply_patch` tool 参数，不再把正文嵌入
`taskspace_control`。TaskSpace control 只携带小型、机械的状态交接参数；两者在同一 provider response
中按“状态交接 barrier -> direct patch -> ordinary actions”执行，以保留一次 request 完成 handoff 和
后续工作的目标。

这不是允许独立完成节点。共享 sequence preflight 必须在执行前机械确认：

- completion handoff 后存在声明的 sibling continuation；
- response 中最多一个 patch，且顺序与节点 binding 一致；
- control 参数或 patch 参数任一不可解析时，不猜测、不改写；
- 状态提交后的 patch 失败由原始工具反馈忠实返回，下一节点保持 Running，由 Agent 决定恢复动作。

该方向尚未进入生产。下一门禁应先做 sibling 双 tool-call provider probe，再做 Runtime fixture，最后才跑
simple/complex Docker 样本。若 provider 不能稳定在同一 response 生成两个调用，安全退路是把状态交接
和 direct patch 分成两个 request，并把额外 request 明确记为产品成本，而不是恢复嵌套 carrier。

## 6. 参考

- [DeepSeek Tool Calls Guide](https://api-docs.deepseek.com/guides/tool_calls)：非 strict 模式不保证 arguments 始终是合法 JSON；strict mode 使用 beta 接口并对 schema 有额外约束。
- [DeepSeek Create Chat Completion](https://api-docs.deepseek.com/api/create-chat-completion/)：确认当前 Chat Completions tool call、stream 和 strict 参数合同。
- [JSON Schema Combining](https://json-schema.org/understanding-json-schema/reference/combining)：说明 `anyOf` 是实例校验组合，不提供模型生成过程的结构保真保证。

机器结果见 `benchmarks/taskspace/r7/nested-patch-control-probe-result.json`，完整假设和证据链见
`coe/2026-07-19-19-30-r7-nested-patch-control-arguments.md`。

## 7. 复现与验证

以下命令都从仓库根目录执行，避免脚本路径与 Cargo workspace 路径相互污染：

```bash
pwsh -NonInteractive -NoProfile \
  -File scripts/taskspace-benchmark/test-r7-nested-patch-control-probe.ps1
cargo run --quiet \
  --manifest-path third_party/codex-cli/codex-rs/Cargo.toml \
  -p codex-tools --example r7_nested_patch_control_schema
pwsh -NonInteractive -NoProfile \
  -File scripts/taskspace-benchmark/probe-r7-nested-patch-control.ps1 -Repeat 6
```

live probe 从 `.env.local` 按需读取 `DEEPSEEK_API_KEY`，但结果文件只包含 hash、字节数、token usage
和 verdict。不得为了调试临时落盘完整 provider arguments；需要比较正文时使用预期/实际 SHA-256。
