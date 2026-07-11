# R5 Input Token 优化审计

- Created: 2026-07-12
- Updated: 2026-07-12
- Status: Analysis Complete / Implementation Not Started
- Scope: R5-J6.6 `count-call-stack` paired run
- Evidence: `target/r5-j6-6-active-single-expression/count-call-stack/count-call-stack/20260712-065907-459`

## 1. 结论

R5仍有Input优化空间，但主项已经不是普通工具schema。当前31,650 tokens差距中，约87%-90%来自
3次额外Provider请求重复携带完整前缀；剩余约3,100-4,000 tokens主要来自Map初始化、finish摘要、
active projection和控制反馈之间的结构性重复。

优化必须遵守以下边界：不压缩或改写Agent语义，不隐藏失败反馈，不让Runtime推断下一动作，不恢复
active ordinary schema双重表达，不用破坏append-only前缀换取表面token下降。

## 2. 当前分账

| Metric | Standard | R5 J6.6 | Delta |
|---|---:|---:|---:|
| Provider requests | 6 | 9 | +3 |
| Input tokens | 43,666 | 75,316 | +31,650 |
| Cached input | 41,088 | 61,952 | +20,864 |
| Uncached input | 2,578 | 13,364 | +10,786 |
| Output tokens | 946 | 1,658 | +712 |
| Active non-message bytes/request | 21,685 mean | 22,496 mean | +811 |

按动作路径移除standalone finish后的patch请求、拆开的validator请求和最后单独end请求，可少发送
8,740 + 9,196 + 9,704 = 27,640 input tokens，占总差距87.3%。按尾部三次请求计算，上界为
28,531，占90.1%。这是反事实区间，不把不同Agent路径伪装为精确因果分账。

若只比较前6次请求的non-message固定区，R5合计126,710 bytes，Standard合计130,108 bytes；R5
因blank-map bootstrap只发送一个control，反而少3,398 bytes。因此继续压普通工具schema不是当前优先项。

R5第3次请求以后缓存命中为61,952 / 63,691 = 97.27%。当前87.30%的request-2+命中率主要被
blank named-control切换到active auto tools时的一次新shape冷启动拉低；active shape稳定后缓存不是主问题。

## 3. 已确认的结构重复

### 3.1 Activation与blank projection重复

首轮额外developer内容包括511 chars的mode transition notice和1,217 chars的blank projection。
其中以下信息被重复表达：

- TaskSpace active / mode=taskspace；
- runtime管理Map硬状态；
- mechanical blank来源；
- objective和node plan pending；
- hard state为无节点；
- `initialize_then_actions`初始化合同。

transition notice还写着Runtime执行`nested actions`，但J6.6 active状态已经改为top-level sibling ordinary
calls。该文案既增加成本，又与当前工具协议不一致。

### 3.2 Map初始化强制Agent重述用户语义

本轮`initialize_then_actions`参数为1,206 chars。Agent被要求同时生成：

- `task_title`和`task_objective`；
- 每个node的`title`和`context_summary`；
- 原始普通工具参数。

task objective重复原始用户请求，node title与context summary也高度重叠。首轮输出为393 tokens，Standard
首轮仅70 tokens；这部分Agent生成内容会被后续8次请求重复携带。

### 3.3 Finish摘要重复已有事实

两个`finish_nodes`参数均为262 chars，`finish_then_end`为551 chars。`result_summary`再次描述已经存在于
工具输出、Agent历史和最终回答中的事实。Map需要完成状态和结果归属，但不必强制每个简单节点再生成一份
自然语言摘要。

### 3.4 当前是固定epoch base，不是逐请求最新Map

9次请求的projection message均为1,796 wire bytes，content hash完全相同；runtime trace也只有一次
`projection_budget`，估算projection body为189 tokens。其内容一直是epoch起点的mechanical blank Map。
后续`initialize_then_actions`和finish调用/输出作为append-only journal delta进入自然历史，因此完整顺序是：

```text
blank epoch snapshot -> initialize delta -> ordinary feedback -> finish delta -> ...
```

这不是状态信息丢失，但当前文案容易被理解为“当前权威Map”，而实际只是epoch base。每次请求都重发
511-char activation和1,217-char blank projection，约432估算tokens/request；大部分可命中缓存，但仍计入
总Input。

正确方向不是每请求刷新完整Map。若把动态projection放到稳定prefix前部，会破坏当前active shape的
97.27% warm cache。应将epoch base压到最小，并继续用Agent原始control journal表达delta；只在新epoch或
compaction后生成一次当前Map snapshot。

### 3.5 Populated projection存在潜在内部重复

renderer在新epoch从已有Map生成snapshot时，`current_node_recent_events`最多选择6个事件、每个excerpt
最多1,200 chars，并输出event id、node、kind、source、success、raw ref、artifacts、command、长度和
正文；`result_refs_available`又为同一批事件重复大部分metadata。

该重复未在本轮固定blank snapshot中实际展开，但代码路径会在新epoch/compaction snapshot中出现。
届时同一事实还可能同时存在于保留的tool history和finish summary，必须由组件telemetry验证实际成本。

### 3.6 Control成功输出Envelope偏重

两个非终态finish输出为242/234 chars，terminal输出215 chars，重复`schema_version`、`action`、`status`、
`success`、step kind和step success。失败输出必须完整保留，但成功ack可以使用更稀疏的typed result。

## 4. 推荐顺序

### P0：补齐精确分账观测

当前`context-projection-summary.json`显示`projection_unavailable`，但rollout内`projection_budget` trace
实际存在189-token估算值，说明benchmark extractor漏取了嵌在snapshot中的trace event。先修复提取，再增加
不记录正文的request级分账：activation bytes、projection bytes、event excerpt bytes、ref bytes、control
history bytes、ordinary history bytes和tool schema bytes，并记录hash/count。

验收：所有TaskSpace request覆盖率100%，组件bytes之和与最终message/tools payload一致；不落盘敏感正文。

### P1：Projection稀疏化而非语义压缩

1. 首个epoch只表达最小base identity和hard state，不把blank base写成持续变化的当前状态。
2. 删除叙述性preamble，只保留版本和机械Map数据。
3. `projection_id`、固定mode、重复map title等可推导字段不再重复发送。
4. current node不再同时完整出现在node skeleton中。
5. 空值、false和zero使用schema默认，不逐字段输出`none`。
6. populated snapshot合并recent events与result refs；同一event只表达一次。
7. 原始输出仍在可见history时只投影event id/status/ref；history被压缩或输出被裁剪时才渐进暴露excerpt/ref。

第7项只依据“内容是否仍可见”这一机械事实，不评估内容重要性，不做语义摘要。不得改成每请求重建
完整Map；epoch内继续保持append-only base + delta journal。

### P1：Map写入契约最小化

1. 根objective引用原始user message/event，不要求Agent复制用户请求；允许Agent在需要时显式覆盖。
2. node只保留`node_id`、`kind`、一个`goal`和dependency，删除title/context summary双份表达。
3. `result_summary`改为可选；默认把当前node已有events/refs机械归档为result依据。
4. terminal final candidate仍由Agent原样提供，不由Runtime生成或压缩。

### P1：修正Activation机械协议

删除没有真实前序对话时的`Previous standard-mode conversation`文案，并删除过时的`nested actions`描述。
Activation只说明Map硬状态和“按可见tool schema、Provider call order执行”，不添加策略或行动建议。

### P2：收敛请求次数

这是最大收益项，但不是上下文裁剪。优先验证修正协议矛盾后Agent是否开始采用现有
`finish_nodes + sibling ordinary call`和`finish_then_end.preceding_finishes`。如果仍不采用，再单独评估
tool schema层的机械组合表达；不得由Runtime自动补动作，也不得恢复大体积ordinary union wrapper。

### P3：成功Control输出稀疏化

成功结果只返回状态变化、result id和next binding；失败结果继续完整返回错误类别、原始原因和失败step。
该项收益较小，放在projection和Map契约之后。

## 5. 明确不做

1. 不对工具正文做LLM摘要或语义优先级裁剪。
2. 不删除失败stderr、exit code、patch反馈或output refs。
3. 不让Runtime因为“看起来可以并行”而自动重排Agent动作。
4. 不用动态替换自然历史破坏当前97.27%的warm active cache。
5. 不重新在active control中嵌入完整ordinary schema。
6. 不把单样本Agent采用率问题伪装成projection必须提示或约束Agent。

## 6. 预期收益区间

| Direction | 单样本Input潜力 | 置信度 |
|---|---:|---|
| 3次请求收敛 | 约27.6K-28.5K | 高，路径直接观测 |
| Map初始化/finish减少重复生成 | 约2K-4K累计 | 中，需精确组件telemetry |
| Projection去重和稀疏化 | 约1K-3K累计 | 中，当前projection正文未落盘 |
| 成功control ack稀疏化 | 数百tokens累计 | 中高 |
| 继续压active工具schema | 很小；不建议优先 | 高 |

各项存在重叠，不能相加作为承诺。P0 telemetry完成后再冻结收益门禁。
