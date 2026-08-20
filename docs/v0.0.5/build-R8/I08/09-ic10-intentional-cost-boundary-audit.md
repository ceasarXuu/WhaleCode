# IC-10 TaskSpace 设计成本边界审计

- Status: complete / no production change
- Date: 2026-08-17
- Evidence: IC-09 repeat=5 final wire、rollout、当前生产代码与历史单变量实验
- Paid Whale Agent run: 0

## 1. 结论

当前 TaskSpace 单轮 input 增量没有暴露一个可无损整块删除的主要结构。大部分增量是已确认产品能力的直接代价：Agent 维护
Map、声明节点归属和合法序列，Runtime 返回原生 Tool 事实与本轮 canonical 状态。后续不得以成本为由删除这些语义。

审计发现一个小型等价压缩候选：`unavailable_direct_work_children` 内的自然语言 `message` 与结构化字段逐值重复，子节点
`state` 在该集合中恒为 `waiting`。历史 trace 静态反算最多减少 2,343 B，约 86.78 B/成功 Exec、占全部单轮新增历史
3.03%。该候选曾有“提高下游不可执行语义显著性”的产品背景，不能直接删除；只有在当前 Base `3.0.5` 真实验收后仍需继续
优化时，才适合作为单变量 A/B。

## 2. 五个方向

### 2.1 同一语义重复编码

| 候选 | 证据 | 判断 |
|---|---|---|
| Base 与 Tool 都描述状态机 | Base 只给宏观工作模型，Tool 给完整硬合同；Base 单变量曾将 Waiting 误选从 2/5 降到 0/5 | 分层显著性，不是无依据重复；保留 |
| Tool 总合同与 sequence branch 描述相似 | 总合同规定通用边界，branch 描述规定该结构何时合法 | 局部选择合同，不删除 |
| unavailable child 的 `message` | 完全重述 `node_id` 和 `incomplete_parent_ids` | 明确重复候选；1,947 B/五轮 |
| unavailable child 的 `state` | 进入该列表的条件已经保证 state=`waiting` | 明确派生候选；396 B/五轮 |
| client result 的 `node_id/tool/outcome` | 与调用历史有对应关系，但保证多动作结果自包含并可归属 | 有消费者的身份事实；保留 |
| `previous_state` | 五轮占 985 B；只在本轮真实改变时出现 | 忠实表达状态变化，不归入机械删除 |

### 2.2 JSON / Exec 等价紧凑化

单轮新增历史中的 Exec/JSON 机械外壳为 13,984 B：

- 9,966 B 是 Provider/Codex Function Call 与 Function Call Output 外层；Source 也需要，产品侧不可删除；
- 2,402 B 是 JSON 容器、字段、括号和分隔符；直接缩短字段会降低合同可读性，收益上限约占新增历史 3.1%；
- 1,616 B 是 client result 归属元数据；存在明确消费者。

29 个合法 Exec 参数中共有约 427 B 空数组字段和 300 B 空 `content` 字段。把这些字段改成可选默认值最多只影响新增历史
0.94%，却会扩大 schema 可选形状并重新引入 Agent 生成稳定性变量，当前不值得实施。

历史 Source 实现不是压缩方向：它仍用 Function Call 外层，并把同一内部协议转义进 description；历史 Tool section 为
57,882 B，对应 Structured 30,627 B，且出现 Source wrapper 缺失。因此继续保持 Structured 单协议。

### 2.3 状态反馈范围

当前实现没有返回完整 Map，而只返回：

1. Agent 本轮直接操作的节点；
2. Runtime 本轮机械改变状态的节点；
3. 这些节点当前不可执行的直接 Work 子节点及精确未完成父节点。

五轮共有 67 个 affected state item、8,011 B。其中特定 run 内逐字重复 6 项、1,434 B；这些项都发生在新的 Tool 操作后，
用于确认 owner 当前 canonical 状态，不是 stale Map 副本。用户已确认“返回操作过的 node 状态本应该做”，因此不改成只返回
state delta，也不删除 unchanged owner。

可探索边界只限于 2.1 的 `message` 与恒定 child state；完整父节点事实、当前 owner state 和真实 state transition 均保留。

### 2.4 非法动作与额外请求

IC-09 五轮的 35 次 TaskSpace 请求中有三次零副作用拒绝：

- 一次 JSON 字符串裸换行；已纳入唯一机械自愈并保证修复版进入正式历史；
- 两次父节点完成时显式提交 waiting child `in_flight`；Base `3.0.5` 已明确先完成父节点、由 Runtime 派生 Ready，并允许同响应
  提交刚解锁子节点 Tool。

错误 call/output 直接载荷只占新增历史 1.64%，但三次拒绝各增加一次完整 Provider 请求，才是主要异常成本。两项修复目前仅
离线通过，因此在真实复验前不增加第三种协议修复，也不把旧异常频率当成当前版本事实。

显式 Finish 后再生成最终总结仍是当前 Tool result 依赖带来的设计请求，不归类为非法动作。

### 2.5 Reasoning 与协议复杂度

三组无拒绝配对中，TaskSpace reasoning 每轮增加约 485～596 B，Standard 为 280～407 B。逐条 trace 显示，最长 reasoning
主要在分析税费舍入、测试预期和 Python rounding；TaskSpace 生命周期内容通常只是初始化、完成节点和 finish 的短句。异常 run
另有明确的 JSON/状态拒绝恢复 reasoning。

因此当前证据只支持“Agent 多处理了 Map 事务，所以 reasoning 有预期增量”，不支持“协议文字过度复杂是 reasoning 主根因”。
不以自然语言语义解析建立 Runtime 优化，也不因 reasoning 成本缩减状态机合同。

## 3. 候选排序

| 顺序 | 动作 | 预期收益 | 风险与前置 |
|---:|---|---|---|
| 1 | 真实验证 JSON 自愈与 Base `3.0.5` | 目标是消除三次额外请求，而非缩小单条反馈 | 需要新预算；两项必须共同作为当前版本验收，不再改协议 |
| 2 | 单变量移除 unavailable child `message` + 恒定 `state` | 静态上限 86.78 B/成功 Exec，约 3.03% 新增历史 | 只有顺序 1 通过后才值得；需验证 Waiting 行为不回归 |
| 3 | 继续观察机械外壳 | 上限小于完整 18.1%，大部分是 Provider 固有 framing | 只接受同义、等价、无消费者字段；不缩短公共原生 Tool |
| 4 | 空字段可选化 | 静态上限约 0.94% 新增历史 | schema 形状变复杂，收益不足；defer |

## 4. 不执行项

- 不删除 Map 更新、节点绑定、合法序列或 affected canonical state；
- 不回到 Source，不维护第二套 carrier；
- 不把 result 改成只靠数组位置或历史反查归属；
- 不自动压缩 Agent reasoning，不解析 reasoning 来控制 Runtime；
- 不在当前静态审计中消费真实 Agent 预算。
