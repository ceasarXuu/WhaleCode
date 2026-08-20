# IC-04 Standard / TaskSpace 静态线材对照

- Status: complete-with-scope
- Date: 2026-08-17
- Subject commit: `0e334bc26`
- Evidence: `target/r8-i08/ic04-20260817-1`
- Paid Whale Agent runs: 0

## 1. 结论

通过真实 `codex-core` 请求构造路径捕获的首请求中，TaskSpace 为 `64,016 B`，Standard 为 `56,395 B`，
TaskSpace 净增 `7,621 B`（`13.51%`）。这个固定结构差值不足以单独解释此前真实运行中接近翻倍的平均每请求 input，
但证明 TaskSpace Tool 合同仍是一个明确的固定增量。

两个现有生产夹具的用户文本和 system 附加内容并不完全相同，因此本结果只建立首请求结构边界；system、history 的差值不能升级为
TaskSpace 产品因果结论。等历史与真实 token 对照由 IC-05、IC-06 承担。

## 2. 顶层结构

| Section | Standard bytes | TaskSpace bytes | Delta |
|---|---:|---:|---:|
| system messages | 16,021 | 12,409 | -3,612 |
| natural history | 343 | 621 | +278 |
| base instructions | 21,045 | 20,034 | -1,011 |
| tools | 18,624 | 30,589 | +11,965 |
| tool choice | 20 | 20 | 0 |
| other payload | 342 | 343 | +1 |
| **Total** | **56,395** | **64,016** | **+7,621** |

`active_projection` 与 `ordinary_tool_feedback` 均为 0。各 section 之和逐字节等于 Provider payload，没有未归属面积。

## 3. Tool 差值

Standard 暴露 12 个原生 client Tool 合同与 1 个 Provider-hosted Tool；TaskSpace 暴露 `taskspace_exec` 与同一个
Provider-hosted Tool，原生 client Tool 合同进入 Exec catalog。

| Tool 组成 | Standard bytes | TaskSpace bytes | Delta |
|---|---:|---:|---:|
| 原生 client Tool / Exec client catalog | 18,553 | 19,925 | +1,372 |
| TaskSpace protocol | 0 | 4,636 | +4,636 |
| TaskSpace Map schema | 0 | 1,311 | +1,311 |
| TaskSpace sequence schema | 0 | 4,566 | +4,566 |
| TaskSpace metadata | 0 | 92 | +92 |
| Provider-hosted Tool | 49 | 48 | -1 |
| Tool envelope | 22 | 11 | -11 |
| **Total** | **18,624** | **30,589** | **+11,965** |

Tool 增量中，`10,605 B` 来自 TaskSpace 协议、Map 与合法序列合同；`1,372 B` 来自 client catalog 相对原生
Tool wire 的结构开销。这里是面积归因，不代表这些内容可删除；正确性合同的必要性必须另行验证。

## 4. 证据边界

1. 两个测试均走 `test_codex -> Responses request builder -> provider wire`，不是手写 JSON。
2. 请求在 Insta 断言前已完整捕获；测试随后因仓库已有快照落后于当前 Skills/Tool description 而失败，不影响捕获值。
3. 生成的 `.snap.new` 已删除，未接受或改写无关快照。
4. 当前结果不提供 Provider token，只提供精确 JSON bytes；不得用 `bytes/4` 冒充真实 token。
5. 原计划中的等历史多轮合成没有继续扩建。真实历史面积可由现有完整 trace 复算，继续造 transcript 会增加迎合测试结论的风险。

## 5. 对后续的约束

- H2 从“未知”收敛为“存在约 12 KB/request 的 Tool wire 固定增量”，但尚未证明它是 input 主因。
- H1、H3、H4、H6 仍需逐请求真实证据。
- IC-06 必须同时报告 request amplification、per-request input amplification 和 Tool/history section area；只比较总 input 不足以归因。
