# R7.1-A2 非终态动作唯一承载对抗性审查

- Date: 2026-07-25
- Target:
  [`42-r7.1-a2-nonterminal-action-ownership-design.md`](../docs/v0.0.5/build-R7/42-r7.1-a2-nonterminal-action-ownership-design.md)
- Review mode: Architecture adversary
- Reviewer context: Fresh, no conversation history
- Implementation review: Not applicable; A2 is design-only
- Overall status: Approvable as implementation candidate

## 1. 审查目标

本轮不验证代码收益，而是攻击 A2 是否：

- 真正消除跨兄弟 Tool call 约束，而不是换一种后置拒绝；
- 保持 Agent 语义所有权和 Runtime 硬底线；
- 准确定义 Map commit、普通 Tool dispatch、失败与崩溃窗口；
- 保留多动作、单 Patch、原生参数/反馈、Standard 和三 projection 隔离；
- 覆盖全部生产入口且不留下双所有权；
- 区分产品批准、工程实施和 R-10 关闭证据。

## 2. 第一轮发现与处置

### B-01：commit/dispatch 崩溃窗口缺少 durable 事实

- Severity: Blocking
- Reviewer finding: boundary 已提交后若进程在业务 dispatch 前崩溃，只有 transient tracing，Agent 恢复时无法区分
  “尚未执行”和“结果未知”。
- Decision: Accept with narrower implementation.
- Correction:
  - 在现有 Map Store 中加入最小 carrier attempt 事实，不建立第二份 Map；
  - boundary commit 与 `committed_not_started` 同事务；
  - dispatch 前更新为 `started`，结果后更新为 `completed`；
  - `read_map`/projection 暴露未完成 attempt；
  - 不保存业务 payload，不自动重试，不让 attempt 参与语义决策。
- Rationale: 单靠日志不满足 faithful recovery；但通用 durable workflow dispatcher 会越界。修订只记录已经发生的
  机械事实。
- Resolution: Addressed in design v1.1 sections 7.4, 8.2, 11.3, 12 and 13.

### B-02：多 carrier 失败后的依赖语义不确定

- Severity: Blocking
- Reviewer finding: 文档写“依赖 segment 跳过”，却没有依赖字段，Runtime 会被迫判断语义依赖。
- Decision: Accept.
- Correction:
  - 使用唯一机械规则：任一 barrier 的 Map commit 或业务 Tool 失败后，同 response 后续调用全部跳过；
  - `active` 同段的并行动作仍按现有规则一起执行；
  - 有返回值依赖或需要失败后重新决策的动作等待下一次 response。
- Rationale: 这与现有 barrier failure 行为一致，不增加 dependency schema，也不要求 Runtime 解释任务语义。
- Resolution: Addressed in sections 7.3, 8.2, 9 and 11.3.

### B-03：“真实动作不可分离”的承诺超出 schema 能力

- Severity: Blocking
- Reviewer finding: Agent 仍可让空查询、错误命令或无价值读取携带 boundary；Runtime 无法机械判断是否“真实有用”。
- Decision: Accept the boundary correction; reject allowlist/semantic gate.
- Correction:
  - 承诺改为“provider-visible schema 中没有脱离普通 Tool call 的 standalone boundary”；
  - 明确语义空洞或错误 Tool 是 Agent 质量问题；
  - 禁止 Runtime 通过 Tool allowlist、命令内容、查询文本或 node goal 判断工作价值。
- Rationale: A2 解决结构断裂，不应把 Agent 能力问题重新变成 Runtime 语义控制。
- Resolution: Addressed in sections 1, 3.1, 5.4 and 11.1.

### B-04：实施面遗漏导致双所有权风险

- Severity: Blocking
- Reviewer finding: 初稿未列 provider endpoint fixtures、L2 旧 wire、机器合同、benchmark reducer 和全仓残留门。
- Decision: Accept.
- Correction:
  - B3 同步迁移 L2 wire、provider endpoint fixtures、schema snapshots、机器合同和 reducers；
  - 实施矩阵增加 Map Store、provider adapter、L2、产品合同和 reducer owner；
  - 全仓扫描旧 action/reason code，只有 historical docs/COE/archive trace 可豁免。
- Resolution: Addressed in sections 10, 13 and 14.

### N-01：设计批准与实现证据混淆

- Severity: Non-blocking
- Decision: Accept.
- Correction:
  - 候选 C 改为首选实施候选，仍受 B0/B4/B5 证据约束；
  - 产品确认只授权进入 B，不关闭 R-10；
  - R-10 还需 R7.1-C 真实模型门。
- Resolution: Addressed in sections 4 and 16.

### N-02：R-21/R-23 状态权威不一致

- Severity: Non-blocking
- Decision: Accept.
- Correction:
  - 更新五层整体约束，R-21 标记为 R7.1-A1 closed，R-23 标记为 R7.1-A0 closed；
  - 二者继续作为不可回退准入门；
  - 当前 open regression 收敛为 R-10/R-19/R-22。
- Resolution: Addressed in
  [`38-r7-five-layer-integrated-change-constraints.md`](../docs/v0.0.5/build-R7/38-r7-five-layer-integrated-change-constraints.md).

## 3. 独立本地补充发现

### L-01：ToolSearch 不能复用普通函数结果包装

- Severity: Blocking if unspecified
- Evidence: 当前 initialization wrapper 对 `ToolSearchOutput` 不做前插，而 provider pairing 要求
  `status=completed`。
- Decision:
  - 保留 `ToolSearchOutput(status=completed)` 作为协议配对；
  - carrier 事实通过关联同一 call id 的 supplemental factual item 返回；
  - `status=completed` 不解释为业务成功；
  - pre-dispatch failure 仍返回空配对输出和唯一 carrier 失败事实。
- Resolution: Addressed in design section 8.1 and implementation matrix.

## 4. 第二轮复审

第二个 fresh-context reviewer 重新读取修订后的设计、五层约束、里程碑和生产 Rust 路径，结论为：

- Blocking findings: 0
- Non-blocking implementation clarification: 1
- Verdict: `APPROVABLE AS IMPLEMENTATION CANDIDATE`

唯一细化项是 ToolSearch carrier feedback 必须复用现有输出惯例：

1. 只生成一个 provider-paired `ToolSearchOutput(status=completed)`；
2. 再生成一个 unpaired factual `Message`；
3. Message 正文携带原 `call_id` 和 carrier 事实；
4. 不生成第二个同 call id 的 provider-paired output。

设计 v1.2 已写死该形状。

## 5. 当前结论

第一轮所有 Blocking 均已在设计层处理，第二轮无剩余 Blocking。A2 可提交产品确认，并可在确认后进入
R7.1-B；此结论不代表生产实现完成、收益成立或 R-10 已关闭。
