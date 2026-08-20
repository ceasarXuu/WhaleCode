# R8 TaskSpace Exec 多行 Patch 自愈结果

- Date: 2026-08-20
- Scope: `taskspace_exec` Function 参数中的确定性 JSON 语法修复
- API usage: `WAR-20260820-000226-R8-MULTILINE-SELF-HEAL-R3`，3 次真实 sample
- Status: implemented；真实自然命中并验证正式历史替换

## 1. 直接证据

`WAR-20260819-223533-R8-BASE-CLIENT-SCOPE-R5` 的五轮运行中，两次长 `apply_patch` 被 JSON parser 拒绝。原始
Function arguments 表明这不是单个裸换行：整个 Patch 正文以原始多行字符串进入 `input`，正文中的双引号同样没有 JSON
转义。既有自愈器只覆盖单个闭合符号和单个裸换行，因此无法修复这两条真实参数。

## 2. 实现边界

本轮增加两条按顺序执行的机械修复：

1. 对 JSON 字符串内部一个或多个原始 LF 统一转义；只有修复后能够完整解码为当前 `taskspace_exec` plan 才接受。
2. 若普通 LF 转义仍失败，只对具备原生 `apply_patch` Tool 身份、`input` 字段和完整 `*** Begin Patch` / `*** End Patch`
   边界的原始 Patch 正文执行一次标准 JSON string 编码。编码后的整个 outer plan 仍必须通过同一生产 decoder。

修复后的 `ResponseItem::FunctionCall.arguments` 在写入会话历史前替换原始参数。自愈不补 Map 状态、不修改 Tool 顺序、
不推断 Agent 意图，也不执行无法通过完整 decoder 的候选。多行字符串同时伴随另一类语法错误时继续零副作用拒绝。

## 3. 验证

- `tools::taskspace_exec::self_heal::tests`：13/13 通过；覆盖单/多裸换行、真实形态多行 Patch、混合错误拒绝和既有闭合符号修复。
- `session::tests::taskspace_raw_newline_self_heal_replaces_the_item_before_history_is_recorded`：1/1 通过；确认正式上下文只保存修复后参数。
- `cargo fmt --check`：通过；仅有仓库既有 stable toolchain 配置警告。

## 4. 真实运行验收

`subscription-billing-repair × map-request × repeat=3` 三轮均完成业务修复、公开验证、隐藏 oracle 和 Map 闭合：

| Run | Requests | Input | Cached | Uncached | Output | Agent wall | Map | 自愈 / syntax reject | 其他拒绝 |
|---|---:|---:|---:|---:|---:|---:|---|---|---:|
| 1 | 9 | 166,565 | 151,552 | 15,013 | 5,686 | 49.058s | 5 nodes / 4 edges / closed | 1 / 0 | 0 |
| 2 | 10 | 185,776 | 166,272 | 19,504 | 6,275 | 55.543s | 5 nodes / 4 edges / closed | 0 / 0 | 1 Waiting |
| 3 | 14 | 262,441 | 242,304 | 20,137 | 7,434 | 60.834s | 6 nodes / 5 edges / closed | 0 / 0 | 1 Waiting + 1 TransitionInvalid |
| **总计** | **33** | **614,782** | **560,128** | **54,654** | **19,395** | **165.435s** | **3/3 closed** | **1 / 0** | **3** |

Run 1 自然生成含原始 LF 的多行 `apply_patch` 非法参数。Runtime 记录原始 hash
`e8f91d...10b1`，修复后 hash `cba9bc...cf9c`；正式 rollout 中同一 `call_id` 参数的逐字 hash 也是
`cba9bc...cf9c`，可解码为 `update_and_work -> apply_patch@fix`，并在同一请求成功执行。错误参数没有进入后续上下文，
也没有产生 syntax reject 或恢复请求。因此“真实输入可修复、同请求执行、修复版进入正式历史”三项验收均通过。

本次自然命中的是通用“全部原始 LF 转义”分支。历史坏例中同时存在正文裸引号时使用的“完整 Patch 正文 JSON string 编码”
后备分支仍只有确定性测试与历史 fixture 证据，本批不能把它写成在线自然命中。

三次状态拒绝与 JSON 自愈无关，均零副作用并在后续请求恢复，继续归 I04。费用按完整 usage 估算为 CNY 0.10464656。
最终请求使累计 input 达到 614,782，超过计划 600,000 上限 2.46%；这是当前请求边界 gate 无法预测本次最终 usage 的
测量边界，未触发额外请求。完整可复算证据见
`benchmarks/taskspace/r8/evidence/WAR-20260820-000226-R8-MULTILINE-SELF-HEAL-R3.json`。

该结果完成多行原始 LF 参数形态的在线工程验收；复合裸引号后备分支仍为离线验收，也不关闭 I03 的其他 Agent 动作组织问题。
