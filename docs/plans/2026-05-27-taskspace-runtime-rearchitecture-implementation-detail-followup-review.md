已完成对 `docs/plans/2026-05-27-taskspace-runtime-rearchitecture-implementation-plan.md` 的只读复核。以下是复核意见：

---

## 复核摘要

**整体评价：文档结构完整、方向明确。** 从问题诊断到数据模型、控制协议、gate 设计、工程落地、分阶段计划都覆盖了，是一份质量较高的实施文档。

---

## 发现的问题与建议

### 1. 文档交叉引用断裂
文档第 36-37 行引用了三个对抗审查记录文件，但这些文件均未在仓库中提交（`git status` 只显示未跟踪的 `2026-05-27-taskspace-runtime-rearchitecture-implementation-detail-review.md`，另外两个 `.md` 完全不存在）。建议明确这些文件的状态——是尚未创建，还是存放路径与引用不一致。

### 2. `NodeStatus::Closed` 不可重新 bind 的一致性风险
第 282-283 行规定 `Closed` 不可重新 bind，但第 509 行 `finish_node` 又允许通过 `next_node_id` 在同一次 state lock 内立即 bind 下一个 node。两处语义一致（都不是 re-bind 同一个 node），但措辞容易让实现者混淆。建议在第 282 行补充说明："不可重新 bind 同一 Closed node，但不影响 `finish_node` 的 `next_node_id` 绑定不同 node。"

### 3. `MaintenanceBarrier` 与 `repair_required` 的优先级未明确定义
Gate 判定表（第 874-882 行）分别列举了两者，但未定义 `maintenance_barrier` 和 `repair_required` 同时存在时谁的优先级更高。建议明确：两者同时存在时，先要求 repair，再处理 barrier。

### 4. Phase 0 的验收标准与 Phase 3/6 的阈值联动
Phase 0 提到"单宽 node 吸收 70+ 工具调用"的失败样本，但 Phase 6 的默认阈值（单 node main tool results 默认 12）远低于此。建议在 Phase 0 就先定义好阈值常量，让失败样本直接断言这些阈值，而不是先写一个 70+ 的样本后面再去对 12 的阈值。

### 5. `EdgeDraft` 缺省值策略
第 243 行规定 `EdgeDraft` 必须显式带 `kind`，"缺省值第一版不允许"。这是一个好的安全决策，但实现时需要考虑对已有调用的影响面——如果旧 `create_node` 路径中有任何隐式创建 edge 的逻辑，必须逐个修改。

### 6. 压缩后的 prompt 注入
第 792 行说"prompt 注入只给 active task pack"，但第 625 行提到 pending task 的 manifest 摘要（ready node 计数、blocked 计数等）也需要注入。两者不完全矛盾（active task 给完整 pack，pending task 给摘要），但措辞可能让实现者忽略 pending task 的摘要需求。建议在压缩策略中补充这一点。

### 7. 并发 claim 的锁粒度
第 318-319 行要求 lease claim 在 `SessionState` 锁内原子完成。需要确认当前 `SessionState` 的锁粒度是否足以承载这个临界区——如果 `SessionState` 锁在 claim 之前就被释放，或者工具 dispatch 路径上有多个锁点，这个要求可能需要额外的架构调整。建议在 Phase 5 实施前先做一次锁路径审计。

---

## 文档中没有明显问题但值得注意的点

- **8 个 Phase 的依赖关系**：Phase 1-3 是强依赖链（必须先改领域模型才能改协议，再改 gate），但 Phase 4（spawn node_id）和 Phase 5（并发/reborn）是否可以部分并行，取决于 spawn schema 变更是否依赖并发原语——文档中未明确这一点。
- **"只进不出"约束**（第 425 行提到 TaskSpace 只进不出）：`ControlRecoveryRequired` 状态下允许的操作集合（`route_task(AskUser/CreateTask)`、`cancel_reborn`、repair 动作）是合理的，但它等于变相允许脱离当前任务。文档可以明确这一点，避免实现者误解为"绝对不能脱离"。

---

需要我针对上述任何一个问题做更深入的分析吗？
