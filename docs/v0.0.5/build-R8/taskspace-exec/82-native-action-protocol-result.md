# TaskSpace 原生动作协议修复结果

- Date: 2026-08-19
- Issue: R8-I03
- Status: reverted / historical evidence only / I03 remains verifying

> 2026-08-19 设计澄清：`taskspace_exec` 替代的是 Codex 顶层 `exec` 超级工具，不是替代或重命名其内部原生 Tool。
> 本文记录的 `exec_command -> shell action` 候选基于错误的替代层级，commit `3750a3932` 已整体回退。以下实现与运行数据
> 仅作为失败候选的历史证据保留，不代表现行架构或修复方向。

## 已纠正的诊断

Provider 最终顶层只声明 `taskspace_exec` 和 Provider-hosted Tool，没有声明 `exec_command`。`taskspace_exec`
内部按原生名称表达 client Tool 是目标设计的一部分：

```json
{"tools":[{"tool":"exec_command","node_id":"inspect","input":{"cmd":"pwd"}}]}
```

历史逃逸调用同时携带原生 `cmd` 和 wrapper 专属 `node_id`，证明模型会把这个内部调用机械提升为顶层
`function_call(name=exec_command)`；但该证据不能推出“内部原生 Tool 不应暴露”。Codex `exec` 同样向 Agent 暴露内部
原生 Tool，只是使用 `tools.exec_command(...)` 命名空间，而不是 TaskSpace 当前的结构化 `tools[].tool` 判别字段。

在 `subscription-billing-repair × standard/always/append/request × repeat=3` 的历史四臂证据中，三个 TaskSpace 臂合计
`7/9` 轮、`11` 次顶层 `exec_command` 逃逸；每个受影响运行的第一次逃逸都紧跟一个成功的 `taskspace_exec` 反馈，且该反馈
包含 `client_results[].tool="exec_command"`。这对“成功反馈重复裸工具名会放大下一轮层级提升”构成强关联证据，但尚无只改变
反馈字段的 A/B，因此不能写成唯一根因。

全量历史分类进一步确认：排除已回退 `shell` 候选、已删除 Hosted 双写协议和离线 fixture 后，当前有效设计中只有
`exec_command` 发生过顶层逃逸；没有发现其他原生 client Tool 同类行为。因此后续不得把问题泛化为所有内部 Tool 的共同
作用域缺陷，应先验证 `exec_command` 特有的直接调用先验、首轮高频位置和反馈重复显著性。

进一步链路审计坐实了“如何发生”：Chat 适配器始终把 outer `taskspace_exec` output 绑定回原 `tool_call_id`，不会读取
`client_results[].tool` 并生成新调用；逃逸调用还保留正确 `node_id`，所以不是 Map/context 丢失。实际缺口是内部作用域只存在于
`taskspace_exec` 的嵌套参数和文字合同中，而 Provider 接受未在本轮顶层 schema 声明的 Function 名。模型可据此输出一个混合
结构：TaskSpace 的正确 owner + 原生 Tool 参数 + 错误顶层 Function 名。

历史转换统计显示，成功 `exec_command` 后再次执行 `exec_command` 时为 `18/199` 次提升；成功 `apply_patch` 后执行
`exec_command` 为 `0/102`。其中初始化后的第一次连续探索为 `13/105`，普通 work 后为 `1/81`。该集中性支持
`exec_command` 高频连续调用、canonical 示例和成功反馈裸 identity 是放大因素，但没有单变量 A/B 前不得把任一项单独写成
唯一根因。反馈长度已被排除：逃逸组的结果中位长度反而低于正确组。

## 已回退的错误候选

候选曾把 Agent 可见合同改为 TaskSpace 自建动作语言：

```json
{"actions":[{"kind":"shell","node_id":"inspect","parameters":{"cmd":"pwd"}}]}
```

该候选错误地把“`taskspace_exec` 替代 Codex 顶层 `exec` 超级工具”理解成“TaskSpace 还要替代 Exec 内部原生 Tool”。真实运行
只把被提升名称从 `exec_command` 改成了 `shell`，并未解决层级提升。commit `3750a3932` 已由 `ab23d8f5b` 整体回退；当前
恢复 `tools[] / tool / input`、原生 Tool identity、原生 Router 与结果合同，不保留 `shell` 兼容分支。

## 回退验收

- `cargo test -p codex-core taskspace --lib --locked`: 123 passed。
- 当前 schema 恢复 `tools[].tool="exec_command"`，并继续由 Runtime 机械解析到原生 Router。
- TaskSpace 顶层 Tool 集合仍不声明 `exec_command`；非法顶层 client Tool 的零副作用拒绝边界不变。

## 证据边界

本轮只完成错误候选回退和根因边界纠正，没有完成新的生产修复。I03 只有在新的单变量修复和真实运行中证明初始化、后续工作、
Patch、验证和结束均持续使用 `taskspace_exec`，且不再生成顶层普通 Function Tool 后才能关闭。

## 真实运行复验

获批计划是 `release-dispatch-repair × map-request × repeat=5`。通用 pair runner 会在物理 left/right 间交替放置逻辑模式，
但本轮错误使用 `RunSide=right`，实际得到 3 次 map-request 和 2 次 Standard；因此不能声称完成五次 TaskSpace 验收。

三次有效 TaskSpace 运行均通过业务、公开验证、隐藏 Oracle 并闭合 Map，共 34 requests、639,189 input、569,856 cached、
69,333 uncached、18,891 output：

1. 顶层 `exec_command` 为 `0/3 runs, 0 calls`，旧同形名称提升未复现；
2. 两轮把新的 `kind=shell` 提升成未声明顶层 `shell`，合计 5 calls；Runtime 全部在副作用前拒绝，Agent 下一请求恢复；
3. 另一轮始终使用 `taskspace_exec`，但有两次普通 schema 错误并在下一请求纠正；
4. 自动把非法顶层 action 包回 Exec 不可接受：两次误用没有 `node_id`，Runtime 无法忠实恢复 Agent 未声明的节点归属。

本轮只证明错误候选会把被提升名称从 `exec_command` 改成 `shell`，没有解决抽象层级逃逸。它不能证明内部原生 Tool
应该被隐藏、改名或替换。I03 保持 `verifying`；原计划中的缓存双臂因行为验收未通过而未执行，不记录为零值结果。

证据：`benchmarks/taskspace/r8/evidence/WAR-20260819-064028-R8-NATIVE-ACTION-R5.json`。
