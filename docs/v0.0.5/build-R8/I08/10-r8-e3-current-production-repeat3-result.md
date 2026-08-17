# R8-E3 当前生产版本 Repeat 3 结果

- Status: complete / closure experiment failed
- Date: 2026-08-17
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arms: Standard + `map-request`
- Repeat: 3 pairs / 6 sample runs
- Subject commit: `394399c3b6123e562b1e5a504d60381fb0bc0a12`
- Binary SHA-256: `2dbb3f75347a3265cce4c0567ab2c45dbbbc7069e2f90a66e90d3450306e40b7`
- Ledger: `WAR-20260817-215521-R8-E3-CLOSURE-R3`
- Evidence: `target/whale-agent-runs/WAR-20260817-215521-R8-E3-CLOSURE-R3/single-file-fast-fix/20260817-215618-477`

`R8-E3` 是本次闭环实验标签；benchmark manifest 的正式 `evidence_target` 为 `E2`。本轮可以提供当前生产 trace 和行为证据，
不能替代 I01/I07 等关闭合同中明确要求的正式 E3。

## 1. 结论

本实验没有通过完整关闭验收。Standard 3/3 完成；TaskSpace 只有 2/3 完成。失败轮首次
`initialize_and_work` 正常执行，第二个 Provider 响应却把内层 `exec_command` 生成成与
`taskspace_exec` 同级的顶层 Function Call。Runtime 硬门在执行前正确阻止了该调用，但 response reconciliation
将它升级为 Fatal 并终止会话，Agent 没有纠正机会。

另一轮在最终 `update_and_finish` 生成了复合结构错误：缺失闭合符号的同时把 `type` 和 `update_map` 放入
`finish_map`。单纯补一个闭合符号仍不满足当前 schema，因此唯一机械自愈正确地没有改写；Agent 下一请求自行纠正。
裸换行自愈本轮没有自然触发，继续只有确定性测试证据。

两次到达节点交接的 TaskSpace 运行均未复现 Waiting frontier 错误，Base `3.0.5` 获得正向但不足以关闭 I04 的证据。
两次成功运行的成本结构与 IC-09 基线基本一致：请求数 `1.17x`、总 input `1.40x`、平均每请求 input `1.20x`。

## 2. 逐轮结果

| Pair | Standard | TaskSpace | Standard req/input | TaskSpace req/input | TaskSpace 关键行为 |
|---:|---|---|---:|---:|---|
| 1 | solved | interrupted | 6 / 77,584 | 2 / 26,401 | 初始化和首次 shell 成功；第二响应顶层 `exec_command` 被零副作用硬拒绝，Fatal 终止 |
| 2 | solved | solved | 6 / 77,656 | 7 / 108,568 | 5 节点/4 边闭合；无 syntax、protocol 或 state reject |
| 3 | solved | solved | 6 / 74,455 | 7 / 104,116 | 一次复合 JSON/schema 错误；零副作用拒绝后下一请求纠正并闭合 Map |

所有 Standard 运行和两次完成的 TaskSpace 运行产生同一文件补丁，公开验证与隐藏 oracle 均通过。

## 3. 失败轮请求路径

1. Request 1：Agent 使用 `taskspace_exec.initialize_and_work` 创建
   `root -> inspect -> fix -> verify -> finish`，并在 `inspect` 上执行 `exec_command`；执行成功，反馈完整返回
   `inspect=in_flight`、下游 Waiting 状态和原生 shell output。
2. Request 2：Agent 明确说明将读取 README、测试和实现，但生成顶层 `exec_command`，参数中还携带
   `node_id=inspect`。当前 Provider 顶层合同只允许 `taskspace_exec` 和原生 Hosted Tool；`exec_command` 只存在于
   Exec 内层 Catalog。
3. `TaskSpaceExecResponseScope` 识别到 forbidden top-level client Tool，真实 shell 没有执行，Map 没有提交；
   `taskspace_response_reconciliation_error` 将结果转换为 Fatal，turn 结束。

当前证据排除“首次没有初始化 Map”“首次 Tool 结果丢失”和“协议没有声明唯一顶层 Function Tool”。直接失败由 Agent
违反已暴露合同与 Runtime 缺少可纠正反馈路径共同构成。内层 Tool catalog 是否放大模型原生顶层 Tool 偏好仍是候选诱因，
本次单例不能坐实。

## 4. 成本

### 4.1 全部实际消耗

| 指标 | Standard | TaskSpace | 合计 |
|---|---:|---:|---:|
| Sample runs | 3 | 3 | 6 |
| Requests | 18 | 16 | 34 |
| Input | 229,695 | 239,085 | 468,780 |
| Cached input | 224,384 | 210,304 | 434,688 |
| Uncached input | 5,311 | 28,781 | 34,092 |
| Output | 5,465 | 4,849 | 10,314 |
| Agent wall | 51.33s | 44.64s | 95.97s |
| Request 2+ cache | 97.42% | 91.57% | - |

按冻结价格估算费用为 CNY `0.06341376`，未超过 CNY `0.15`、48 requests、700K input、20K output 或 900 秒上限。

### 4.2 只比较两个业务均成功的 Pair

| 指标 | Standard | TaskSpace | TS / Standard |
|---|---:|---:|---:|
| Requests | 12 | 14 | 1.17x |
| Input | 152,111 | 212,684 | 1.40x |
| 平均每请求 input | 12,676 | 15,192 | 1.20x |
| Cached input | 148,736 | 196,480 | 1.32x |
| Uncached input | 3,375 | 16,204 | 4.80x |
| Output | 3,521 | 4,382 | 1.24x |
| Agent wall | 33.01s | 39.70s | 1.20x |
| Request 2+ cache | 97.46% | 91.46% | -6.00pp |
| 相邻请求 input 增量均值 | 584.8 | 660.5 | 1.13x |

该成功子集与 IC-09 的 `1.39x` 总 input、`1.13x` 请求数和约 `92.86%` TaskSpace warm cache 同方向。失败轮必须
单独报告，不能用更低的总请求和 input 把失败伪装成成本改善。

## 5. 全局问题重评

| Issue | 本轮证据 | 状态影响 |
|---|---|---|
| I01 | 34 个完成请求均无 logical retry 或 duplicate；只覆盖 `map-request`，没有完成三 projection 验收 | 保持 verifying，增加正向证据 |
| I02 | Exec call/output 均按 `call_id` 单次出现，没有 output body 重复；失败轮有一个被拒绝的 orphan top-level call | 保持 verifying，final-wire 正向证据 |
| I05 | 复合 JSON 错误获得一次准确、零副作用反馈并纠正；顶层 client escape 却直接 Fatal，没有 Agent 可纠正反馈 | 保持 verifying，新增明确恢复缺口 |
| I06 | forbidden 顶层 client Tool 在线自然触发，真实 Tool 和 Map 均未产生副作用；成功运行 Patch 均为每请求一个 | 硬门在线成立，保持静态关闭候选 |
| I10 | TaskSpace trace 中 capability identity 68 次均为 `05b41a...a3bf`，无冲突 | 保持 verifying，生产 identity 正向证据 |
| I07 | 34 个 request/attempt/boundary/completion/usage 身份完整，无 retry/duplicate；两次 Patch declaration/result 均 1/1 | 仍不能关闭，见第 6 节的新观测缺口 |
| I03 | 1/3 顶层 client escape 导致任务失败；1/3 复合 JSON/schema 错误后纠正 | 明确未通过，保持 verifying |
| I04 | 两次到达交接的运行均无 Waiting frontier 或非法 `waiting -> in_flight`；失败轮未到达交接 | 正向但样本不足，保持 verifying |
| I08 | 成功 Pair 成本与 IC-09 基线一致；失败轮证明异常行为仍会破坏成本和完成率解释 | 保持 investigating |

## 6. 新暴露的观测与运行治理缺口

这些表现并入 I07，不新增全局问题编号：

1. performance observer 将失败 Map 标记为 `root_task_active_after_nodes_closed`，但 canonical 节点仍包含
   `root/inspect=in_flight` 和三个 Waiting 节点；该 warning 的“nodes closed”判断不忠实。
2. Pair report 的 `token_usage_record_count` 与 canonical `request-facts.json` 的 usage rows 不一致；本报告直接以
   request facts 逐请求复算，没有使用该展示字段。
3. 账本声明“业务/oracle 失败即停止”，runner 在 Pair 1 TaskSpace 失败后仍自动执行 Pair 2/3。没有超预算或自动重试，
   但停止合同没有被机械执行，后续真实矩阵前必须修正或显式改写停止策略。

## 7. 当前停点

本轮不继续修改协议或消费真实预算。下一步应先处理两个明确问题：

1. forbidden 顶层 client Tool 必须保持零副作用硬拒绝，但不能直接 Fatal；需要给 Agent 一次准确、可继续的合同错误反馈。
2. 修正 I07 的 Map 完成判断、usage 展示和 runner 停止合同，再决定是否需要新的真实复验。

不得因本轮失败增加 Runtime 语义决策，也不得自动把顶层 client Tool 改写进 Exec。
