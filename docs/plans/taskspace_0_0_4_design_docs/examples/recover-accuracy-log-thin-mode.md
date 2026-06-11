# 示例：recover-accuracy-log 的 thin mode 运行形态

## recommended_mode

```yaml
recommended_mode: thin
reason:
  - standard 在 0.0.3 中 5/5 成功
  - 任务目标集中在输出 artifact 恢复
  - 不需要多 surface 并行调查
  - subagent 默认不应启用
node_budget_hint: 1-4
subagent_budget_hint: 0
```

## 期望 node path

```text
node-1 discover/diagnose: identify required output contract and source logs
node-2 patch: generate/fix recovered outputs
node-3 validate: run public validator once
node-4 synthesize: report satisfied criteria and risks
```

## 不期望行为

```text
创建多个并行 inspect node；
spawn agent 只为读同一组文件；
重复 validator 而没有新 decision；
final synthesis 前没有 success criteria。
```
