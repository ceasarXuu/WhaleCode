# IC-06 当前提交真实双臂结果

- Status: complete-with-anomaly
- Date: 2026-08-17
- Ledger: `WAR-20260817-045635-R8-I08-ICB`
- Subject commit: `0405d49f9cb3b6aca391dca2b6dbfeaa013833af`
- Binary SHA-256: `05c4c634bc23e0c4fad34366477125af533f3a45e2fb1eeb30a77ecc59dd8508`
- Evidence: `target/r8-i08/ic-b/single-file-fast-fix/20260817-045806-605`
- Sample: `single-file-fast-fix`
- Model: `deepseek-v4-flash`

## 1. 结果

Standard 与 map-request 均完成业务、公开验证和隐藏 oracle，产生相同补丁。TaskSpace Map 为
`root -> inspect -> fix -> finish`，4 nodes / 3 edges，最终完整闭合。

TaskSpace 出现一次 Waiting frontier 零副作用拒绝，因此这是一份有效的“当前实际产品成本”证据，但不是无协议异常的 clean baseline。
按照预先停止条件，本批次不自动补跑；总比例可用于解释本次观测成本，不能单独升级为正常路径稳定频率。

| Metric | Standard | map-request | Ratio / delta |
|---|---:|---:|---:|
| Result | passed | passed | same |
| Requests | 6 | 8 | 1.33x |
| Runtime Tool calls | 9 | 6 | 0.67x |
| Input | 77,447 | 123,675 | 1.60x / +46,228 |
| Cached input | 75,392 | 114,688 | 1.52x |
| Uncached input | 2,055 | 8,987 | 4.37x |
| Output | 1,817 | 2,138 | 1.18x |
| Request 2+ cache | 96.96% | 91.98% | -4.98 pp |
| Agent wall | 18.29s | 20.72s | 1.13x |

14 个请求合计 201,122 input、190,080 cached、11,042 uncached、3,955 output；按冻结价格估算 CNY 0.0227536。
runner shell exit 1 只表示 repeat=1 未达到聚合证据门槛；`run-status.exit_code=0`、`run_validity=valid`，不是 sample 失败。

## 2. 请求路径

| Request | Standard | map-request |
|---:|---|---|
| 1 | 两个 shell 调用发现文件 | `initialize_and_work` 建 Map，并在 `inspect` 下执行首次 shell |
| 2 | 四个 shell 调用读取 README、配置、实现和测试 | `work`，合并读取 README、配置、实现和测试 |
| 3 | 跑失败测试 | 错用 `work` 在 Waiting `fix` 上跑测试，被零副作用拒绝 |
| 4 | 应用单个 patch | `update_and_work` 完成 `inspect`、解锁 `fix` 并跑失败测试 |
| 5 | 跑通过测试 | 在 `fix` 下应用单个 patch |
| 6 | 最终总结 | 在 `fix` 下跑通过测试 |
| 7 | - | `update_and_finish` 完成 `fix` 并显式关闭 Map |
| 8 | - | 最终总结 |

额外两个请求来源明确：

1. request 3 是可避免的 Agent 序列误选，实际消耗 14,608 input；此前反馈已明确返回 `inspect=in_flight`、
   `fix=waiting`、不完整 parent `inspect` 和“下游不可执行”，Tool schema 与 Base 也明确要求改用 `update_and_work`。
   因此这次不是上下文丢失或 Runtime 语义扭曲，而是 Agent 没有遵循已收到的硬合同。
2. request 7 是当前产品合同要求的显式 Map finish；Runtime 返回 finish 结果后，Agent 才在 request 8 形成最终总结。它是当前设计开销，
   不是故障。

两个请求实际承载 31,609 input，但不能把该值直接当作可删除收益：去掉前一请求会改变后续历史长度和语义阶段。

## 3. 请求数与单请求体量

```text
request amplification          = 8 / 6 = 1.333x
per-request input amplification = 15,459.4 / 12,907.8 = 1.198x
total input amplification       = 123,675 / 77,447 = 1.597x
```

按 Standard 每请求 input 先计算请求数增量：总差值 46,228 中，约 25,816（55.84%）对应多出的两次请求，约
20,412（44.16%）对应 TaskSpace 八个请求各自更大。该算术分解与 IC-05 历史对照的 55.5% / 44.5% 基本一致。

## 4. 每请求结构差值

| Section | Standard bytes/request | map-request bytes/request | Delta |
|---|---:|---:|---:|
| system messages | 5,049.0 | 5,049.0 | 0.0 |
| natural history | 7,286.7 | 10,268.9 | +2,982.2 |
| base instructions | 21,045.0 | 20,034.0 | -1,011.0 |
| tools | 18,623.0 | 26,688.0 | +8,065.0 |
| tool choice | 20.0 | 20.0 | 0.0 |
| other payload | 355.0 | 356.0 | +1.0 |
| **Wire total** | **52,378.7** | **62,415.9** | **+10,037.2 / +19.16%** |

Provider 实际平均每请求 input 增幅为 19.77%，与精确 wire bytes 增幅一致。Base 和 system 不是高 input 根因；TaskSpace Base
反而每请求少 1,011 bytes。

### 4.1 Tool declaration

| Tool 组成 | Standard bytes/request | map-request bytes/request |
|---|---:|---:|
| 原生 client Tool / Exec client catalog | 18,553 | 16,024 |
| TaskSpace protocol | 0 | 4,636 |
| TaskSpace Map schema | 0 | 1,311 |
| TaskSpace sequence schema | 0 | 4,566 |
| TaskSpace metadata | 0 | 92 |
| Provider-hosted Tool | 48 | 48 |
| Tool envelope | 22 | 11 |
| **Total** | **18,623** | **26,688** |

TaskSpace 专用 protocol/Map/sequence/metadata 合计 10,605 bytes/request；Exec catalog 比 Standard 原生 Tool wire 小
2,529 bytes/request，最终净增 8,065 bytes/request。该增量占每请求 wire 净差的 80.35%，是最大固定结构来源。

### 4.2 Natural history

| History 组成 | Standard area bytes | map-request area bytes | Delta |
|---|---:|---:|---:|
| user messages | 2,736 | 5,821 | +3,085 |
| assistant messages | 2,182 | 5,108 | +2,926 |
| reasoning items | 10,464 | 12,485 | +2,021 |
| direct client Tool calls | 5,277 | 0 | -5,277 |
| direct client Tool outputs | 23,061 | 0 | -23,061 |
| `taskspace_exec` calls | 0 | 14,023 | +14,023 |
| `taskspace_exec` outputs | 0 | 44,714 | +44,714 |
| **Total** | **43,720** | **82,151** | **+38,431** |

TaskSpace 没有同时保留 direct client Tool call/output，因此没有发现同一原生 Tool 结果双份进入 Provider history。
`taskspace_exec` carrier 是替代载体，但它还承载 node 绑定、Map revision、affected node states 和精确嵌套结果，所以比 Standard
直接载体更大。Exec carrier 与 Standard direct Tool carrier 的面积差为 30,399 bytes，占 history 净差的 79.10%。

## 5. 根因判断

| Hypothesis | 结论 |
|---|---|
| H1 请求次数放大 | 坐实；多 2 requests，其中 1 次 Agent 误选，1 次当前 finish 合同开销 |
| H2 Tool wire 固定增量 | 坐实；净增 8,065 B/request，主要是状态机和合法序列合同 |
| H3 outer history 双份复制 | 证伪；TaskSpace direct client call/output 为 0，Exec 是替代载体 |
| H4 Map projection/read 主增量 | 证伪；map-request 无 active projection、无 `read_map` |
| H5 跨层协议明显重复 | 未坐实；Base 是宏观/简洁生命周期，Tool 是完整硬合同，不能仅因主题相同判为重复 |
| H6 reject 放大 | 坐实；本轮 Waiting reject 直接增加一次请求 |
| H7 观察器或 usage 错误 | 证伪；14/14 request identity、usage 和 section bytes 闭合 |

当前观测成本的直接原因不是单点 bug，而是两个结构因素叠加：**TaskSpace 请求更多，并且每个请求的 Exec 合同与 carrier 更大。**
其中一次额外请求来自本轮异常；无异常正常路径的稳定比例仍需独立 clean pair 才能晋升。

## 6. 修复边界

当前没有可直接执行且符合全局约束的单变量修复：

- Waiting 误选所需状态和规则已经完整进入上下文；Runtime 自动完成 parent、改写序列或放行 Waiting work 都会越过状态机底线。
- 删除状态机/合法序列合同预计能省 input，但会削弱已证明必要的正确性约束，不能仅按 bytes 删除。
- 取消显式 finish 后的结果确认，或让同一响应同时携带可提交 final summary，属于产品协议变更，需要先单独设计与确认。
- 压缩 Exec result 只能删除无消费价值的机械字段；affected state、原生 Tool 结果和错误语义当前均有明确消费者，尚未发现安全冗余。

因此已批准的额外修复复验额度暂不消费。下一步应先选择一个产品方向或发现一个可证明无语义损失的字段级冗余，再执行单变量修复。
