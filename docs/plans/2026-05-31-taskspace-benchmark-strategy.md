# TaskSpace 分层 E2E 与 Benchmark 设计

日期：2026-05-31

## 背景

当前 TaskSpace 已经通过两类真实 E2E：

- 自然用户请求不暴露 `taskspace/map/node/subagent/parallel` 等内部概念。
- Whale 在真实 `whale exec --taskspace` 路径中创建任务图、派生 subagent、写回 node result、修改沙盒代码库并运行真实测试。
- E2E 不再只统计 node 数量，而是校验 edge、依赖顺序、并行调查轨道、实施与验证依赖、最终节点闭合。

这证明了第一阶段工程可行性，但还不能证明 TaskSpace 在更复杂、更脏、更长的真实开发任务中有稳定净收益。

下一阶段测试目标是：用复杂度分层 benchmark 持续施压，观察 TaskSpace 是否能在中高复杂度任务中优于标准线性模式，并确保低复杂度任务不被拖累。

## 证据等级

必须区分“机制能跑”和“产品有收益”。不同等级的测试只能支持不同强度的结论，不能越级宣传。

| 等级 | 名称 | 需要证据 | 允许结论 |
|---|---|---|---|
| E0 | Mechanism Smoke | 单次真实 `whale exec --taskspace`、真实工具、真实测试、基础 map/node/edge 观测 | 机制路径可运行 |
| E1 | Constructed Regression | 自建沙盒、多种变体、隐藏 oracle、图健康硬校验 | 机制在构造场景中稳定 |
| E2 | Paired Utility | 同题 standard/taskspace 对照、统一 oracle、成本与漏项统计、多次运行 | TaskSpace 在该类任务上出现可测净收益 |
| E3 | Real-world Utility | 历史真实失败样本或外部 benchmark、paired 对照、重复运行统计、人工复核 | TaskSpace 对真实复杂任务有产品收益证据 |

约束：

- E0/E1 不能宣称 TaskSpace 已证明真实复杂任务净收益。
- 只有达到 E2，才能说“在某类构造任务上优于 standard”。
- 只有达到 E3，才能说“对真实复杂任务有收益证据”。
- 文档、报告、发布说明必须标注当前证据等级。

## 用户叙事硬约束

TaskSpace benchmark 默认按照真实用户叙事运行。用户完全不知道 `TaskSpace`、`task map`、`node`、`subagent`、并行调度、结构化工具、图健康指标、hidden oracle、测试目标等内部概念存在。

硬约束：

- 用户 prompt 只能描述用户目标、项目症状、业务规则、验收期望、约束条件。
- 用户 prompt 不得明示或暗示 agent 采用测试目标所期待的内部行为。
- 用户 prompt 不得要求 agent 创建 task、map、node、edge、plan graph。
- 用户 prompt 不得要求 agent 并行、拆给多个 agent、调用 subagent、委派、fan out。
- 用户 prompt 不得暗示“为了测试 TaskSpace 效果”“为了让图生长”“为了验证多 agent 协作”等 benchmark 意图。
- 用户 prompt 不得把内部观测指标转写成自然语言要求，例如“请产生多个独立调查轨道”“请确保实施依赖调查结果”。
- benchmark 可以在 manifest 中声明 TaskSpace 结构期望，但这些期望只属于 harness/oracle，不得进入 agent 可见输入。

允许的用户叙事：

- “先理解 README、测试和实现，再修改。”
- “区分产品规则和错误测试预期。”
- “不要只修表面问题。”
- “修改前后都跑测试。”
- “说明你做了什么，以及为什么这么改。”

这些表达是用户对工作质量的自然要求，不是对内部协作机制的指令。

判断原则：

```text
如果一个真实用户不会自然这么说，
或者这句话只有知道 TaskSpace 内部设计/测试目标的人才会说，
则该 prompt 不合格。
```

违反该约束的 run 只能用于内部机制调试，不能计入 E1/E2/E3，也不能用于产品收益判断。

## 产品假设

TaskSpace 的定位不是替代所有简单执行路径，而是面向中高复杂度问题解决的非线性工作组织层。

核心假设：

```text
低复杂度任务：
  TaskSpace 不应明显慢于 standard，不强制复杂图，不强制 subagent。

中复杂度任务：
  TaskSpace 应减少漏项、误改和上下文混杂。

高复杂度任务：
  TaskSpace 应形成健康任务图，让主 agent 能调度调查、整合、实施、验证。

长程复杂度任务：
  TaskSpace 应在多轮追问、插话、目标变化、上下文增长下维持 task 边界和工作结构。
```

当前优先级：

- 可观察：用户和开发者能看到任务如何推进。
- 可检查：E2E 能判断图结构和执行顺序是否健康。
- 暂不优先做可恢复/时空回溯。相关能力未来可在 viewer、历史 map、reborn 链路上发展，但不应牵引第一阶段复杂度。

## 分层测试矩阵

### L1 低复杂度任务

目标：证明 TaskSpace 不拖累简单任务。

典型场景：

- 单文件小 bug 修复。
- 简单测试断言修正。
- 小型文案或配置变更。
- 明确错误信息定位到单个函数。

用户输入要求：

- 像真实用户一样描述问题，不出现内部概念。
- 不要求拆分、并行、subagent、task、node。
- 验收条件清晰，例如“修完后跑这个测试”。

期望 TaskSpace 行为：

```text
task -> 1 到 3 个 node -> validation -> final
```

不要求：

- subagent。
- 多个并行 inspect node。
- 复杂 edge 网络。

硬指标：

| 指标 | 期望 |
|---|---|
| task created | true |
| ordinary_before_binding | false |
| completed_nodes | >= 1 |
| open_leaf_nodes | 0 |
| validation_passed | true |
| edit_owned_by_implementation_node | true |
| unexpected_taskspace_gate_failures | 0 |

软指标：

| 指标 | 观察目标 |
|---|---|
| wall_time_ratio_vs_standard | 初始阈值 <= 1.5，超过则标记成本劣化 |
| tool_call_ratio_vs_standard | 初始阈值 <= 1.5，超过则标记成本劣化 |
| token_ratio_vs_standard | 可采集时初始阈值 <= 1.5 |
| node_count | 不追求多，过多反而是负面信号 |
| subagent_count | 通常为 0 |

失败信号：

- 简单任务被拆成大量 node。
- 没有必要却 spawn subagent。
- 为维护图而重复读取大量文件。
- standard 一步能完成，TaskSpace 绕很久才完成。

### L2 中复杂度任务

目标：证明 TaskSpace 能处理多文件关联问题，并比线性模式更少漏项。

典型场景：

- 多文件关联 bug，有明确测试。
- README、测试、实现之间有轻微冲突。
- 一个行为由 parser、service、formatter、tests 共同决定。
- 前端组件、状态管理、接口 mock 三处共同影响一个问题。

用户输入要求：

- 正常描述项目症状和验收。
- 可以说“先理解再改”“区分业务规则和测试预期”，但不能提示内部协作策略。

期望 TaskSpace 行为：

```text
boundary/事实源识别
  -> 2 到 4 个独立 inspect node
  -> synthesis 或 baseline validation
  -> implementation
  -> regression/smoke
  -> final
```

硬指标：

| 指标 | 期望 |
|---|---|
| edge_count | >= 关键节点数 - 1 |
| edge_order_violations | 0 |
| key_edges_have_reason | true |
| implementation_consumes_upstream_result | true |
| implementation_has_incoming_edge | true |
| test_depends_on_implementation | true |
| direct_test_depends_on_implementation | true |
| nodes_with_results | 与 completed_nodes 接近 |
| validation_passed | true |
| unexpected_failed_collab_tool_calls | 0 |

按场景启用的硬指标：

| 条件 | 额外要求 |
|---|---|
| 存在多个独立事实面 | 至少 2 个 inspect track |
| 使用 subagent | subagent result 必须写回 node |
| README 与测试冲突 | 最终 diff 必须体现对产品真相的选择 |

软指标：

- 是否把事实源、实施、验证拆成不同节点。
- 是否存在明确的调查结果整合。
- 是否避免把所有读取塞进一个超大 inspect node。
- 是否避免多个 subagent 读取高度重叠文件。

### L3 高复杂度任务

目标：证明 TaskSpace 在混乱信息环境下仍能维持结构。

典型场景：

- 架构质量分析并提出治理方案。
- 跨模块重构，涉及接口、测试、日志、文档。
- Debug 场景中日志、复现、代码路径、配置互相矛盾。
- README 过时，测试缺失，代码行为隐式依赖历史约定。
- 多种修复路径，需要先比较方案再实施。

用户输入要求：

- 可以表达复杂目标，例如“检查架构质量并优化”“定位这个跨模块问题”“不要只修表面”。
- 不出现 task/node/map/subagent/parallel/delegate。
- 不直接告诉 agent 怎样拆任务。

期望 TaskSpace 行为：

```text
scope/boundary
  -> evidence tracks
  -> risk tracks
  -> synthesis/decision
  -> staged implementation
  -> smoke/regression/review
  -> final
```

硬指标：

| 指标 | 期望 |
|---|---|
| has_boundary_node | true |
| parallel_inspect_tracks | >= 2 |
| parallel_inspect_tracks_independent | true |
| key_edges_have_reason | true |
| implementation_consumes_upstream_result | true |
| implementation_depends_on_parallel_inspect_tracks | true |
| direct_implementation_depends_on_parallel_inspect_tracks | true |
| validation_node_has_real_command | true |
| edit_outside_implementation | 0 |
| edge_order_violations | 0 |
| open_leaf_nodes | 0 |
| open_final_synthesis_nodes | 0 |

效用型指标：

- 与 standard 模式对照时，是否少漏关键文件。
- 是否能识别错误测试、错误 README、错误实现中的至少一种冲突。
- 是否能解释采用方案和放弃方案。
- 是否减少无依据修改。
- 是否更容易从 viewer 看出当前卡点和下一步。

失败信号：

- 只有一个巨大 inspect node。
- 边退化成顺序流水账，无法表达真实依赖。
- subagent 都扫同一批文件，没有独立事实面。
- 主 agent 不读取 node result，直接线性重扫和修改。
- 实施节点没有依赖调查节点。

### L4 长程复杂度任务

目标：验证 TaskSpace 在多轮会话和上下文增长中是否保持 task 边界。

典型场景：

- 用户先要求架构检查，中途插入小 bug，再回到架构任务。
- 用户修改目标，从“修 bug”变成“先设计方案，不动代码”。
- 用户否定上一轮方案，要求换思路。
- 上下文压缩后继续同一个 task。
- 同一 session 中存在多个 active/pending task。

用户输入要求：

- 多轮自然对话。
- 用户不理解 TaskSpace 内部概念。
- 用户可能含糊、插话、改变优先级。

期望 TaskSpace 行为：

```text
turn 1: 建立 task A
turn 2: 更新 task A map
turn 3: 识别插话为 task B 或 task A 的子问题
turn 4: 回到 task A，不污染 task B
turn 5: 压缩后继续，task/map/node/result 结构仍可用
```

硬指标：

| 指标 | 期望 |
|---|---|
| task_routing_required | true |
| no_ordinary_before_task_binding | true |
| task_count | 能随用户主题自然增长 |
| active_task_switch_logged | true |
| node_results_preserved_after_compaction | true |
| current_binding_valid_after_compaction | true |
| no_cross_task_result_pollution | true |

暂缓指标：

- 完整时空回溯。
- 任意历史节点重新执行。
- 自动 reborn。

这些属于未来可恢复能力，不作为当前 benchmark 的第一优先级。

## 两类指标

### 工程可行性指标

这类指标回答“机制有没有按设计运行”。

| 类别 | 指标 |
|---|---|
| 启动 | taskspace_enabled、viewer_url、task created |
| 绑定 | ordinary_before_binding、current node lease |
| 图结构 | node_count、edge_count、edge_order_violations |
| 节点生命周期 | completed_nodes、open_leaf_nodes、open_final_synthesis_nodes |
| subagent | spawn_agent_calls、subagent_results、unexpected_failed_collab_tool_calls |
| 工具归属 | edit_owned_by_implementation、test_owned_by_validation |
| 验证 | pytest/command exit code、hidden oracle、git diff |
| 稳定性 | crash events、timeout、invalid_request_error |

工程指标用于硬失败判断。只要工程指标失败，就不能声称 TaskSpace 机制健康。

### 效用指标

这类指标回答“TaskSpace 是否比标准线性模式更有价值”。

| 类别 | 指标 |
|---|---|
| 漏项 | 是否检查到关键文件、关键事实源、关键测试 |
| 误改 | 是否修改了无关文件、是否为了错误测试扭曲产品规则 |
| 幻觉 | 是否引用不存在的行为、文件、命令结果 |
| 调度 | 是否主动拆分独立事实面、是否利用 node result |
| 成本 | wall time、tool calls、token、subagent count |
| 可观察 | viewer 是否能解释当前状态、依赖、卡点 |
| 对照收益 | 与 standard 同题对照的成功率、漏项率、耗时、误改率 |

效用指标不应全部变成硬门槛。复杂任务存在自然不确定性，benchmark 需要长期积累统计，而不是单次运行绝对判断。

## Standard 对照策略

从 E2 开始，每个 benchmark 场景必须有两条运行路径：

```text
standard:
  whale exec ...

taskspace:
  whale exec --taskspace ...
```

对照时不要要求 TaskSpace 在所有维度胜出。合理目标是：

- L1：TaskSpace 不明显更差。
- L2：TaskSpace 在漏项和误改上更稳。
- L3：TaskSpace 在结构化调查、验证闭环、可观察性上明显更好。
- L4：TaskSpace 在多主题和压缩后结构保留上优于 standard。

对照报告至少包含：

- 两种模式的最终业务验证结果。
- 两种模式的 git diff。
- 两种模式的命令执行记录。
- TaskSpace 的 observability artifact。
- standard 的线性 transcript 摘要。
- 统一 oracle 输出：关键文件覆盖、禁止修改文件、产品真相选择、隐藏测试、漏项分类。
- 失败分类：工程失败、业务失败、调度失败、成本劣化、观测不足。

对照运行策略：

- 每个 E2 场景初始至少重复 3 次，避免单次模型随机性误判。
- 每个 E3 场景初始至少重复 5 次，并保留人工复核摘要。
- 统计时分开记录 pass rate、business success、graph health、cost、leak/mis-edit。
- 如果 standard 和 taskspace 都失败，不能算 TaskSpace 负收益；应归入任务难度或模型能力失败，再看失败形态是否不同。
- 如果 TaskSpace 成功但成本超过 L1 阈值，低复杂度场景仍判成本劣化。

## 工程实施架构

Benchmark 不应是多个互相独立的临时脚本，而应收敛为一套可扩展 harness。

建议目录：

```text
scripts/
  taskspace-benchmark/
    run-taskspace-benchmark.ps1
    run-scenario-pair.ps1
    collect-run-artifacts.ps1
    compare-paired-runs.ps1
    export-benchmark-summary.ps1
    lib/
      scenario-manifest.ps1
      oracle-runner.ps1
      metrics-extractor.ps1
      variable-control.ps1
      failure-classifier.ps1

benchmarks/
  taskspace/
    scenarios/
      single-file-fast-fix/
        scenario.json
        prompt.txt
        fixture/
        public/
        private-oracle/
      order-pipeline-growth/
        scenario.json
        prompt.txt
        fixture/
        oracle/
    corpora/
      historical-failures/
      terminal-bench-adapter/

target/
  taskspace-benchmark/
    <scenario-id>/
      <run-id>/
        standard/
        taskspace/
        pair-report.md
      aggregate-report.md
```

现有 `scripts/run-action-map-*.ps1` 可以继续作为 E1 回归脚本，但 E2/E3 应逐步迁移到统一 harness，避免每个场景重复实现 prompt guard、artifact 收集、oracle、指标抽取和失败归因。

### Scenario Manifest

每个场景必须有 `scenario.json`，作为测试契约。

示例：

```json
{
  "id": "order-pipeline-growth",
  "level": "L2",
  "evidence_target": "E2",
  "description": "README/tests/implementation conflict with multi-file behavior.",
  "prompt_file": "prompt.txt",
  "fixture_dir": "fixture",
  "mode_delta_contract": {
    "allowed_deltas": [
      "taskspace_flag",
      "taskspace_runtime_system_behavior",
      "taskspace_structural_tools",
      "taskspace_observability_export"
    ],
    "forbidden_agent_visible_deltas": [
      "different_prompt",
      "different_fixture",
      "different_model_params",
      "different_permissions",
      "different_output_budget",
      "different_agent_readable_path_label",
      "hidden_oracle_visibility"
    ]
  },
  "narrative_contract": {
    "user_knows_internal_concepts": false,
    "forbid_internal_mechanism_hints": true,
    "forbid_benchmark_goal_hints": true,
    "prompt_guard_required": true,
    "guard_output_required": true,
    "external_original_prompt": false,
    "manual_review_required_for_contextual_hits": true
  },
  "turns": [
    {
      "id": "turn-1",
      "prompt_file": "prompt.txt",
      "expect_task": "primary"
    }
  ],
  "compaction_trigger": null,
  "resume_assertions": [],
  "external_benchmark": null,
  "resource_policy": {
    "network": "disabled",
    "timeout_seconds": 1200
  },
  "human_review_required": false,
  "oracle": {
    "type": "python",
    "entry": "private-oracle/hidden_oracle.py",
    "public_validation": ["python", "-m", "pytest", "tests", "-q"]
  },
  "expected": {
    "allowed_modified_paths": [
      "src/order_pipeline/parser.py",
      "src/order_pipeline/pricing.py",
      "tests/test_invoice.py"
    ],
    "forbidden_modified_paths": [
      "README.md",
      "pyproject.toml"
    ],
    "required_commands": [
      "python -m pytest tests -q"
    ],
    "required_key_files_read": [
      "README.md",
      "tests/test_parser.py",
      "tests/test_pricing.py",
      "tests/test_invoice.py"
    ]
  },
  "thresholds": {
    "repeats": 3,
    "timeout_seconds": 1200,
    "l1_cost_ratio_limit": 1.5,
    "max_unexpected_tool_failures": 0
  },
  "taskspace_expectations": {
    "min_nodes": 4,
    "min_edges": 3,
    "requires_edge_reason": true,
    "requires_result_consumption": true,
    "requires_validation_node": true
  }
}
```

原则：

- prompt 是用户输入，不写内部流程指令。
- manifest 是测试契约，可以包含 TaskSpace 期望。
- fixture 是初始代码库，不在运行中动态生成核心业务文件，除非场景本身测试脚手架生成能力。
- private oracle 隐藏在测试 harness 中，不暴露给 agent。
- `narrative_contract` 是硬门禁。任何 prompt 泄漏内部概念、协作机制或 benchmark 目标，该 run 标记为 `invalid_prompt`。
- L1/L2/L3 单轮场景可以只有一个 `turns[]`。
- L4 必须使用 `turns[]`、`compaction_trigger` 和 `resume_assertions`。
- E3 外部 benchmark 必须填写 `external_benchmark`、`resource_policy` 和 `human_review_required`。
- E3 外部 benchmark 的原始 prompt 如果包含通用工程词，不自动判 invalid；只有暴露 TaskSpace 内部机制、被改写成 TaskSpace 友好 prompt、或额外添加内部方法论提示时才判 invalid。

### Run ID 与 Artifact

每次 paired run 生成一个稳定 `run_id`：

```text
<scenario-id>-<yyyyMMdd-HHmmss>-<short-random>
```

目录：

```text
target/taskspace-benchmark/<scenario>/<run_id>/
  manifest.resolved.json
  prompt.txt
  logical-mode-map.json
  left/
    repo/
    artifacts/
      whale-exec.jsonl
      whale-exec.stderr.log
      last-message.md
      git-diff.patch
      public-validation.stdout.log
      public-validation.stderr.log
      hidden-oracle.stdout.log
      hidden-oracle.stderr.log
      metrics.json
  right/
    repo/
    artifacts/
      whale-exec.jsonl
      whale-exec.stderr.log
      last-message.md
      git-diff.patch
      public-validation.stdout.log
      public-validation.stderr.log
      hidden-oracle.stdout.log
      hidden-oracle.stderr.log
      observability/
        action-map-observability.json
        action-map-observability.md
        action-map-observability.html
      metrics.json
  reviewer-only/
    private-oracle/
    original-external-metadata/
  pair-report.md
```

`manifest.resolved.json` 必须写入实际 whale path、sha256、model、provider config 摘要、环境变量白名单、fixture checksum、prompt checksum，保证后续能追溯同一对照是否真的同源。

`left` 与 `right` 是 agent 可见路径中的唯一 mode 区分。它们不能叫 `standard`、`taskspace`、`control`、`treatment` 等会泄漏实验条件的名字。

`logical-mode-map.json` 只给 harness 和 reviewer 使用：

```json
{
  "left": "standard",
  "right": "taskspace",
  "run_order": ["left", "right"]
}
```

## Paired Run 协议

E2/E3 的基本单元不是一次 run，而是一对 run：

```text
same fixture
same prompt
same model
same model parameters
same whale binary
same cwd shape
same env policy
same permissions
same timeout
different mode only:
  standard logical mode: whale exec ...
  taskspace logical mode: whale exec --taskspace ...
```

执行步骤：

1. 解析 `scenario.json`。
2. 复制 fixture 两份：`left/repo` 与 `right/repo`。
3. 分别初始化 git，确保初始 commit、文件内容和 checksum 一致。
4. 写入相同 `prompt.txt`。
5. 运行 prompt guard，确认用户输入不含内部协作概念。
6. 按本次 repeat 的 logical-mode-map 运行 left/right。
7. 另一侧使用相同命令参数，除 logical mode treatment 外不得改变。
8. 对两边分别执行 public validation。
9. 对两边分别执行 hidden oracle。
10. 收集 JSONL、stderr、last message、git diff、命令记录、退出码。
11. taskspace 额外导出 observability。
12. 抽取 metrics。
13. 生成 pair-report。
14. 聚合多次 repeats。

运行顺序控制：

- 默认 logical mapping 为 left=standard、right=taskspace，但每个 scenario 的 repeats 中应交替 mapping 和运行顺序：
  - odd repeat: left=standard, right=taskspace, run left -> right
  - even repeat: left=taskspace, right=standard, run left -> right
- 这样可以降低 provider 状态、缓存、机器负载、网络波动造成的顺序偏差。
- 如果某一侧发生 harness failure，可以重跑该 pair；如果是 agent failure，不自动重跑并覆盖结果。

### Mode Delta Contract

paired run 的 treatment delta 只能是：

- 是否传入 `--taskspace`。
- TaskSpace runtime 因该 flag 注入的系统行为。
- TaskSpace 结构化工具可用性。
- TaskSpace observability 导出。

禁止差异：

- 用户 prompt 不同。
- fixture 不同。
- 模型名称、temperature、top_p、max output、reasoning effort、service tier、tool set、system prompt baseline 版本不同，除非差异是 `--taskspace` treatment 的必然产物并记录。
- permission/sandbox 不同。
- timeout 不同。
- agent 可见路径名泄漏实验条件。
- hidden oracle 或 reviewer-only artifact 对 agent 可见。

`manifest.resolved.json` 必须逐项记录：

```json
{
  "model": {
    "name": "deepseek-v4-flash",
    "temperature": "provider-default-or-explicit",
    "top_p": "provider-default-or-explicit",
    "max_output_tokens": "provider-default-or-explicit",
    "reasoning_effort": "provider-default-or-explicit",
    "service_tier": "provider-default-or-explicit"
  },
  "tool_policy": {
    "tool_set_hash": "...",
    "permissions": "...",
    "sandbox": "..."
  },
  "prompt_sha256": "...",
  "fixture_sha256": "...",
  "cwd_policy": "neutral-left-right",
  "allowed_mode_delta": "taskspace_flag"
}
```

任何字段不一致且不在 allowed delta 内，pair 标记为 `invalid_pair`，不得进入 E2/E3 utility aggregate。

## 变量控制

### 必须固定的变量

| 变量 | 控制方式 |
|---|---|
| 初始代码库 | 同一 fixture checksum，复制为两份工作区 |
| 用户 prompt | 同一 `prompt.txt` checksum |
| 模型 | manifest 固定 model，例如 `deepseek-v4-flash` |
| 模型参数 | name、temperature、top_p、max output、reasoning effort、service tier 逐项记录和比较 |
| Whale binary | 记录 path、version、sha256 |
| 权限模式 | 两边使用相同 permission/sandbox 配置 |
| 工作目录形态 | 两边 repo 相同相对路径结构，agent 可见路径使用 `left/right` 中性名 |
| 环境变量 | 使用白名单传入，记录摘要 |
| timeout | manifest 固定 |
| public validation | 同一命令 |
| hidden oracle | 同一 oracle |
| 外部网络 | 默认禁用或同策略；需要网络的场景单独标记 |

### 不可完全固定的变量

| 变量 | 处理方式 |
|---|---|
| 模型采样随机性 | repeats + paired aggregate |
| provider 延迟/缓存 | 交替运行顺序，记录 wall time 但不单独作为质量指标 |
| 本机负载 | 记录 started/finished、可选记录 CPU/内存摘要 |
| tool 输出中的时间戳 | oracle 不依赖时间戳；diff 可归一化 |
| 文件系统顺序 | scenario 中避免依赖目录枚举自然顺序 |

### 禁止的变量污染

- taskspace prompt 比 standard prompt 多提示。
- taskspace 场景暴露内部 node/subagent/map 词。
- 用户 prompt 暗示 benchmark 希望看到的内部行为，例如并行调查、多个 agent、任务图生长、节点依赖。
- 用户 prompt 提到测试目标、图健康、TaskSpace 效用验证、结构化协作指标。
- standard 用不同模型或不同权限。
- taskspace 允许读 hidden oracle。
- agent 可见路径包含 `standard` 或 `taskspace`。
- hidden oracle、reviewer-only metadata、external benchmark answer key 被复制进 agent repo。
- 运行后修改 fixture 再跑另一侧。
- 对失败的一侧手动补跑并覆盖原始 artifact。
- 只展示 taskspace 成功样本，不展示 paired standard 结果。

## Oracle 设计

Oracle 分三层：

### Public Validation

agent 被允许看到或主动运行的验证，例如：

```text
python -m pytest tests -q
cargo test -p ...
npm test
```

用途：

- 验证 agent 是否按用户要求完成显性测试。
- 检查测试命令是否归属 validation node。

局限：

- public tests 可能有错误预期。
- public tests 可能覆盖不足。

### Hidden Oracle

agent 不可见的业务验收，用于检查真实目标。

类型：

- 隐藏测试脚本。
- 静态 diff oracle。
- 行为 probe。
- 文件修改 allowlist/denylist。
- 关键事实覆盖检查。

Hidden oracle 不应包含 TaskSpace 内部结构期望，只检查业务正确性和安全边界。

Hidden oracle filesystem contract：

```text
benchmarks/taskspace/scenarios/<scenario>/private-oracle/
  hidden_oracle.py

target/taskspace-benchmark/<scenario>/<run_id>/reviewer-only/private-oracle/
  hidden_oracle.py

target/taskspace-benchmark/<scenario>/<run_id>/left/repo/
target/taskspace-benchmark/<scenario>/<run_id>/right/repo/
```

规则：

- agent cwd 只能是 `left/repo` 或 `right/repo`。
- private oracle 不复制进 agent repo。
- public validation 可以存在于 fixture 内，hidden oracle 不可以。
- harness 在 agent 结束后，从 repo 外执行 hidden oracle。
- `reviewer-only/private-oracle` 只能给报告审查使用，不进入 agent 可读路径。
- 如果 sandbox 暂时无法硬隔离父目录，harness 必须至少做路径 denylist 检查，并在 report 中标记 oracle isolation 等级。
- `hidden_oracle.py` 不再写入 agent artifacts；只写 stdout/stderr、exit code、oracle sha256。

### TaskSpace Structural Oracle

只用于 taskspace 路径，检查机制健康：

- task/map/node 是否创建。
- 普通工具是否在 binding 后执行。
- edge 是否存在、顺序是否正确。
- implementation 是否依赖上游调查。
- validation 是否依赖 implementation。
- subagent result 是否写回 node。
- key edge 是否有 reason。
- implementation 是否消费上游 result。
- open leaf/final 是否闭合。

Structural oracle 不判断业务语义对错。业务对错由 public validation 和 hidden oracle 判断。

## 指标抽取

每侧生成 `metrics.json`。

通用字段：

```json
{
  "scenario_id": "order-pipeline-growth",
  "mode": "standard",
  "evidence_level": "E2",
  "exec_exit_code": 0,
  "public_validation_exit_code": 0,
  "hidden_oracle_exit_code": 0,
  "wall_time_ms": 73000,
  "tool_call_count": 22,
  "command_count": 4,
  "changed_paths": [],
  "allowed_modified_paths_ok": true,
  "forbidden_modified_paths_touched": [],
  "evidence": {
    "required_key_files_read": {
      "README.md": ["whale-exec.jsonl:event-123"]
    },
    "required_commands": {
      "python -m pytest tests -q": ["whale-exec.jsonl:event-456"]
    },
    "forbidden_modified_paths": {},
    "business_success": ["hidden-oracle.stdout.log"]
  },
  "business_success": true,
  "harness_failure": false,
  "agent_failure": false
}
```

TaskSpace 额外字段：

```json
{
  "taskspace_enabled": true,
  "maps": 1,
  "nodes": 10,
  "edges": 17,
  "edge_order_violations": 0,
  "key_edge_reason_coverage": 0.9,
  "result_consumption_coverage": 0.8,
  "source_overlap_smells": 1,
  "spawn_agent_calls": 4,
  "subagent_results": 21,
  "open_leaf_nodes": 0,
  "open_final_synthesis_nodes": 0,
  "ordinary_before_binding": false,
  "edit_outside_implementation": 0,
  "unexpected_gate_failures": 0,
  "taskspace_evidence": {
    "key_edge_reasons": ["action-map-observability.json:edges[0]"],
    "result_consumption": ["action-map-observability.json:events[42]"],
    "source_overlap_smells": ["pair-report.md#source-overlap"]
  }
}
```

注意：

- token 字段能采集则采集，不能采集时写 `unknown`，不要估算。
- cost ratio 只在两边字段同源时计算。
- graph health 只用于 taskspace，不用来评价 standard。

## Pair Report

`pair-report.md` 面向工程审查，不是营销材料。

结构：

```text
# TaskSpace Benchmark Pair Report

## Scenario
- id
- level
- evidence target
- prompt checksum
- fixture checksum
- whale sha256
- model

## Outcome
- standard business success
- taskspace business success
- public validation
- hidden oracle
- cost ratio
- failure classification

## Variable Control
- same prompt: yes/no
- same fixture: yes/no
- same model: yes/no
- same permissions: yes/no
- run order

## TaskSpace Structural Health
- maps/nodes/edges
- edge order
- key edge reason coverage
- result consumption coverage
- source overlap smells
- open leaf/final

## Diff Comparison
- standard changed paths
- taskspace changed paths
- forbidden paths
- behavioral diff notes

## Failure Analysis
- harness failure
- runtime failure
- decomposition failure
- business failure
- cost regression
- observability failure

## Artifacts
- standard paths
- taskspace paths
```

如果变量控制失败，例如 prompt checksum 不一致，pair report 必须标红为 invalid pair，不得进入 E2/E3 聚合。

## 聚合与统计

`aggregate-report.md` 按 scenario 汇总多次 repeats。

进入 utility aggregate 的 pair 必须满足：

- `invalid_pair = false`
- `invalid_prompt = false`
- `harness_failure = false`
- `prompt/fixture/model/permission/cwd` variable checks 全部通过
- hidden oracle isolation 等级满足场景要求

不进入 utility aggregate 但仍保留记录：

- invalid pair
- invalid prompt
- harness failure
- oracle isolation failure
- variable-control self-test failure

核心表：

| 指标 | standard | taskspace | delta |
|---|---:|---:|---:|
| business success rate | | | |
| public validation pass rate | | | |
| hidden oracle pass rate | | | |
| forbidden edit rate | | | |
| median wall time | | | |
| median tool calls | | | |
| taskspace graph health pass rate | n/a | | |

判读原则：

- 样本数少时只做趋势判断，不做强统计显著性声明。
- L1 重点看成本劣化和成功率不下降。
- L2/L3 重点看 hidden oracle、漏项、误改和结构健康。
- E3 必须保留人工复核摘要，尤其是“失败是否可归因于任务难度而非 TaskSpace”。
- aggregate 必须同时展示 `all pairs`、`valid utility pairs`、`excluded pairs`，防止隐藏基础设施失败。

## Flake 与重跑规则

只允许重跑 harness failure：

- fixture 复制失败。
- oracle 脚本自身崩溃。
- whale binary 不存在。
- 环境依赖缺失。
- artifact 写入失败。

不允许自动重跑并覆盖的 agent failure：

- agent 没完成任务。
- agent 改错文件。
- agent 没跑测试。
- taskspace 图退化。
- provider 返回有效错误。
- 超时但已有 agent 行动轨迹。

重跑必须生成新 run id，并在 aggregate 中保留原失败记录。

重跑 lineage：

```json
{
  "run_id": "order-pipeline-growth-20260531-010000-a1b2",
  "rerun_of": "order-pipeline-growth-20260531-005500-z9y8",
  "rerun_reason": "harness_failure: oracle dependency missing",
  "included_in_utility_aggregate": true
}
```

原始 harness failure 的 `included_in_utility_aggregate` 为 false，但必须进入 `excluded pairs` 表。

## 外部 Benchmark 适配

Terminal-Bench/SWE-bench 类任务只做薄封装：

```text
external task prompt
  -> standard runner
  -> taskspace runner
  -> original benchmark validator
  -> Whale extra observability
```

禁止：

- 改写原始任务为 TaskSpace 友好 prompt。
- 给 taskspace 额外提示内部方法论。
- 删除对 TaskSpace 不利的样本。

允许：

- 记录 TaskSpace observability。
- 增加 Whale-specific artifact 收集。
- 对任务做安全隔离和资源限制。

外部 benchmark 接入前应先跑小样本 dry run，确认：

- sandbox 能运行。
- validator 可重复。
- artifact 不含隐私数据。
- standard/taskspace 使用同一 prompt 和初始状态。

E3 manifest 必须补充：

```json
{
  "external_benchmark": {
    "name": "terminal-bench",
    "sample_id": "sample-001",
    "original_prompt_sha256": "...",
    "original_validator_sha256": "...",
    "adapter_version": "whale-taskspace-adapter-v1"
  },
  "validator": {
    "type": "external",
    "command": ["..."],
    "container": "optional-container-id"
  },
  "resource_policy": {
    "network": "disabled|enabled-with-allowlist",
    "cpu_limit": "optional",
    "memory_limit": "optional",
    "timeout_seconds": 1800
  },
  "privacy_scrub": {
    "enabled": true,
    "rules_sha256": "..."
  },
  "human_review_required": true
}
```

## Benchmark 场景库

第一阶段自建场景：

| 场景 | 层级 | 目的 |
|---|---|---|
| single-file-fast-fix | L1 | 验证低复杂度不拖累 |
| config-test-fix | L1 | 验证小范围配置/测试修复 |
| order-pipeline-natural | L2 | README、测试、实现冲突 |
| order-pipeline-growth | L2/L3 | 多 inspect track、subagent、edge health |
| logging-regression-debug | L3 | 日志、复现、代码路径三源调查 |
| architecture-quality-review | L3 | 架构检查、治理建议、部分落地 |
| interrupted-session | L4 | 用户插话和回到旧任务 |
| compaction-continue | L4 | 压缩后继续 task |

第二阶段引入外部 benchmark：

- Terminal-Bench：适合观察真实终端任务、长命令链、环境问题和任务完成率。
- SWE-bench 风格任务：适合观察修复正确性、测试闭环和跨文件定位。
- 自定义 Whale regression corpus：保存历史真实失败样本，避免只测人工构造的理想场景。

外部 benchmark 适配原则：

- 不把 benchmark prompt 改写成 TaskSpace 指令。
- 不泄露内部 map/node/subagent 概念。
- 保留原始验收方式。
- TaskSpace 额外导出 observability artifact。
- 同一题尽量跑 standard 与 taskspace 对照。
- 原始 prompt 中的通用工程词不因黑名单命中自动 invalid；必须结合语境判断是否泄漏 Whale/TaskSpace 内部机制。
- 如果为了适配 Whale 而补充提示，只能补充运行环境/安全/验收路径，不得补充内部方法论、并行、subagent、node、graph 等协作策略。

实施顺序约束：

- E1 自建场景用于保护机制回归。
- E2 paired 对照不应长期后置；L1/L2 自建场景稳定后立刻补 standard/taskspace 对照。
- E3 外部 benchmark 和历史真实失败 corpus 是收益声明前置条件，不是锦上添花。

## Prompt Guard

Prompt Guard 是用户叙事硬约束的自动化检查层，不是全部判断。它负责抓显性泄漏；人工审查和 scenario review 负责抓隐性暗示。

Prompt Guard 分两类命中。

### Hard Internal Tokens

这些词默认 hard fail，因为普通用户不应在自然请求中要求 Whale 使用这些内部机制：

```text
taskspace
action map
subagent
spawn_agent
taskspace_control
multiple agents
multi-agent
split ... agents
fan out
graph health
structured collaboration
coordination strategy
test objective
observability target
```

命中 hard internal token 时，run 标记为 `invalid_prompt = true`，除非该 prompt 是外部 benchmark 原始文本且人工复核确认该词不是 Whale/TaskSpace 内部机制含义。

### Context-Sensitive Terms

这些词可能是普通工程语义，不能一刀切 hard fail：

```text
map
node
parallel
parallelize
concurrent
simultaneously
delegate
delegation
task graph
dependency graph
benchmark
```

处理规则：

- 如果上下文是在说 Whale/agent 的内部组织方式，例如“让多个 agent 并行调查”“创建 node”“生成 task graph”，判 invalid。
- 如果上下文是业务或工程对象，例如 Node.js、source map、map parsing、parallel tests、concurrent request race、dependency graph of packages、performance benchmark，允许或进入人工复核。
- 外部 benchmark 原始 prompt 中出现 context-sensitive terms，不自动 invalid。只有 harness 改写 prompt、额外添加内部方法论提示、或暴露 TaskSpace 结构目标时才 invalid。

允许出现：

- “先理解再修改”
- “区分产品规则和测试预期”
- “不要只修表面”
- “跑测试验证”
- “说明你怎么组织工作”

这些是普通用户可自然表达的工作要求，不属于内部协作机制泄漏。

隐性暗示同样禁止，例如：

- “你可以同时从多个方向调查。”
- “你可以安排其他 agent 帮你看不同模块。”
- “请把工作拆成多个节点推进。”
- “请让你的任务图体现前后依赖。”
- “这次我要观察你是否会主动并行。”

这些句子即使不包含被禁关键词，也会污染真实用户路径。

Prompt Guard 输出必须记录：

```json
{
  "invalid_prompt": false,
  "guard_hits": [
    {
      "matched_span": "parallel tests",
      "hit_category": "context_sensitive",
      "decision": "allowed",
      "manual_review_required": false,
      "external_original_prompt": false,
      "false_positive_allowed_reason": "parallel describes test execution, not agent orchestration"
    }
  ]
}
```

如果出现 context-sensitive hit 且无法自动判断语义，必须设置 `manual_review_required = true`。人工复核结论必须写入 pair report，不能只在终端输出。

## 报告格式

每个 E2E run 生成：

```text
target/real-user-e2e/<scenario>/<timestamp>/
  repo/
  artifacts/
    user-prompt.txt
    whale-exec.jsonl
    whale-exec.stderr.log
    last-message.md
    git-diff.patch
    validation.stdout.log
    validation.stderr.log
    hidden_oracle.py
    hidden-oracle.stdout.log
    observability/
      action-map-observability.json
      action-map-observability.md
      action-map-observability.html
    report.md
```

`report.md` 必须包含：

- scenario id。
- whale binary path、version、sha256。
- model。
- thread id。
- prompt leak 检查结果。
- 工程可行性指标。
- 效用指标。
- 证据等级。
- key edge reason 覆盖率。
- node result consumed 事件或等价证据。
- per-node source refs 与 source overlap 摘要。
- failures。
- artifact 路径。

关键 edge 报告字段：

```text
edge_id
from_node_id
to_node_id
dependency_kind
edge_reason
created_at
created_by
from_result_id
consumed_by_node_id
```

第一版如果 runtime 尚未持久化全部字段，E2E report 必须标注字段缺失，不得把 edge_count 单独解释为健康依赖证据。

## 失败分类

| 类型 | 含义 | 处理方式 |
|---|---|---|
| Harness failure | 脚本、路径、环境、依赖失败 | 修测试基础设施 |
| Runtime failure | TaskSpace gate、binding、lease、tool call 协议失败 | 修 runtime |
| Decomposition failure | 图没有健康生长，主 agent 退回线性模式 | 修方法论注入和调度约束 |
| Business failure | 最终代码行为不满足验收 | 分析 agent 能力和任务上下文 |
| Observability failure | 任务完成但无法解释过程 | 修 viewer/export |
| Cost regression | 简单任务被明显拖慢或工具调用过多 | 调整复杂度识别和轻量路径 |

## 近期实施顺序

1. 把现有 natural/growth E2E 归入 L2/L3。
2. 补 L1 简单任务基准，证明 TaskSpace 不拖累。
3. 给 L1/L2 增加 standard/taskspace paired 对照，建立 E2 证据。
4. 补 L3 架构质量/Debug 压力任务，观察主 agent 是否主动调度。
5. 补 L4 多轮任务和上下文压缩样本。
6. 建立历史真实失败 corpus，接入少量 Terminal-Bench/SWE-bench 风格任务，冲击 E3 证据。

## L4 Harness 草案

L4 不在第一阶段追求回溯，但需要能证明 task 结构没有被压缩或插话破坏。

最小 harness：

```text
turn 1: 用户提出 task A，中复杂度任务。
turn 2: 用户补充 task A 约束，触发 map growth。
turn 3: 用户插入 task B，要求快速处理。
turn 4: 用户回到 task A，要求继续。
turn 5: 人为触发或构造上下文压缩边界。
turn 6: 用户继续 task A。
```

验收：

- task A/B 有不同 task identity。
- task B 的 node result 不写入 task A。
- 回到 task A 时 current binding 指向 task A 的有效 node。
- 压缩后 task manifest 仍保留 task id、active map、open node、关键 completed result summary。
- 不要求恢复旧完整上下文，不要求重放历史节点。

L4 manifest 扩展：

```json
{
  "turns": [
    { "id": "turn-1", "prompt_file": "turns/001-task-a.md", "expect_task": "task-a" },
    { "id": "turn-2", "prompt_file": "turns/002-task-a-constraint.md", "expect_task": "task-a" },
    { "id": "turn-3", "prompt_file": "turns/003-task-b-interruption.md", "expect_task": "task-b" },
    { "id": "turn-4", "prompt_file": "turns/004-return-task-a.md", "expect_task": "task-a" }
  ],
  "compaction_trigger": {
    "type": "force_context_compaction_after_turn",
    "turn_id": "turn-4"
  },
  "resume_assertions": [
    "task-a-active-map-still-visible",
    "task-b-results-not-in-task-a",
    "current-binding-valid"
  ]
}
```

L4 runner 不能只使用一次 `whale exec`。它需要支持同一 session/thread 的多 turn 发送、观测导出和压缩触发。若当前 CLI 暂不支持强制压缩，L4 场景先标记为 harness gap，不能伪造压缩通过。

## Harness 自测

统一 harness 需要自己的自测，避免 benchmark 基础设施悄悄失真。

必备自测：

| 自测 | 做法 | 期望 |
|---|---|---|
| variable-control self-test | 故意让 prompt、fixture、model param、permission、cwd policy 任一不一致 | pair 标记 invalid，不进入 utility aggregate |
| prompt-narrative self-test | 构造显性/隐性内部机制暗示 prompt | 标记 invalid_prompt，不进入 utility aggregate |
| prompt-narrative false-positive test | 构造 Node.js、source map、map parsing、parallel tests、concurrent request race、performance benchmark 等自然工程 prompt | 不得自动 invalid；需要复核的必须记录原因 |
| hidden-oracle-isolation test | 构造 agent 尝试读取 private oracle 的场景 | 读取失败或 report 标记 isolation failure |
| l4-manifest test | 使用多 turn manifest、插话、resume assertions | runner 能按 turn 执行并导出 task identity 证据 |
| flake-lineage test | 模拟 harness failure 后重跑 | 新 run id、原失败保留、aggregate denominator 正确 |
| external-dry-run test | 原始 prompt/validator 原样跑 | standard/taskspace 初始 checksum 一致 |

没有通过 harness 自测时，不允许把任何结果标为 E2/E3。

## 当前非目标

- 不用 E2E 做主观质量打分。
- 不要求低复杂度任务强行拆成漂亮 graph。
- 不把用户 prompt 写成内部流程指令。
- 不在第一阶段追求完整回溯、历史节点重放或自动恢复。
- 不为了通过 benchmark 写固定关键词回复或绕过模型路径。
