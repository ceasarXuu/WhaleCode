# 原生 Hosted Tool 身份真实验收结果

- Date: 2026-08-14
- Subject: `2b92f23459a24ef0347f72f3f6c029bdbd363360`
- Model: `deepseek-v4-flash`
- Scope: `single-file-fast-fix × {standard, map-request} × repeat=1`，以及
  `provider-web-search-probe × map-request × repeat=1`
- Result: **原生身份修复在线成立；Hosted 同响应归属仍未稳定，不能据此关闭 I03**

## 1. 本轮验证的问题

本轮只验证 TaskSpace 是否停止自建 Hosted Tool 公共名称，并机械复用当前请求中原生 `ToolSpec::name()`。
对于 Web Search，唯一公共 Tool 身份是 `web_search`；Provider 返回中的 `search`、`open_page`、`find_in_page`
只是 `web_search_call.action.type`，不是额外 Tool、额外 Map action 或额外归属记录。

实现没有修改原生 ToolSpec、Standard Tool 集合、Provider response item、Map 状态机或同响应对账规则。
离线 `cargo test -p codex-core taskspace_exec --lib --locked` 为 74/74 PASS。

## 2. 真实运行结果

| Sample / Arm | Requests | Input | Cached | Uncached | Output | Req 2+ cache | Runner 耗时 | 费用 | 业务结果 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `single-file-fast-fix / standard` | 6 | 74,492 | 61,952 | 12,540 | 1,518 | 97.64% | 39.410 s | 已计入批次 | PASS |
| `single-file-fast-fix / map-request` | 6 | 88,763 | 73,856 | 14,907 | 2,110 | 91.95% | 39.737 s | 已计入批次 | PASS |
| **两臂合计** | **12** | **163,255** | **135,808** | **27,447** | **3,628** | - | **87.346 s** | **USD 0.0052386824** | **2/2 PASS** |
| `provider-web-search-probe / map-request` | 12 | 362,286 | 320,640 | 41,646 | 11,136 | 90.37% | 121.583 s | USD 0.0098463120 | 业务 PASS；协议未通过 |
| **全部获批运行** | **24** | **525,541** | **456,448** | **69,093** | **14,764** | - | **208.929 s** | **USD 0.0150849944** | 见分层判定 |

全部运行均有完整 Provider usage、缓存和费用账本；没有执行重试。两次无 API 预检失败分别来自旧 binary attestation
和相对 `run-root` 的 runner 路径错误，不计入真实 Agent run，也不形成产品结论。

## 3. 结构化 trace 结论

### 3.1 原生身份修复在线成立

1. Provider 请求的顶层 Tool 声明稳定为 `taskspace_exec + web_search`，没有 TaskSpace 自建的 `search`、
   `open_page` 或 `find_in_page` ToolSpec。
2. Provider 原始返回共出现四个 `web_search_call`：两个 `action.type=search`、一个 `open_page`、一个
   `find_in_page`。四者公共 Tool 类型始终是 `web_search_call`。
3. Runtime 的 mismatch 反馈只报告原生 ToolSpec 名 `web_search`，没有把内部 action subtype 暴露为 Tool 身份。
4. 普通样本的 Standard 和 map-request 均完成业务；map-request 的 Exec、Map、client 结果和节点绑定无失败，说明
   身份来源改造没有破坏普通 Tool 路径。

### 3.2 Hosted 双写仍未通过

专项样本实际调用了原生 Web Search，但 Agent 没有在相同 assistant response 中生成对应
`taskspace_exec` 的 `execution: "already_executed"` 归属项：

- 首次搜索发生在 Map 初始化之前，只有原生 `web_search_call`，没有 Exec 归属；
- 后续 `open_page + find_in_page + search` 同属一次逻辑 Web Search 使用，仍没有 Exec 归属；
- 下一次 Exec 尝试写文件时，Runtime 看到当前 response scope 中 `actual=[web_search]`、声明为空，因此零副作用拒绝；
- Agent 随后不再使用 Hosted Tool，改走 client Tool 和已有搜索事实，最终完成文件、校验和 Map。

因此性能观察中的 `provider_results=0` 不是“没有真实搜索”，也不是 Web Search 内部动作统计错误，而是没有任何一组
原生 Hosted action 与 Exec 归属成功通过同响应对账。业务 oracle 只证明最终文件正确，不能替代协议验收。

## 4. 判定

- **通过**：TaskSpace Hosted 身份逐字复用原生 `ToolSpec::name()`；Web Search 内部 action 未被另造为公共 Tool。
- **通过**：普通 Standard / map-request 工作流与缓存没有结构性回归。
- **未通过**：两个独立顶层 response item 的同响应双写仍不稳定，本轮两个逻辑 Web Search 使用均漏登。
- **未通过**：I03 不关闭；当前结果继续支持“文字合同和分别独立的 schema 不能结构性保证共现”。
- **观测修正**：性能观察器把本轮报成 `provider action/result = 0` 是对“成功登记数”的事实，但若解释为
  “Provider 没有执行”则会失真。I07 需要区分原生执行、成功归属和 mismatch 拒绝三项，不新增产品语义。
- **缓存发布**：候选 surface `19b2395a7c61547d38d91813fc8238897b119333bcba34c9102b0b9239240af2`
  已完成获批真实 smoke，但未自动替换 accepted baseline；晋升仍按缓存门禁流程单独决策。

## 5. 证据

- Smoke result: `benchmarks/cache-regression/results/WAR-20260814-025239-CACHE-REGRESSION-EB43D143.json`
- Hosted result: `benchmarks/cache-regression/results/WAR-20260814-025657-CACHE-REGRESSION-3ABDE358.json`
- Evidence roots: `benchmarks/cache-regression/evidence/WAR-20260814-025239-CACHE-REGRESSION-EB43D143/`、
  `benchmarks/cache-regression/evidence/WAR-20260814-025657-CACHE-REGRESSION-3ABDE358/`
- Full local trace: `target/r8-native-hosted-tool-live/`
