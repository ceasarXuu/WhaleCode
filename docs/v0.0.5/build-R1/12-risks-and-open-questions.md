# 12. Risks and Open Questions

## 1. 主要风险

### 风险 1：state_commit 过大，模型填错

缓解：

```text
- 支持 partial commit
- runtime 局部接受/拒绝
- 提供模板
- 允许模型先提交最小 decision chain
```

### 风险 2：projection 过短导致模型失忆

缓解：

```text
- artifact refs 可按需展开
- active blockers/criteria/decisions 强制保留
- 先 shadow，不直接删除标准 history
```

### 风险 3：map GC 误删关键证据

缓解：

```text
- GC 不物理删除，默认 archive/audit-only
- 所有 GC 有 trace event
- 可 request_expand archived evidence
```

### 风险 4：thin routing 误判复杂任务

缓解：

```text
- validator failure / ambiguity 自动升级
- router confidence 低时进入 default-compact
- E3 报告 routing mistakes
```

### 风险 5：成本下降但正确率下降

缓解：

```text
- analyze-access-logs 作为可靠性回归
- quality gate 允许小幅回退但必须解释
- 不把 2x 成本目标孤立作为 release success
```

## 2. 需要讨论的问题

1. v0.0.5 的 2x 成本目标是否按 suite aggregate 判断，还是每个 sample 都必须满足？
2. TaskSpace solved 是否允许低于 v0.0.4 的 8/15？如果允许，容忍范围是多少？
3. `state_commit` 是否作为唯一新 action，还是保留少数高频 action 的快捷版本？
4. 大输出 threshold 初始值是否采用 8KB/50KB/150KB？
5. semantic replacement rate 的 release gate 是否只 report-only？
6. thin routing 是否默认启用在所有 TaskSpace tasks，还是只在 E3 profile 中启用？
7. 是否需要保留 v004 legacy TaskSpace profile 作为回放/回归？

## 3. 建议默认答案

```text
1. 2x 按 suite aggregate + sample-level warning 判断，不要求每个 pair 严格 <=2x。
2. solved 不应低于 Standard；若低于 v0.0.4 8/15，需要明确是成本压缩导致还是随机波动。
3. state_commit 作为主入口，旧 action soft-deprecate。
4. threshold 采用 8KB/50KB/150KB，后续按数据调。
5. semantic replacement rate v0.0.5 report-only。
6. thin routing 默认启用，但可升级 default-compact。
7. 保留 v004 legacy profile。
```
