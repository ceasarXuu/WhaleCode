# Phase R3-C. Provider-visible Integration and Payload Proof

## C.1 目标

把 R3-A/B 的 context compiler 接入真实 provider request 路径，并重写 active
replacement proof，使它证明真实发送给 DeepSeek 的 payload 使用了 bundle，而不是仅证明
本地 projection 看起来正确。

## C.2 当前失败解释

Phase H 证明：

```text
provider_payload_available = true
exact_payload_scan_matching_provider_event = true
active_projection_present = true
replacement_confirmed = false
legacy_taskspace_history_present = true
raw_taskspace_control_history_tokens = 917
protected_items_present = false
```

这说明：

```text
active projection 已经进入 provider payload。
但 provider-visible payload 仍不能证明旧 TaskSpace 历史被替换。
现有 scanner 还可能把 projection 中合法 action guidance 误判为旧历史。
```

R3-C 的修复重点是生产路径和证明路径同时收敛。

## C.3 集成边界

| Current Path | R3 Target |
|---|---|
| `append_context_projection_active` 直接生成自然语言 projection | compiler 生成 bundle，再渲染 provider view |
| `prepare_provider_visible_prompt_items` 做分类过滤 | compiler 决定 provider-visible items |
| `prepare_taskspace_action_contract_prompt_items` 另走裁剪逻辑 | action-contract 使用 compiler profile |
| `client.rs` 扫描 payload 字符串 | client 验证 bundle manifest 与 exact payload hash |
| release script 读 scan booleans | release script 读 structured proof + scan negative checks |

## C.4 Payload proof 新规则

通过条件：

```text
provider_payload_available = true
bundle_id is present on provider request event
exact_context_bundle_verified = true
exact_payload_scan_matching_provider_event = true
provider_payload_sha256 matches manifest
raw_taskspace_history_tokens = 0
legacy_taskspace_sections_present = false
protected_items_verified = true
cache_plan_verified = true
```

不得通过的情况：

```text
hash-only proof
synthetic producer
missing provider request join
bundle generated but not used by provider payload
fallback path used without explicit diagnostic status
protected items missing
raw taskspace tool call/output body present
```

## C.5 Scanner 调整

Scanner 仍需要保留，但它不应承担理解全部上下文语义的职责。

| Check | Old Approach | R3 Approach |
|---|---|---|
| active replacement | grep active marker and legacy marker | verify bundle id, section hashes, provider payload join |
| raw control history | grep `taskspace_control(` globally | scan raw history sections and tool call/output bodies only |
| protected items | grep `- protected` | verify structured protected_items and evidence refs |
| large raw output | inspect payload body size and markers | keep negative raw body scan |
| cache plan | not part of scanner | verify cache plan hash fields on provider event |

## C.6 实施任务

| Task | Production Code Path | Expected Behavior |
|---|---|---|
| provider request carries bundle id | `client.rs`, `session/turn.rs` | request event links bundle to payload |
| exact payload scan uses manifest | `client.rs` | scan validates bundle proof and negative raw-body checks |
| release decision reads bundle proof | `write-release-decision.ps1` | release blocks weak proof |
| cost instrumentation parses bundle metrics | `cost-instrumentation.ps1` | cache/context metrics available |
| payload fixtures cover false positives | tests/scripts fixtures | legal action guidance not flagged |

## C.7 完成证据矩阵

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| Provider bundle join | request event carries bundle id | client/session | provider request | provider_request_budget tests | provider trace tags | none | planned |
| Exact proof rewrite | manifest/hash/scan agree | client scanner | payload capture | exact scan tests | exact scan event | none | planned |
| Release fixtures | weak proof blocked | release script | release decision | release fixture tests | release-decision.json | none | planned |
| Cost summary | context/cache metrics parsed | cost instrumentation | benchmark artifact | PowerShell tests | cost summary JSON | none | planned |

## C.8 验证

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | provider payload uses bundle | exact payload fixture | manifest hash matches payload |
| Correctness | no old raw TaskSpace body | scanner negative checks | raw history tokens = 0 |
| Correctness | legal action guidance allowed | payload fixture | no false legacy hit |
| Benefit | active replacement proof | B-tier smoke | replacement_confirmed=true |
| Benefit | payload cost reduced | cost diagnostics | provider_direct_input_output_ratio below agreed threshold or diagnosed |
| Benefit | cache preserved | provider cache summary | request_2_plus_hit_rate >= 0.95 |
| Observability | join trace complete | event/artifact inspection | request_id, bundle_id, payload_sha all present |

## C.9 Exit criteria

```text
Focused unit tests pass.
PowerShell release/cost fixture tests pass.
B-tier smoke shows exact_context_bundle_verified=true.
B-tier smoke shows raw_taskspace_history_tokens=0.
B-tier smoke shows protected_items_verified=true.
B-tier smoke keeps request_2_plus_hit_rate >= 0.95.
```
