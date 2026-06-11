# 示例：processing-pipeline 的 standard mode 运行形态

## recommended_mode

```yaml
recommended_mode: standard
reason:
  - 多脚本/多文件表面
  - 可能有 environment、permissions、script logic 等独立证据轨
  - 可允许有限 subagent，但必须有 ROI 追踪
node_budget_hint: 4-12
subagent_budget_hint: 0-3
```

## 期望 node path

```text
node-1 discover: identify script chain, validator contract, relevant files
node-2 diagnose: classify likely failure surfaces
node-3 design: decide patch set and validation strategy
node-4 patch: edit scripts/permissions/paths
node-5 validate: run pipeline/validator
node-6 synthesize: close criteria, report risks
```

## subagent 使用

仅在明确独立轨道时 spawn，例如：

```text
Track A: script content correctness
Track B: environment/permissions/path assumptions
```

每个 subagent 必须返回 evidence summary，并被 main agent adopted 或 discarded。
