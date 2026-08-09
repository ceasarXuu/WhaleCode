# Phase B5 VA-02 复验结果

- Date: 2026-08-10
- Status: evidence complete / zero-Hosted contract verified online / VA-02 still blocked
- Model: `deepseek-v4-flash`
- Scope: `single-file-fast-fix` × `map-request` × repeat 1
- Record: `WAR-20260810-051702-CACHE-REGRESSION-EEF1DDF4`
- Zero-Hosted revalidation: `WAR-20260810-061241-CACHE-REGRESSION-A143B6F0`
- Subject: `98d0e52efe66c6ae09e781a39ba30d5ded151349`

## 1. 结论

第二轮证明当前生产链已经能完整执行合法的 `taskspace_exec`，并且 CP-13 后的 runner、Map 观测、provider boundary、usage、缓存和账本结算均可在线完成。但模型连续两轮都在首个响应生成了不可解析的 outer Function arguments，第二个响应才根据原始错误自行纠正。

随后获批的最小零 Hosted 复验证明该局部修复在线生效：首响应省略 `hosted_bindings`，严格解码、Map 初始化和嵌套 `exec_command` 均一次成功。VA-02 仍未通过，因为第二个响应生成了未声明的顶层 `exec_command`；Provider 实际收到的顶层 Tool 始终只有 `taskspace_exec` 与 `web_search`，Runtime 没有重新暴露普通 client Tool，并在执行前按合同拒绝。

因此当前阻塞从“是否会使用顶层 Exec”收敛为“首次 Exec 参数是否稳定合法”。VA-03 四臂测量继续阻断，不能用第二响应可自愈替代首响应稳定性验收。

## 2. 请求路径

| Request | Agent 行为 | Runtime 结果 | 副作用 |
|---:|---|---|---|
| 1 | 选择顶层 `taskspace_exec`，声明 `initialize_map + exec_command` | arguments 在 `hosted_bindings` 附近不是合法 JSON，严格解析拒绝 | 无 |
| 2 | 保持同一 Map 和 work 计划，修正 outer JSON | 预检通过；初始化 Map；执行 `exec_command` 并返回原始结果 | Map revision 3；inspect action succeeded |
| 3 | Agent 本应继续读取和修复 | 本地 provider request hard limit 在越界前返回 429 | 未进入 provider，不计 API 请求 |

第二请求创建的 Map 为：

```text
root -> inspect -> fix -> verify -> finish
```

节点和边均符合样本任务；`inspect` 绑定一次成功的 `exec_command`。本轮没有发生错误节点选择、Runtime 自动绑定、自动状态推进或语义修复。

## 3. 首次参数失败

首个 arguments 长度 751；严格 JSON 解析在 column 589 失败。模型把本应位于 outer object 的 `hosted_bindings` 写进了仍未闭合的 `calls` 数组：

```text
... "finish": {...}}}, "hosted_bindings": []}, {"tool": "exec_command", ...
```

第二个 arguments 长度 736，顶层字段为 `calls` 和 `hosted_bindings`，含两个 calls，可直接解析并执行。

前一轮 `WAR-20260810-044303-CACHE-REGRESSION-417B0312` 的首次失败同样位于空 `hosted_bindings` 附近，但表现为结尾多一个 `}`。两次错误形态不同，排除固定 Runtime 截断；共同点是无 hosted output 时仍要求模型填写空数组。

Runtime 当前行为正确：原始错误忠实返回、候选 Map 未提交、client Tool 未执行，不做括号修复或语义猜测。

## 4. 成本与缓存

| 指标 | 数值 |
|---|---:|
| Provider requests | 2 |
| Local wire attempts | 3 |
| Input tokens | 28,131 |
| Cached input | 27,520 |
| Uncached input | 611 |
| Output tokens | 633 |
| Request 2+ cache hit | 96.20% |
| Elapsed | 31.418 s |
| Estimated known cost | USD 0.000339836 |

第一请求自身命中 99.50%，第二请求命中 96.20%。这证明当前 `map-request` 的连续追加与静态 Tool shape 可以保持高缓存命中；旧 accepted manifest 未晋升仍是发布门禁状态，不等于本轮 provider 缓存异常。

## 5. 已验证修复

1. CP-13 删除旧 Map management consumer 后，runner 成功生成完整 metrics、报告和 durable evidence，没有再因 `node.results` 空 ID 中止。
2. usage 分析按 provider boundary ID 统计为 2 个请求，同时保留第三次 local-only 失败；没有再次把完整 usage 判为零。
3. `taskspace_exec` request/finalize/preflight/persist/complete 事件携带同一 capability identity、provider request identity、outer call identity、Map identity 和 revision。
4. 第二响应的 client result 原样进入唯一 outer feedback，Map action 只记录 Tool identity 和 outcome。

## 6. 当前阻塞与修复收敛

第二轮结束时的直接阻塞是：无 provider-hosted output 的普通编码请求仍要求 Agent 生成 `hosted_bindings: []`。该机械空字段在两轮首次响应中都成为 JSON 结构错误的相邻位置。

用户已批准并完成以下最小修复：

- 无 hosted output 时允许省略 `hosted_bindings`，canonical 示例也省略；
- 存在 hosted output 时仍必须逐项声明，漏绑、错绑和少绑继续由 response-local preflight fail closed；
- 不增加 Runtime 默认归属，不修复非法 JSON，不改变 client calls、node binding、Map 或 Standard；
- schema 和 description 改变属于缓存敏感面，必须先过免费门禁，再申请新的最小真实复验预算。

实现只使用静态 schema 必填列表与 Serde 默认空集合：未增加 Runtime 分支、兼容层或语义修复。TaskSpace Exec
69 项单测通过；缓存门禁以候选指纹 `e49cc5ff2184b34e08872ebaccf9c7d9bb92b947072befec0e2b467005a91a56`
识别出预期 final-wire 变化并允许候选提交，发布仍保持阻断。该修复随后已在最小真实复验中直接通过，但 VA-02 因第二响应的顶层 client Tool 越界继续阻断。

## 8. 零 Hosted 最小复验

第三次运行严格使用用户批准的 `single-file-fast-fix × map-request × repeat=1`、最多 2 个 Provider 请求且不重试。

| Request | Agent 行为 | Runtime 结果 |
|---:|---|---|
| 1 | `taskspace_exec` 内声明 `initialize_map + exec_command`，省略 `hosted_bindings` | 严格解码和预检通过；Map revision 2；`exec_command` 成功 |
| 2 | 生成顶层 `exec_command`，并把 `node_id` 混入其参数 | response contract 在副作用前拒绝；任务停止，代码未修改 |

该运行确认：

1. 零 Hosted 合同的目标已达成，首响应 arguments 是合法 JSON，省略字段没有丢失语义。
2. 两次请求的 `tools_hash`、`tools_count=2` 和 `tool_choice=auto` 完全一致；final-wire 的两个顶层 Tool 为 `taskspace_exec` 和 `web_search`，不存在 `exec_command`。
3. 第二响应是模型生成了未声明的顶层 Tool，不是 Runtime 动态改写 Tool schema。Runtime 拒绝属于不可绕过协议底线，没有替 Agent 做语义决定。
4. 性能观测器曾因 StrictMode 直接读取已合法省略的 `hosted_bindings` 而误报 `exec_arguments_invalid`；现已按缺省空集合读取并增加回归测试，不改变生产合同。

| 指标 | 数值 |
|---|---:|
| Provider requests | 2 |
| Input tokens | 28,368 |
| Cached input | 12,544 |
| Uncached input | 15,824 |
| Output tokens | 366 |
| Request 2+ cache hit | 54.69% |
| Agent sample elapsed | 25.507 s |
| Runner elapsed | 29.649 s |
| Estimated cost | USD 0.0023529632 |

VA-03 继续阻断。下一步应先把“已声明 outer Tool 下仍生成未声明内层 Tool 名”作为 I03 的当前生产表现归因；没有新预算不得再次运行真实 Agent。

新增证据：

- Result: `benchmarks/cache-regression/results/WAR-20260810-061241-CACHE-REGRESSION-A143B6F0.json`
- Durable evidence: `benchmarks/cache-regression/evidence/WAR-20260810-061241-CACHE-REGRESSION-A143B6F0/`
- Local trace: `target/cache-hit-regression/WAR-20260810-061241-CACHE-REGRESSION-A143B6F0/`

## 7. 证据

- Result: `benchmarks/cache-regression/results/WAR-20260810-051702-CACHE-REGRESSION-EEF1DDF4.json`
- Durable evidence: `benchmarks/cache-regression/evidence/WAR-20260810-051702-CACHE-REGRESSION-EEF1DDF4/`
- Local trace: `target/cache-hit-regression/WAR-20260810-051702-CACHE-REGRESSION-EEF1DDF4/`
- Ledger: `benchmarks/whale-agent-run-ledger.json`
