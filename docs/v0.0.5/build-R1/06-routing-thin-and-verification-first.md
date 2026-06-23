# 06. Routing, Thin Path, and Verification-First Workflow

## 1. 背景

v0.0.4 说明 TaskSpace 不同任务收益差异明显：

```text
analyze-access-logs: TaskSpace 5/5，Standard 4/5，但成本极高
log-summary: TaskSpace 3/5，Standard 3/5，subagent 多但净收益不稳定
count-call-stack: 双方 0/5，TaskSpace 更贵但没有形成新路径
```

这说明 TaskSpace 不能所有任务都走同一套重型协议。

## 2. 设计目标

v0.0.5 增加 TaskShapeRouterV1，把任务分到：

```text
thin
default-compact
verification-first
subagent-assisted
deep
```

本版重点不是自动精准，而是避免明显低收益任务被重型化。

## 3. Task Shape 分类

### Thin

适用：

```text
- 单文件或少量文件
- 目标明确
- 不需要并行调查
- validator 明确
- 标准模式通常能快速处理
```

行为：

```text
- 不创建大 graph
- 不默认 spawn subagent
- success criteria 批量初始化
- state_commit 低频
- 只在失败或 ambiguity 时升级
```

### Verification-first

适用：

```text
- parser/output-format 敏感任务
- validator failure 文本对成败关键
- 任务本身不难，但格式要求精确
```

典型：`count-call-stack`。

行为：

```text
1. 先读取测试/validator/expected format
2. 记录 expected format decision
3. 生成本地 checker 或最小验证脚本
4. 生成产物
5. 本地 checker 通过后再 public validation
6. validator 失败后必须产生 revised decision
```

### Default-compact

适用：中等复杂度、多步但不需要多 agent。

行为：

```text
- state_commit
- compact projection
- result lifecycle
- limited graph
```

### Subagent-assisted

适用：

```text
- 多证据轨
- 可并行独立检查
- subagent artifact 可以明确验收
```

行为：

```text
- spawn 前必须有 decision_target
- subagent result 必须 review/adopt/reject/defer
- no-yield 后停止同类 spawn
```

### Deep

适用：高不确定、多模块、多阶段工程任务。

v0.0.5 不以 deep 为默认优化目标。

## 4. Router 输入

```json
{
  "task_prompt_features": {
    "file_scope": "small|medium|large",
    "output_artifact_required": true,
    "format_sensitive": true,
    "validator_visible": true,
    "multi_source": false,
    "code_patch_required": false,
    "ambiguity": "low|medium|high"
  },
  "observed_runtime_features": {
    "validator_failure_seen": false,
    "large_output_seen": false,
    "uncertainty_increased": false
  }
}
```

## 5. Router 输出

```json
{
  "recommended_mode": "verification-first",
  "confidence": "medium",
  "reason": "task requires exact output.txt format and validator tests are available",
  "initial_constraints": {
    "subagent_allowed": false,
    "node_budget": 4,
    "state_commit_budget": 4,
    "large_output_policy": "ref-only",
    "must_read_validator_first": true
  }
}
```

## 6. Escalation policy

thin / verification-first 可以升级，但必须有触发条件：

```text
- validator failure after local self-check
- multiple incompatible hypotheses
- evidence source too large for single agent
- ambiguity cannot be resolved by local inspection
- repeated local checker failure
```

升级后不是进入 legacy full TaskSpace，而是进入 `default-compact`。

## 7. Downgrade / stay-thin policy

当任务已经有明确 patch path 或产物 path 时，保持 thin：

```text
- success criteria clear
- no open blocking question
- no need for subagent
- local checker available
```

不要因为 TaskSpace enabled 就自动扩 graph。

## 8. 验收指标

| 指标 | 目标 |
|---|---:|
| `count-call-stack` 进入 verification-first | 100% |
| thin/default/deep routing 输出存在 | 100% TaskSpace runs |
| thin task subagent spawn | 0 by default |
| thin task state_commit_count | <= 4 before first validation |
| verification-first expected-format decision | 100% parser/format tasks |
| validation failure -> revised decision | 100% verification-first failed runs |

## 9. 风险

| 风险 | 缓解 |
|---|---|
| router 误判复杂任务为 thin | 允许 validator failure / ambiguity 升级 |
| verification-first 增加前置成本 | 只对 format-sensitive 任务启用 |
| subagent 被过度抑制 | high-uncertainty / multi-source 可转 subagent-assisted |
