# Phase B5 TaskSpace Exec 单变量压缩记录

- Status: SC-01 accepted as experiment baseline / SC-02 pending
- Budget package: `R8-EXEC-SCHEMA-USD100-20260811`
- Subject commit: `95579125c805be645a64b5f938a63bcba647f177`

## 1. 执行边界

本专题只压缩 `taskspace_exec` 的模型可见固定合同。每轮只改变一个因素，先通过离线合同、final-wire 和缓存敏感面门禁，再运行
`single-file-fast-fix × map-request × repeat=1`。业务、结构、usage 或成本证据不完整时不得叠加下一变化。

## 2. SC-01：移除完整 outer result TypeScript 展开

唯一变化是取消 Tool description 中完整 `TaskSpaceExecResult` TypeScript 合同，改为一条机械反馈保证。以下能力保持不变：

- 内部 typed result 与 capability identity；
- Agent 实际收到的完整 JSON、原生 client 结果及错误；
- 输入 schema、Map 操作、序列、node binding、Hosted 对账；
- Router、Runtime、preflight、自愈、协议示例和 Standard 路径。

离线验证：TaskSpace Exec `82/82`、production final-wire fixture、Standard final-wire 和缓存敏感面候选检查通过。生产 wire 的 Tool
段由上一有效 7-request 基线的 `30,522 bytes/request` 降至 `25,773 bytes/request`，减少 `4,749 bytes`（`15.56%`）。

| Run | 结果 | Requests | Input | Cached | Uncached | Output | Req 2+ hit | Agent wall | 估算费用 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| 旧合同 R03 | PASS | 7 | 114,793 | 107,904 | 6,889 | 2,096 | 93.33% | 19.3s | $0.001853 |
| SC-01 首轮 | PASS | 7 | 102,358 | 87,808 | 14,550 | 1,989 | 92.59% | 18.5s | $0.002840 |
| SC-01 暖缓存 | PASS | 8 | 120,306 | 113,792 | 6,514 | 2,728 | 94.04% | 23.2s | $0.001994 |

同为 7 requests 时，SC-01 input 减少 `12,435`（`10.83%`），首请求从 `13,850` 降至 `12,500`。首轮费用上升来自新 Tool
前缀第一次出现的冷缓存，不是 input 反弹；第二轮暖缓存已恢复到 94% 以上且费用低于旧 R03。

两轮都完成相同 5-node 线性 Map、一个最终 patch、公开测试和隐藏 oracle。没有 syntax、wrapper、顶层逃逸、结果缺失或 usage 缺口。
暖缓存轮的两次 preflight reject 均为 Agent 过早选择 waiting 子节点，Runtime 忠实返回未完成父节点后下一请求纠正；该既有行为与删除结果合同无
语义关联。

结论：`SC-01` 保留并作为下一因素的实验基线。两轮有效运行累计估算 `$0.0048342`，USD 1.00 包剩余约 `$0.9951658`。

## 3. 门禁边界

现有基线晋升器只允许自动替换 Standard semantic snapshot，`taskspace_production_tool_wire` 尚不在其受保护替换集合中，因此本轮没有为了
推进实验而扩大门禁策略。下一变量使用本轮 gate report 保存的完整 SC-01 candidate payload 做直接静态差分，正式缓存门禁仍保持发布阻断，最终
候选确定后再单独处理基线晋升范围。

启动期间有两个 Provider-zero 事件：第一次相对 `RunRoot` 在账本创建前失败；第二次证据目录名含 `taskspace`，触发 neutral-cwd 门禁并以
`provider_boundary_requests_minimum=0` 结算。二者均未消耗 API token，不计入有效样本和预算费用。

## 4. 下一因素

`SC-02` 只处理协议示例重复：保留首次 `initialize_map + client work` 与父节点完成后交接两个高价值示例，评估删除可由输入 schema 和硬规则直接
表达的 `read_map`、最终 finish 两个示例。不得同时修改协议规则、schema、Map、Runtime 或结果反馈。
