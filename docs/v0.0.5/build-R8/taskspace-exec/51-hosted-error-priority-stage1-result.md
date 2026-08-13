# Hosted 错误优先级第 1 阶段结果

- Date: 2026-08-14
- Subject: `a54cae056dbcd881f9c8462df2521d757fc3f2ac`
- Model: `deepseek-v4-flash`
- Planned scope: `provider-web-search-probe × map-request × repeat=3`
- Actual scope: `repeat=1`；首轮业务失败后按用户停点停止
- Result: **离线改动成立，真实阶段未通过，不进入第 2 点**

## 1. 单变量改动

预检现在先核对本响应实际发生的 Hosted Tool 集合与 `taskspace_exec` 的 `already_executed` 登记集合，再校验
client Tool 所属节点是否可执行。该改动只调整错误反馈优先级，不改变 Tool 执行、Map 状态、节点绑定或 Agent 协议。

离线验证：Hosted preflight 5/5 PASS；缓存敏感面指纹保持
`19b2395a7c61547d38d91813fc8238897b119333bcba34c9102b0b9239240af2`。

## 2. 真实运行结果

| Runs | Requests | Input | Cached | Uncached | Output | Req 2+ cache | 耗时 | 估算费用 | 结果 |
|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| 1/3 | 12 | 496,940 | 445,952 | 50,988 | 12,222 | 89.49% | 146.550 s | USD 0.0118091456 | FAIL |

后两次 repeat 未执行。前 12 个 Provider 请求均返回 200；实际文件通过公开验证和隐藏 oracle，但 Agent 在本地校验成功后
发起第 13 个收尾请求时，被本次预算代理的每-run 12 requests 硬上限以 429 拒绝。该请求没有转发到 DeepSeek，也没有模型
usage。Map 保持 `complete=false`、`validate=in_flight`、`finish=waiting`，因此不能把业务文件正确扩大为 TaskSpace 生命周期通过。

## 3. Trace 结论

本轮没有出现“Hosted 漏登与 waiting client 同时存在”的目标复合输入，因此没有在线命中本阶段改变的错误优先级分支。
它实际暴露的动作链是：

1. 首次 Exec 缺少一个 JSON 闭合符，第二次又错误增加顶层 `arguments` 包装；第三次才完成 Map 初始化。
2. Agent 在 Provider Tool 尚未发生时提前登记一次 `web_search`，Runtime 正确拒绝。
3. 下一响应真实执行 `web_search`，但 Exec 只包含两个 client Tool，漏掉 Hosted 归属；Runtime 准确报告漏登。
4. 再下一响应中，Agent 成功把原生 `web_search` 与 `already_executed` 归属放在同一响应，Hosted 归属成立。
5. 两次 Map 扩展因 DAG 不合法被拒绝；随后 Agent 修正 Map，写入文件并完成本地校验。
6. 最终还需要一次 Map 收尾请求，但它是第 13 个请求，被本地预算硬门禁拒绝，任务被中断。

第 1 点的代码改动没有证据表明引入了上述结构错误或 429；它不改变 Agent 可见 schema/prompt，并且本轮目标复合分支没有被调用。
但真实验收既未完成生命周期，也未直接验证目标分支，因此不能判通过。

## 4. 基础设施记录

正式运行前有三次零 Provider 请求的预检失败：未安装 attestation、相对 `run-root` 触发 runner 路径错误、以及
`run-root` 名称包含 `taskspace` 触发中性 cwd 门禁。它们均未消耗模型 token，不形成产品结论；最后一次已由账本记录为
`WAR-20260814-040257-CACHE-REGRESSION-88C74A21`。

## 5. 停点

按照用户要求，本阶段未通过后暂停，不实施第 2～4 点。下一步需要先决定：

- 保留第 1 点，先用确定性复合错误测试直接证明优先级，再重新安排真实验证；或
- 先处理首次 Exec 结构稳定性和协议使用路径，再恢复四阶段实验。

## 6. 证据

- Result: `benchmarks/cache-regression/results/WAR-20260814-040440-CACHE-REGRESSION-06CF20B8.json`
- Evidence: `benchmarks/cache-regression/evidence/WAR-20260814-040440-CACHE-REGRESSION-06CF20B8/`
- Full local trace: `target/r8-hosted-priority-stage1/run/`
