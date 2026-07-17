# R7 Phase C map-append 接入结果

- Created: 2026-07-18
- Updated: 2026-07-18
- Status: Complete / Phase D Ready
- Production Commit: `4366b95ec`
- Observer Commit: `f04b5573d`
- Diagnosis Commit: `49b185a0b`
- Machine Result: `benchmarks/taskspace/r7/phase-c-result.json`

## 1. 阶段结论

Phase C 已按用户澄清后的产品合同完成：`map-append` 不依赖 revision commit 或 tool result，而是在
**每轮 provider request 构造时，把当时最新完整 projection 机械、持久地追加为最后一条 message**。
因此上下文按 `A+P1 -> A+P1+B+P2` 线性增长；Map 未变化时，下一轮仍可再次追加同一 revision，最后
一份 projection 是唯一当前状态。

旧实现同时存在触发点和 carrier 缺陷：它只在 revision commit 后追加 `developer` snapshot，DeepSeek
adapter 又把该消息转换为会话中部 `system`。修复后 emission 由 `ProviderRequest` 触发，projection 在
provider-visible copy 中使用自然历史兼容的 `user` role。两个 Docker 样本共 31 次 TaskSpace request，
31/31 均以 projection 收尾，identity 对齐且 revision 非递减。

两组 Standard/R7 均通过 public/hidden validator，Map 均闭合。R7 request 2+ cache hit 从旧实现的
46.51%/69.36% 提升至 78.95%/87.35%，相同 request shape 的零命中为 0。旧 projection 累积、额外
request 和总 input 增长仍是 `map-append` 的已知产品成本。本阶段每臂只有 1 次，不用于效用排序。

## 2. 工程改造

| 区域 | 结果 |
|---|---|
| Envelope | 共享 envelope 使用 `request_snapshot`、`supersedes_all_prior_projections`、`last_projection_only` |
| Policy | `ProviderRequest` 触发 `AppendSnapshot`；revision 允许重复但不允许回退 |
| Ordering | 每轮最终 message 必须是当时最新 projection；不依赖 control/tool output carrier |
| Composer | projection 持久写入 canonical history，并在 provider-visible copy 中序列化为 `user` |
| Retry | 当前历史末项已是同一 projection 时不重复持久写入 |
| Resume/compaction | 从可见末项机械恢复 cursor，后续 request 继续执行同一末项合同 |
| Scanner | 校验 projection 末项、revision 非递减及 Map/revision/hash 四元 identity |
| Schema | 最后 projection 唯一权威，不增加 Agent 工作建议或 Runtime 语义判断 |
| Logs | 增加 `projection_is_message_tail`、request snapshot identity 与 request 级缓存字段 |
| Session | Phase C 开放 `map-always` 与 `map-append`；`map-request` 继续机械拒绝 |

Renderer 仍只忠实构造 canonical Map；policy 只决定 projection 如何进入 context。Runtime 没有增加
节点优先级、下一步动作、任务摘要或纠错建议。

## 3. Request-tail 与反馈门禁

| 指标 | Simple | Complex | 合计 |
|---|---:|---:|---:|
| TaskSpace provider request | 11 | 20 | 31 |
| bootstrap / request snapshot | 1 / 10 | 1 / 19 | 2 / 29 |
| 末项 projection | 11 / 11 | 20 / 20 | 31 / 31 |
| identity confirmed | 11 / 11 | 20 / 20 | 31 / 31 |
| 可见 revision | `2..9`，2/7 重复 | `2..9`，2/3/5/6 重复 | 非递减 |
| exact scan failure | 0 | 0 | 0 |
| tail / identity / regression violation | 0 / 0 / 0 | 0 / 0 / 0 | 0 / 0 / 0 |
| Map nodes / edges / open | 6 / 5 / 0 | 6 / 5 / 0 | 全部闭合 |

两组运行分别出现 2/5 次 control failure，均以明确失败反馈进入历史，Agent 随后纠正并完成任务；
Complex observer 另记录 4 处生成参数的诊断解析失败，但不影响 Runtime 原始 tool result、scanner 或
最终验证。未观察到 projection 导致的反馈丢失、扭曲或错误提交。

## 4. 三臂快速对照

Frozen R6 来自 Phase A 冻结基线；Standard 和 R7 是本阶段新成对运行。模型均为
`deepseek-v4-flash`、reasoning effort 为 `max`，执行边界均为 Docker hard boundary。

### 4.1 Simple：single-file-fast-fix

| 指标 | Current Standard | Frozen R6 | R7 map-append |
|---|---:|---:|---:|
| 结果 | solved | solved | solved |
| provider request | 7 | 19 | 11 |
| runtime tool / control | 9 / 0 | 18 / 9 | 10 / 10 |
| wall time | 13.76s | 46.55s | 25.34s |
| input token | 48,627 | 231,221 | 148,311 |
| cached input | 46,592 | 207,744 | 114,816 |
| uncached input | 2,035 | 23,477 | 33,495 |
| output token | 1,394 | 4,499 | 2,584 |
| request 2+ cache hit | 95.51% | 89.74% | 78.95% |
| message prefix preserved | 100.00% | 88.89% | 80.00% |
| Map nodes / edges / open | 0 / 0 / 0 | 5 / 4 / 0 | 6 / 5 / 0 |

R7 相对当前 Standard：request `1.57x`、wall `1.84x`、input `3.05x`、cached input `2.46x`、
uncached input `16.46x`。其中 request 2 是唯一零命中，trace 将其归类为初始化后的
`named_function -> auto` tool-choice 形状切换，不是 same-shape miss。

### 4.2 Complex：subscription-billing-repair

| 指标 | Current Standard | Frozen R6 | R7 map-append |
|---|---:|---:|---:|
| 结果 | solved | solved | solved |
| provider request | 12 | 16 | 20 |
| runtime tool / control | 20 / 0 | 22 / 7 | 24 / 13 |
| wall time | 47.22s | 58.93s | 91.68s |
| input token | 126,172 | 209,772 | 453,446 |
| cached input | 119,424 | 184,704 | 393,600 |
| uncached input | 6,748 | 25,068 | 59,846 |
| output token | 5,223 | 5,767 | 10,527 |
| request 2+ cache hit | 94.44% | 87.87% | 87.35% |
| message prefix preserved | 100.00% | 86.67% | 89.47% |
| Map nodes / edges / open | 0 / 0 / 0 | 4 / 3 / 0 | 6 / 5 / 0 |

R7 相对当前 Standard：request `1.67x`、wall `1.94x`、input `3.59x`、cached input `3.30x`、
uncached input `8.87x`。20 个 request 没有零命中，主 `auto` loop 为 84%-97%；低点主要位于初始化与
最终收口的 tool-choice 形状切换。R7 只执行 2 个 patch，Standard 执行 5 个，双方每 request 均不超过
1 个 patch。

## 5. 输入与缓存解释

| 指标 | Simple | Complex |
|---|---:|---:|
| provider request 累计可见 projection 历史 | 129,691 B / 32,428 est. tokens | 512,115 B / 128,036 est. tokens |
| 累计自然历史 | 62,106 B / 15,530 est. tokens | 425,768 B / 106,452 est. tokens |
| 累计 tool schema | 295,911 B / 73,986 est. tokens | 538,020 B / 134,520 est. tokens |
| 累计 TaskSpace control feedback | 28,629 B / 7,160 est. tokens | 64,100 B / 16,031 est. tokens |

缓存修复是明确收益：旧 `developer -> system` carrier 被移除后，两个样本分别提升 32.44/17.99 个
百分点，且 same-shape zero hit 均为 0。message-level prefix 指标仍低于 100%，因为 scanner 会看到
每轮新增自然历史和尾部 projection；它不能替代 provider 返回的 cache token 事实。

input 并未随缓存修复消失。每轮都追加完整 projection 意味着旧版本在后续每个请求中继续计费；上表
`active_projection` 原始分类名在 append 模式下实际汇总的是“当前加历史 projection 消息”，不是只有
最后一份当前状态。再叠加 R7 的额外 request/control，形成 Simple `3.05x`、Complex `3.59x` 的总
input。这是当前 `map-append` 的设计成本，不再误判为 carrier bug。

## 6. 测试结果

```text
cargo check -p codex-core --tests                           PASS
cargo fmt --all -- --check                                  PASS
cargo test -p codex-core projection --lib                    24 passed
provider map-append / rollback / session policy tests        PASS
codex-tools taskspace control tests                          4 passed
R7 projection policy contract                               PASS
cost instrumentation selftest                               PASS
performance observation selftest                            PASS
TaskSpace benchmark harness self-test                        PASS
Docker simple/complex Standard + R7                          all solved
```

完整 `codex-core --lib`：`1891 passed / 25 failed / 3 ignored`。失败项与 Phase C 修复前的 25 项集合一致；
新增 Phase C 测试全部进入通过项。两个既有 TaskSpace 子代理 spawn/replay 失败继续记录在
`coe/2026-07-18-03-15-r7-subagent-map-restore.md`，其余为共享 `/tmp`、file watcher、guardian key
注入、临时 diff 路径等非本阶段失败。

## 7. 无效运行与运行经验

旧 `052006` 运行保留为修复前诊断证据，不能与新实现混算。新有效运行根目录为：

```text
target/r7-phase-c/request-tail/single-file-fast-fix/20260718-072520-561
target/r7-phase-c/request-tail/subscription-billing-repair/20260718-072634-693
```

以后从 shell 启动 benchmark 时，应使用 `set -a; source .env.local; set +a` 将环境显式导出，且不得
打印 secret。当前每臂 `repeats=1`，只验证工程合同和定向成本变化，不生成统计效用结论。

## 8. Phase D 准入

Phase C 的 request-tail、correctness、feedback 与 cache 缺陷门禁均满足，可以进入 Phase D：

1. 31/31 provider request 的末项 projection 与 identity 可精确复核；
2. 同 revision 跨 request 重复符合合同，revision regression 为 0；
3. projection 不再依赖 revision/tool carrier，DeepSeek wire 不再插入 system；
4. request 2+ cache 明显恢复，same-shape zero hit 为 0；
5. 输入增长和旧 snapshot 累积已完整量化；
6. shared renderer/composer 未产生策略架构分叉；
7. Standard/R7 simple 与 complex 均 solved，完整回归无新增失败。

Phase D 只能在共享 `taskspace_control` 上增加 `read_map`；不得修改 Map 状态机、ordinary tool 权限
或复制 renderer/provider context 路径。Phase C 单次样本不支持三策略效用排序，正式结论仍需后续重复
实验。
