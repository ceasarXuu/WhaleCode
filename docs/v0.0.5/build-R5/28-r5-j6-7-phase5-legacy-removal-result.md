# R5-J6.7.5 旧双轨代码物理删除结果

- Date: 2026-07-12
- Phase: R5-J6.7.5
- Status: Complete
- Next: J6.7.6 收益门禁与收口

## 1. 结果

J6.7.5 已完成旧 TaskSpace 双轨路径物理删除。TaskSpace provider 输入现在直接使用 canonical Event
Store 的原生 item 顺序，不再经过正文分类、过滤、配对或语义 composer；Map 只保存状态、event ID、
artifact ref 和机械成功状态，不再保存 `body/preview/visible_excerpt`。

本阶段净删除约 1,500 行 Rust 旧路径，并移除以下 production 行为：

- legacy/shadow/compact/thin projection marker composer；
- Runtime 根据工作区 diff 反推“实现成功”并回填 Map；
- Runtime feedback 正文副本和无 caller 的 node event 写入；
- `NodeResult.body`、`NodeEvent.body/visible_excerpt` 与旧 summary 字段；
- bundle/cache-plan/legacy-history/stale-node/rejected-body 的 dead payload telemetry；
- observer 从 result body 反解析 tool、call ID、success 和 preview 的兼容 parser。

Standard 继续使用原 `ConversationHistory`，没有改动其 compaction、恢复或 provider 输入路径。

## 2. Nested 工具链路缺口与修复

首次 `count-call-stack` R5 诊断运行在文件修复和 validator 已通过后出现 95-request runaway。证据表明：

1. `initialize_then_actions` 内两个 nested ordinary calls 已执行成功；
2. 它们只存在于 outer control output 正文，没有独立 canonical call/output event；
3. J6.7.5 result attribution 因缺少 source event ID 失败，留下恰好两个 in-flight reservation；
4. 后续 finish 均被 `node_tool_calls_in_flight` 硬规则正确拒绝；
5. final rejection 原文持续进入上下文，反馈没有丢失或扭曲。

修复没有放宽状态机，也没有让 Runtime 推断任务完成。state barrier 成功后，每个 nested call/output
现在各写一个带 `parent_call_id` 和 node owner 的 canonical event；Map 引用 call event；outer control
output 只返回 call/output event refs，不再复制 nested output 正文。工具结束后 reservation 机械释放，
即使 attribution 遇到基础设施错误也不会继续伪装成 in-flight。

该根因和修复证据已记录到
`coe/2026-07-10-22-56-r5-request-amplification.md` 的 H-023 / E-054 / E-055。

## 3. Observer 合同

当前 observer 从 snapshot 读取 `sourceEventRef/artifactRefs/toolSuccess`，不再读取或回退
`body/preview`。completed task 会清空 `activeMapId`；新 canonical result 不再被合成为空
`evidencePackage/unreviewed`。发布门禁只检查当前 projection 的唯一性、必要结构、大正文引用化、
Runtime 越界 marker 和 provider request 精确关联，不再要求已删除的 bundle/cache-plan 指标。

## 4. 工程验证

| Gate | Result |
|---|---:|
| ActionMap / Event Store | 28 passed |
| tools 全模块 | 341 passed |
| active provider passthrough | 14 passed |
| rollout reconstruction | 23 passed |
| protocol Map events | 3 passed |
| codex-state | 118 unit + 3 bin + 1 doc passed |
| codex-rollout | 45 passed |
| observer self-test | PASS |
| benchmark harness | PASS |
| cost instrumentation | PASS |
| performance observation | PASS |
| release decision | PASS |
| locked Whale build | PASS |
| old caller/field scan | 0 hit |
| `cargo check` warnings | 0 |

## 5. Docker 横向复验

有效 run：`target/r5-j6-7-5-live/count-call-stack/20260712-123635-091`。

| Mode | Result | Requests | Runtime tools | Input | Cached | Uncached | Wall |
|---|---|---:|---:|---:|---:|---:|---:|
| Standard | solved | 8 | 12 | 62,150 | 59,264 | 2,886 | 16.51s |
| R5 | solved | 10 | 16 | 86,855 | 83,328 | 3,527 | 24.52s |

R5 request 2+ cache hit 为 95.82%，比 Standard 高 0.75 个百分点。R5 的 3 个 nested actions 形成
3 组独立 call/output events，`parent_call_id` 和 `node=explore` owner 完整；payload/output/call duplicate
与 orphan call/output 均为 0。Map 为 1 map、4 nodes、1 edge，task、map 和全部节点均 completed，
open nodes 为 0。

被中止的 `20260712-122002-493` 只作为 H-023 诊断证据，未生成完整 metrics/validator/pair gate，
不得进入收益统计。

## 6. Phase Gate

- old production caller：0；
- compatibility parser/branch：0；
- dead semantic duplicate fields：0；
- canonical nested pair completeness：100%；
- Standard/R5 correctness：通过；
- build/tests：通过。

J6.7.5 退出门满足，允许进入 J6.7.6。
