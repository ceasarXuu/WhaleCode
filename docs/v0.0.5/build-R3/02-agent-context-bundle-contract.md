# Phase R3-B. Agent Context Bundle Contract

## B.1 目标

定义 `TaskSpaceAgentContextBundleV1`。它是 TaskSpace 传给 Agent 的语义上下文，
不是完整事件日志，也不是自由文本 projection。

Bundle 必须同时满足：

```text
Agent 能完成当前任务。
历史调查路径可见但不全量展开。
完整原始轨迹仍可通过 refs 渐进读取。
DeepSeek cache layout 可预测。
release gate 能结构化验证。
```

## B.2 Bundle 顶层结构

```text
TaskSpaceAgentContextBundleV1
  schema_version
  bundle_id
  compiler_version
  source_snapshot_hash
  stable_prefix
  task_map_snapshot
  current_node_focus
  nearby_nodes
  completed_path_summary
  accepted_facts_and_decisions
  evidence_refs
  dynamic_turn_delta
  retrieval_ref_index
  omission_audit
  cache_plan
  verification_manifest
```

## B.3 可见上下文分区

| Section | Agent Visible | Cache Region | Content | Limits |
|---|---|---|---|---|
| `stable_prefix` | yes | stable | TaskSpace schema、action schema、固定约束 | 版本变更才变 |
| `task_map_snapshot` | yes | semi-stable | task objective、map skeleton、criteria、current path | bounded nodes |
| `current_node_focus` | yes | dynamic | 当前 node kind/status/allowed actions/recent evidence | highest detail |
| `nearby_nodes` | yes | dynamic | dependencies、next candidate、parent/child path | medium detail |
| `completed_path_summary` | yes | semi-stable | 已完成历史节点一句话摘要和 outcome | no raw output |
| `accepted_facts_and_decisions` | yes | semi-stable | accepted facts、decisions、satisfied criteria | evidence refs required |
| `evidence_refs` | yes | dynamic/semi-stable | output-ref/result-ref/validator-ref | no raw body |
| `dynamic_turn_delta` | yes | dynamic | 当前用户输入、最近工具摘要、必要 recovery hint | bounded chars |
| `retrieval_ref_index` | no or compact | hidden refs | 完整轨迹、原始工具输出、subagent body | ref only |
| `omission_audit` | no by default | audit | 被裁剪内容和原因 | artifact only |
| `cache_plan` | no by default | audit | hashes、token estimates、risk reasons | artifact/trace |
| `verification_manifest` | no by default | audit | exact payload proof join data | artifact/trace |

## B.4 Map snapshot 规则

`task_map_snapshot` 应该是完整逻辑结构，而不是完整字段 JSON。

必须包含：

```text
task_id
objective
success_criteria with status
current_node_id
current_path_node_ids
node_summaries ordered by path relevance
open blockers
next_best_action when known
```

节点细节密度：

| Node Class | Detail Level | Included Fields |
|---|---|---|
| current node | high | id, kind, status, title, why active, allowed actions, latest evidence |
| dependency/next node | medium | id, kind, status, summary, dependency reason |
| completed accepted node | low | id, kind, outcome, accepted claims, evidence refs |
| rejected/failed path | low | id, failure reason, avoid-repeat note |
| unrelated/stale node | ref only | id and retrieval ref |

## B.5 Protected items

`protected_items_present` 不能再靠自然语言 `- protected` 判断。Bundle 应有结构化字段：

```text
protected_items:
  - id
    kind: user_requirement | accepted_fact | accepted_decision | validator_result | output_contract
    source_ref
    evidence_refs
    visibility: visible | ref_only
    reason
```

通过条件：

```text
protected_items.length > 0 when task has user requirement or accepted facts.
Every accepted fact/decision/criterion exposed to agent has evidence_refs.
Unreviewed subagent results are not promoted to accepted protected items.
```

## B.6 Action guidance 表达规则

当前 scanner 误报风险之一是 projection 内合法出现 `taskspace_control(action=...)`。
R3-B 应避免把 action guidance 表达成容易和旧历史混淆的自然语言。

推荐结构：

```text
next_valid_actions:
  - action: state_commit
    channel: taskspace_control
    reason: record accepted findings
    required_fields: [schema_version, sections]
  - action: finish_node
    channel: taskspace_control
    reason: move from inspect to implement
```

Provider-visible rendering时可以短文本化，但 verification 不得全局 grep
`taskspace_control(` 作为污染判断。

## B.7 Omission audit

每次编译必须记录被省略、压缩、引用化的内容。

```text
omission_audit:
  raw_taskspace_control_calls:
    action: omitted
    replacement: current_node_focus.next_valid_actions
  legacy_tool_outputs:
    action: ref_only
    replacement: evidence_refs
  large_raw_outputs:
    action: output_ref
    replacement: retrieval_ref_index
  shadow_projection:
    action: omitted
    replacement: task_map_snapshot
```

Release gate 不能要求 audit 为空；它应该要求 audit 可解释，并且 raw body 不在
provider-visible payload。

## B.8 Verification manifest

`verification_manifest` 用来取代 hash-only 或 synthetic proof。

```text
verification_manifest:
  bundle_id
  provider_request_id
  provider_payload_sha256
  source_snapshot_hash
  rendered_prompt_item_hashes
  exact_payload_scan_event_id
  exact_payload_scan_matching_provider_event
  exact_context_bundle_verified
  raw_taskspace_history_tokens
  protected_items_verified
  cache_plan_verified
```

## B.9 完成证据矩阵

| Plan Item | Expected Behavior | Production Code Path | Integration Entry | Test Evidence | Runtime / Log Evidence | Mock / Stub Exposure | Status |
|---|---|---|---|---|---|---|---|
| Bundle schema | 定义结构化上下文 | context compiler module | provider request | schema fixture tests | bundle artifact | none | planned |
| Protected items | 结构化保护关键事实 | bundle renderer | compiler output | protected item tests | protected count/hash | none | planned |
| Map snapshot | 分层展示 map | bundle renderer | current task context | path fixture tests | snapshot hash | none | planned |
| Omission audit | 记录压缩/引用化原因 | compiler audit | release diagnostics | audit fixture tests | omission artifact | none | planned |
| Verification manifest | exact payload proof join | compiler/client bridge | provider request | release fixture tests | manifest artifact | none | planned |

## B.10 测试和收益验证

| Validation Type | Validation Item | Method | Passing Standard |
|---|---|---|---|
| Correctness | current node detail | fixture with active node | node kind/status/allowed actions present |
| Correctness | history summary | fixture with completed nodes | historical raw body absent, summary present |
| Correctness | protected items | fixture with facts/criteria | protected items verified |
| Correctness | unreviewed result handling | fixture with subagent result | unreviewed result not promoted |
| Benefit | scanner no false positive | payload fixture with structured next actions | legal action guidance not counted as legacy history |
| Benefit | raw history absent | exact payload scan | raw_taskspace_history_tokens=0 |
| Observability | audit completeness | artifact inspection | every omitted category has reason |

## B.11 Exit criteria

```text
TaskSpaceAgentContextBundleV1 contract documented and implemented.
Compiler emits bundle artifact and verification manifest for provider requests.
Release fixtures reject missing protected items, synthetic bundle proof, hash-only proof, and mismatched provider payload hash.
```
