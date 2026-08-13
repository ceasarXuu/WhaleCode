# LS-09 同响应双写协议五轮复验结果

- Date: 2026-08-14
- Subject: `806b297807d20e8b762886b010171093f0e8dc1d`
- Matrix: `provider-web-search-probe × map-request × repeat=5`
- Model: `deepseek-v4-flash`
- Result: **同响应合同部分生效，但 5/5 仍出现协议或序列错误，不能关闭 I03**

## 1. 本轮修复

模型可见合同不再把 `taskspace_exec` 笼统描述为所有 Tool 的唯一顶层入口，而是明确区分：

1. Map 和 client Tool 只通过顶层 Function Tool `taskspace_exec` 提交；
2. Provider-hosted Tool 继续使用 Provider 原生顶层接口；
3. 同一 assistant response 必须同时生成原生 Hosted Tool item 和且仅一个 Exec 归属声明；
4. 原生 item 执行工作，Exec 只登记节点归属；禁止提前登记、漏登或下一响应补登；
5. mismatch 反馈直接区分“本响应已执行但未登记”和“本响应登记但未执行”。

实现提交为 `806b29780`。`cargo test -p codex-core taskspace_exec --lib --locked` 为 74/74 PASS；构建和本机安装完成。

## 2. 五轮结果

| 轮次 | Record | Requests | Input | Cached | Uncached | Output | Req 2+ cache | 耗时 | 费用 | 公开验证 | Agent 结束 |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|
| 1 | `C8EE908A` | 12 | 419,629 | 370,816 | 48,813 | 17,569 | 90.57% | 165.116 s | USD 0.0127914248 | PASS | interrupted |
| 2 | `059E5635` | 12 | 310,861 | 280,192 | 30,669 | 7,366 | 89.74% | 86.301 s | USD 0.0071406776 | PASS | interrupted |
| 3 | `B4175C28` | 12 | 495,566 | 457,984 | 37,582 | 10,507 | 92.23% | 125.064 s | USD 0.0094857952 | FAIL | interrupted |
| 4 | `A77EFD54` | 12 | 319,855 | 292,096 | 27,759 | 7,173 | 90.99% | 87.134 s | USD 0.0067125688 | FAIL | complete |
| 5 | `FAC26AFA` | 12 | 401,841 | 371,456 | 30,385 | 9,124 | 92.21% | 111.511 s | USD 0.0078486968 | FAIL | interrupted |
| **总计** | 5 runs | **60** | **1,947,752** | **1,772,544** | **175,208** | **51,739** | **91.28% weighted** | **575.126 s** | **USD 0.0439791632** | **2/5** | **1/5 complete** |
| **均值** |  | **12** | **389,550.4** | **354,508.8** | **35,041.6** | **10,347.8** | **91.15%** | **115.025 s** | **USD 0.0087958326** |  |  |
| **中位数** |  | **12** | **401,841** | **370,816** | **30,669** | **9,124** | **90.99%** | **111.511 s** | **USD 0.0078486968** |  |  |

五轮全量 Token 加权缓存命中率为 91.00%。所有 usage 均可对账；总费用远低于获批预算。runner 以业务失败作为固定停点，
导致原 repeat=5 批次在第一轮停止；剩余四轮在原批准总范围内使用四个独立 repeat=1 授权完成，没有增加 sample 或重试。

## 3. 协议行为统计

> 2026-08-14 纠偏：`search`、`open_page`、`find_in_page` 是同一个 Hosted Web Search 调用中的内部 action，
> 不是三个 TaskSpace Tool，也不分别产生三份归属。此前把 subtype 映射列为主根因的结论无效。

五轮 trace 中 Runtime 始终把同一 Provider response scope 内的 Web Search 内部 output 归于原生 ToolSpec
`web_search`，没有自建名称，也没有把内部步骤拆成 Map action。Agent 共生成 16 次 Hosted 归属声明，其中 8 次完成同响应对账；
另外出现 6 次漏登拒绝、3 次提前登记拒绝和 2 次 owner 节点状态拒绝。每轮至少出现一种协议或序列错误。

有效正向证据仅能说明 Agent 偶尔可以在同一个响应中同时生成 Hosted Web Search 和 Exec 归属；它不能证明内部 action
需要分别映射。稳定缺口发生在两个独立顶层 response item 之间：当前 schema 不能结构性表达二者必须同时存在，只能依赖
Agent 遵循文字合同，再由 Runtime 事后核对。

## 4. 剩余根因

### 4.1 两个顶层 response item 缺少结构性耦合

Hosted Web Search 的原生结果与 `taskspace_exec` 归属是两个独立顶层 response item。JSON Schema 可以分别约束二者的
内部结构，但不能声明“出现 A 时必须在同一响应出现 B”。当前实现只能用文字合同要求 Agent 双写，并在响应完成后由 Runtime
忠实核对；漏写、提前写或错写仍然由模型自由生成。这是本轮证据支持的机制缺口，与 Web Search 内部 action 无关。

### 4.2 `Hosted work -> complete owner` 缺少合法同批顺序

部分响应同时包含最后一次 Hosted 结果归属和 owner 节点完成。当前 `update_and_work` 先应用 Map update，再检查 Tool owner；
如果 Agent 先把 owner 标成 Completed，随后同批 Hosted 归属会因 owner 已不可执行而被拒绝。现有闭集只表达
`Map update -> work`，没有表达“先登记本响应已完成的 Hosted work，再完成 owner”。这不是 Tool outcome 自动完成节点，
而是 Agent 在同一响应显式提交两个有顺序的动作；需要单独复核合法序列设计，不能靠 Runtime 猜测或重排。

### 4.3 测试 runner 的 repeat 停点与授权语义不一致

cache runner 的 `stop_reason()` 当前对任意业务失败都停止，即使 proposal 的 stop conditions 没有该条件。因此原 repeat=5
只执行一轮。该缺口属于 I07 测量工程，不改变本轮 Agent/Product 结论；四个后续独立记录保留了完整账本边界。

## 5. 判定

- **已验证收益**：同响应双写语义可被模型偶尔执行；8 次真实逻辑对账成功；Hosted `input` 误用不再是主导失败。
- **未达成**：协议没有稳定闭合，5/5 均有协议或序列错误，公开业务验证仅 2/5 通过。
- **缓存**：加权全量 91.00%、request 2+ 91.28%，没有出现缓存结构崩溃；但业务失败结果不得晋升 accepted baseline。
- **边界**：不引入 Runtime 自动绑定、默认 Root、跨响应 pending 或 Provider 结果重解释。

下一步应先单变量验证是否能在不引入 Runtime 自动绑定的前提下增强两个顶层 item 的配对合同，再单独解决 Hosted 归属后
完成 owner 的合法序列；不得再把内部 action 映射作为修复方向。

## 6. 证据

- Results: `benchmarks/cache-regression/results/WAR-20260814-001055-CACHE-REGRESSION-C8EE908A.json` 及随后四个 record。
- Evidence roots: `benchmarks/cache-regression/evidence/WAR-20260814-001055-CACHE-REGRESSION-C8EE908A/`、
  `WAR-20260814-001541-CACHE-REGRESSION-059E5635/`、`WAR-20260814-001715-CACHE-REGRESSION-B4175C28/`、
  `WAR-20260814-001927-CACHE-REGRESSION-A77EFD54/`、`WAR-20260814-002101-CACHE-REGRESSION-FAC26AFA/`。
- 无 API 的相对 `run-root` 预检失败保存在 `target/r8-ls09/same-response/preflight-relative-run-root-failure/`，不计入五轮。
