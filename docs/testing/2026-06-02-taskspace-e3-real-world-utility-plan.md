# TaskSpace E3 Real-world Utility 推进计划

日期：2026-06-02

## 当前结论

TaskSpace 已经具备 E2 基础：同一题目下 `standard` 与 `taskspace` 可以用同一 prompt、fixture、model、权限、oracle 做 paired 对照，并生成可复核 artifact。

E3 不能继续用自建构造题证明。E3 的核心是：

```text
真实或外部来源任务
  -> standard/taskspace paired 对照
  -> 原始验收方式或等价 hidden oracle
  -> 多次运行
  -> artifact audit review
  -> 统计收益与失败形态
```

E3 允许的结论也必须更窄：

```text
TaskSpace 在某类真实复杂任务上出现产品收益证据。
```

不能说：

```text
TaskSpace 默认优于 standard。
TaskSpace 对所有简单任务不拖累。
TaskSpace 已经证明真实世界通用收益。
```

## 外部参考

E3 设计参考三类成熟 benchmark 思路：

- Terminal-Bench：强调 agent 在真实终端环境中完成端到端任务，并用验证脚本判定结果。官方站点将任务组织为 instruction、environment、verification 的组合，适合作为 Whale terminal-agent 对照入口。参考：https://www.tbench.ai/
- SWE-bench：从真实 GitHub issue 与对应修复 PR 构造软件工程任务，强调 repository-level issue resolution，而不是代码补全。参考论文：https://arxiv.org/abs/2310.06770
- OpenAI 对 SWE-bench Verified 的复盘：指出真实 benchmark 仍会存在测试充分性、问题质量、样本污染等局限，说明 E3 必须保留独立 artifact audit 与失败分类，不能只看 pass rate。参考：https://openai.com/index/why-we-no-longer-evaluate-swe-bench-verified/

这些参考共同约束 Whale E3：

- 保留原始用户任务叙事，不改写成 TaskSpace 友好 prompt。
- 保留外部或历史样本的原始验收方式。
- paired 对照只允许 `--taskspace` 作为 treatment delta。
- 自动 oracle 之外必须有 artifact audit review，尤其复核失败是否来自任务质量、validator 不足、模型能力，还是 TaskSpace 机制。

## Artifact Audit Review

E3 需要的不是用户亲自逐条复核，而是独立、可追溯的 artifact audit。

可接受的复核者：

- 当前 Codex 主审查者，在任务完成后基于完整 artifact 进行复核。
- 独立 reviewer agent / subagent，在不继承执行上下文的情况下读取 artifact 进行复核。
- 人类工程师，在需要产品判断或争议裁决时介入。

复核依据必须来自详实记录，而不是执行 agent 的口头自证：

- 用户原始 prompt / 脱敏 prompt。
- `whale exec` JSONL transcript。
- assistant message、tool call、tool result、错误输出。
- token / walltime / tool call 指标。
- git diff、changed paths、forbidden edit 检查。
- public validation 与 hidden/external oracle 输出。
- TaskSpace observability JSON/Markdown/HTML。
- node/result/edge/lease 等任务图记录。
- reviewer 的结构化 audit report。

因此文档和 manifest 中保留的 `human_review_required` 字段，语义上应理解为“必须存在独立 artifact audit review”。第一版沿用该字段名只是为了避免同时重命名 runner、报告和历史 artifact。

## E3 与 E2 的边界

| 维度 | E2 | E3 |
|---|---|---|
| 样本来源 | 自建构造场景 | 历史真实失败样本或外部 benchmark |
| prompt | 自写自然用户叙事 | 原始用户叙事或外部 benchmark 原始 instruction |
| oracle | 自建 hidden oracle | 原始 validator 优先，必要时补等价 hidden oracle |
| audit review | 可选 | 必须，且必须基于完整 artifact |
| repeats | >= 3 | 初始 >= 5 |
| 允许结论 | 构造任务上有 paired utility 证据 | 某类真实复杂任务上有产品收益证据 |

E3 不以“任务更难”定义，而以“样本来源更真实”定义。一个任务即使看起来简单，只要来自真实失败 corpus 或外部 benchmark，仍可进入 E3 候选。

## 样本来源策略

### 来源 A：Whale 历史真实失败 Corpus

优先级最高，因为它最贴近产品真实使用。

样本来源：

- 用户真实 session 中 Whale 失败或明显退化的任务。
- 已经发生过的 runtime/API 错误，例如 reasoning_content 回传失败、tool_call/tool_result 序列错误。
- TaskSpace 真实使用中 map 没有健康生长、viewer 不可用、命令不可用等产品缺陷。
- 真实 repo 中发生过的修复、调试、重构任务，能够脱敏并重建初始状态。

纳入条件：

- 能重建初始 repo 或最小脱敏 fixture。
- 有原始用户 prompt 或忠实脱敏后的 prompt。
- 有明确验收：测试、日志断言、错误不再出现、artifact audit 标准。
- 不包含隐私数据、密钥、私人业务信息。

不纳入：

- 只有聊天印象、无法重建执行环境的任务。
- 为了 TaskSpace 特意编造的题。
- prompt 被改写成暗示 task/map/node/subagent/parallel 的题。

### 来源 B：Terminal-Bench 小样本 Adapter

第二优先级。适合验证 Whale 作为 terminal coding agent 的真实终端执行能力。

第一阶段只做 dry-run 小样本：

- 选择 3 到 5 个任务。
- 任务必须能在 Windows 或可控容器中稳定运行。
- 优先选择 coding/file/debug 类型，暂不选择强依赖 Linux daemon、Docker-in-Docker、云服务、长时间网络任务。
- 原始 instruction 不改写。
- 原始 verification script 作为 public/external validator。

### 来源 C：SWE-bench 风格样本

第三优先级。适合观察 repository-level issue repair，但环境成本更高。

第一阶段只做 adapter 设计，不立即大规模运行：

- 选择小型 Python repo 或已知依赖可控的样本。
- 保留原始 issue text。
- 使用原始 tests 或等价 validator。
- Whale artifact 只补充 observability，不改变任务。

## E3 Manifest 扩展

E3 场景仍复用 `benchmarks/taskspace/scenarios/<id>/scenario.json`，但需要扩展字段：

```json
{
  "id": "historical-whale-reasoning-content",
  "level": "L3",
  "evidence_target": "E3",
  "sample_origin": {
    "type": "historical_whale_failure",
    "source": "sanitized_user_session",
    "source_date": "2026-06-01",
    "sanitized": true,
    "privacy_review_required": true,
    "privacy_review_completed": true,
    "sanitization_summary": "Removed private paths, user identifiers, and unrelated project names.",
    "privacy_risk_summary": "No secrets or private business data remain after sanitization.",
    "original_prompt_sha256": "..."
  },
  "external_benchmark": null,
  "human_review_required": true,
  "e3": {
    "minimum_repeats": 5,
    "manual_review_template": "docs/testing/templates/taskspace-e3-human-review.md",
    "claim_scope": "Whale runtime/API failure recovery task",
    "primary_utility_metrics": [
      "business_success_rate",
      "failure_recovery_rate",
      "unnecessary_edit_rate",
      "wall_time_distribution",
      "tool_call_distribution",
      "taskspace_graph_health"
    ]
  }
}
```

External benchmark 样本使用：

```json
{
  "sample_origin": {
    "type": "external_benchmark",
    "source": "terminal-bench",
    "source_version": "1.0",
    "sample_id": "example-id",
    "original_prompt_sha256": "...",
    "original_validator_sha256": "..."
  },
  "external_benchmark": {
    "name": "terminal-bench",
    "adapter_version": "whale-taskspace-e3-adapter-v1",
    "original_instruction_file": "original-instruction.txt",
    "validator_command": ["..."]
  }
}
```

## E3 Evidence Gate

E3 必须在 E2 gate 基础上增加门槛。

基础门槛：

- `invalid_pair = false`
- `invalid_prompt = false`
- provider/model/sandbox/timeout 等变量一致
- hidden oracle 或 external validator 隔离合格
- standard/taskspace 均保留完整 artifact
- `Repeats >= 5`

E3 额外门槛：

- `sample_origin.type` 是 `historical_whale_failure` 或 `external_benchmark`
- `sample_origin.source`、`original_prompt_sha256` 完整
- 原始 prompt/instruction checksum 记录完整
- 若 prompt 脱敏，必须记录脱敏说明与风险
- historical 样本必须 `sanitized = true`、`privacy_review_completed = true`，并记录 `sanitization_summary` 与 `privacy_risk_summary`
- external 样本必须记录 `sample_id`、`original_validator_sha256`、benchmark name 与 adapter version
- `human_review_required = true`，语义为必须存在独立 artifact audit review
- 每个 pair 必须有 audit review 记录，且 review decision 必须是可进入 aggregate 的 include 类结论
- `e3.claim_scope` 必须非空
- aggregate 必须显示 artifact audit 完成数、decision 分布与复核分歧数量；样本量足够后再派生 pass rate
- 报告必须标注 claim scope，禁止泛化

降级规则：

| 条件 | 降级 |
|---|---|
| repeats < 5 | E2-candidate 或 E3-candidate，不进入 E3 aggregate |
| 只有自建 fixture | E2，不得标 E3 |
| 没有 artifact audit review | E3-candidate |
| 缺少原始 prompt checksum、claim scope、脱敏说明、隐私复核或外部 validator metadata | E3-candidate |
| human review 没有有效 include decision | E3-candidate |
| 原始 prompt 被 TaskSpace 友好化 | invalid_prompt |
| validator 不稳定且无法解释 | excluded_pair |
| 样本含隐私且未脱敏 | excluded_pair |

当前 runner 的真实执行路径尚未接入 artifact audit report，因此即使 E3 manifest 存在，实际 run 也只能产出 `E3-candidate`。只有后续实现 audit report 读取、复核结论聚合，并满足上述全部字段后，runner 才允许报告 `E3`。

实现约束：

- `e3.minimum_repeats` 可以提高门槛，但不能把 E3 最低重复次数降到 5 以下。
- `E3-candidate` 不进入 E2 utility aggregate，也不进入 E3 aggregate。
- E3 aggregate 必须显示 artifact audit 完成数、decision 分布和复核分歧数量；pass rate 只能由这些计数派生，不作为第一版硬输出字段。

## Audit Review 模板

E3 audit review 不做主观“好不好”评分，而是回答结构化问题。复核可以由我执行，也可以由独立 reviewer agent 执行；只要复核基于完整 artifact 并写入结构化报告，就满足 E3 的复核形态。

每个 pair 复核：

```text
scenario:
pair:
reviewer:
date:

1. 样本来源是否真实或忠实脱敏？
2. prompt 是否保持用户自然叙事，没有 TaskSpace 方法论暗示？
3. standard 是否完成业务目标？
4. taskspace 是否完成业务目标？
5. 两边失败或成功的关键原因是什么？
6. TaskSpace 是否带来可观察的结构收益？
7. TaskSpace 是否引入额外误导、重复阅读或无关修改？
8. 成本增加是否在该任务复杂度下可接受？
9. 该 pair 是否可进入 utility aggregate？
10. 结论允许声明到什么范围？
```

复核结论枚举：

- `include_taskspace_better`
- `include_standard_better`
- `include_no_clear_delta`
- `exclude_harness_failure`
- `exclude_invalid_prompt`
- `exclude_validator_unclear`
- `exclude_privacy_or_sample_risk`

## Utility 指标

E3 不能只看 pass/fail。

必须统计：

- `business_success_rate`
- `hidden_or_external_validator_pass_rate`
- `manual_review_include_rate`
- `taskspace_better_rate`
- `standard_better_rate`
- `no_clear_delta_rate`
- `median_wall_time_ratio`
- `p95_wall_time_ratio`
- `median_tool_call_ratio`
- `failed_tool_call_delta`
- `unnecessary_edit_rate`
- `forbidden_edit_rate`
- `taskspace_graph_health_pass_rate`
- `observability_usefulness_notes`

对简单或混合复杂度真实样本，不再把 TaskSpace 比 standard 更慢直接作为失败。成本只作为收益判断的一部分。

## 工程实施阶段

### Phase E3-0：文档与 schema 准备

产物：

- 本文档。
- E3 manifest 字段定义。
- artifact audit review 模板。
- E3 报告降级规则。

验收：

- 不把当前 E2 matrix 误标为 E3。
- 后续 runner 修改有明确 schema。

### Phase E3-1：历史真实失败 corpus

产物：

```text
benchmarks/taskspace/corpora/historical-failures/
  README.md
  <sample-id>/
    sample.json
    original-prompt.txt
    sanitized-prompt.txt
    fixture/
    validator/
    privacy-review.md
```

首批样本建议：

- `reasoning-content-api-error`：验证 provider thinking/reasoning_content 回传兼容性修复能力。
- `tool-call-result-sequence-error`：验证 tool_call/tool_result 协议错误诊断和修复能力。
- `task-show-viewer-state-refresh`：验证 viewer 自动刷新不破坏用户 UI 状态。
- `taskspace-map-growth-missing`：验证开启 taskspace 后真实任务 map 是否健康生长。

这些都是产品真实使用中出现过的问题类型。进入 corpus 前必须脱敏并能稳定复现。

### Phase E3-2：E3 runner 最小改造

改造现有 harness，而不是新造一套：

- `scenario-manifest.ps1` 读取 `sample_origin`、`external_benchmark`、`human_review_required`、`e3`。
- `pair-report.ps1` 输出 E3 字段与 claim scope。
- 现有 `pair-report.ps1` 内的 aggregate writer 增加 E3 artifact audit 聚合；后续再清理旧的 `aggregate-report.ps1` 占位实现，避免重复入口。
- 新增 `docs/testing/templates/taskspace-e3-human-review.md`。
- 可选新增 `run-taskspace-e3-corpus.ps1`，只作为现有 runner 的批量入口。

不改：

- TaskSpace runtime。
- `whale exec` 调用方式。
- E2 runner 的核心 paired 执行逻辑。

### Phase E3-3：Terminal-Bench dry run

目标是 adapter 可行性，不追求榜单。

产物：

- 3 到 5 个任务的小样本 dry-run。
- 每个任务 paired artifacts。
- validator 稳定性记录。
- Windows/容器依赖风险记录。

如果外部环境成本过高，先不阻塞 E3；优先推进历史真实失败 corpus。

### Phase E3-4：E3 初始证据包

最低目标：

- historical corpus 至少 5 个样本。
- 每个样本 repeats >= 5。
- `deepseek-v4-flash` 先跑。
- 重点样本再补 `deepseek-v4-pro`。
- 每个 pair 有 artifact audit review。
- aggregate 给出 claim scope，不做泛化。

## 风险

| 风险 | 处理 |
|---|---|
| 历史样本难以复现 | 只纳入可重建初始状态的样本 |
| 脱敏破坏真实度 | 记录脱敏说明，必要时降级为 E2-like constructed |
| 外部 benchmark 环境成本过高 | 先做 historical corpus，不阻塞 |
| artifact audit 主观性强 | 使用固定模板、枚举结论和完整 artifact 引用 |
| TaskSpace 失败但 standard 也失败 | 记录失败形态，不直接判 TaskSpace 负收益 |
| 成本增加被误读 | 成本与成功率、误改率、可观察性一起报告 |

## 下一步

1. 新增 artifact audit review 模板。
2. 扩展 manifest parser 对 E3 字段做非破坏性读取。
3. 扩展 pair/aggregate report 输出 E3 字段，但不影响 E2。
4. 建立 `historical-failures` corpus 目录和首个样本骨架。
5. 跑 harness self-test，确认 E2 路径不受影响。
