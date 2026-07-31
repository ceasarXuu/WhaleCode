# R8-I09 Store Hydrate 完整性修复结果

- Completed: 2026-07-31
- Status: closed
- Scope: canonical Map 从 SQLite Store 进入 Core Runtime 的恢复边界
- Whale Agent runs: 0

## 1. 实际根因

I09 由两个相连但可独立验证的缺口组成：

1. `restore_store_map()` 安装持久化 canonical Map 前没有调用已有
   `rooted_dag::validate()`，且失败前会先改变 Runtime mode。
2. child/fork hydrate 在验证父 Map 可恢复前先写入 binding，恢复失败后留下持久化副作用。

失败测试分别证明：cycle Map 会被旧恢复路径接受；Map ID mismatch 会污染 mode；非法父 Map 被拒绝后，
新 child/fork 线程仍能查询到 binding。

被否定的假设：

- 不是项目缺少 Map 合法性检查器；
- 不是 Store codec 应复制 Core rooted-DAG 规则；
- 不是 `ActionMapInstance::from_graph()` 已经隐式校验；
- 不是 rollout/session history 还在生产路径重建第二份 canonical Map。

## 2. 实施结果

| 工作面 | 结果 |
|---|---|
| Runtime restore | 非空 Store Map 先检查 identity，再复用唯一 `rooted_dag::validate()`；成功后才改变 mode 并安装 |
| 安装入口 | `restore_canonical_map()` 收窄为 `restore_store_map()` 内部私有实现 |
| child/fork | 父 record 在写 binding 前通过同一恢复校验；失败不写 binding |
| 合法图保护 | 多父 DAG、closed Map、reopened Map 恢复后与 Store canonical value 精确相等 |
| 日志 | 新增 `taskspace.map_store_hydrate_rejected`，记录稳定原因码与机械身份，不记录节点目标或用户内容 |
| 恢复权威 | 生产恢复保持 SQLite Store 单一事实源；`replay_batches` 仅用于 rooted-DAG 测试 |

没有修改 Map schema、状态机规则、Tool、projection、Standard session 或历史兼容策略。

## 3. 验证结果

| 命令 | 结果 |
|---|---:|
| `cargo test -p codex-core restore_store_map_ -- --nocapture` | 4 passed |
| `cargo test -p codex-core session::taskspace_store_tests -- --nocapture` | 8 passed |
| `cargo test -p codex-state taskspace_map -- --nocapture` | 7 passed |
| `cargo check -p codex-core` | passed |
| `git diff --check` | passed |

覆盖的负向状态包括 cycle、不可达节点、fact conflict、Map ID mismatch 和非法 parent binding；
正向状态包括显式空 Map identity、active、多父依赖、closed、reopened、resume、child 和 fork。

## 4. 日志合同

拒绝事件包含：

- `reason_code=canonical_map_invalid|canonical_map_identity_mismatch|canonical_map_restore_rejected`
- `map_id`
- `actor_thread_id`
- `owner_thread_id`
- `relation`
- `store_revision`
- `map_revision`
- `terminal`

完整 violation 仍通过原始错误返回给调用方，日志不写 violation subject、node goal、工具结果或用户正文。

## 5. 全局约束检查

| 约束 | 结论 |
|---|---|
| Map Store 唯一事实源 | 满足；未增加 replay、snapshot 或 fallback |
| Runtime 只守硬底线 | 满足；只校验既有 canonical invariant，不替 Agent 决策 |
| 不复制状态语义 | 满足；Core 复用唯一 validator，State 只做存储一致性 |
| 不入侵普通 Tool | 满足；Tool 路径无改动 |
| Standard 隔离 | 满足；Standard session 路径无代码变更 |
| 不做历史兼容 | 满足；非法或旧 schema 明确失败 |
| 日志驱动 | 满足；拒绝事件有字段级回归 |
| 成本门禁 | 满足；未运行真实 Whale Agent，无 API 成本 |

## 6. 提交

- `e92241ed6` `fix(taskspace): validate stored maps before restore`
- `6a31eeb96` `fix(taskspace): validate parent map before binding`
- `c7ec19d0b` `test(taskspace): cover persisted map lifecycle restore`
- `923e8c945` `feat(taskspace): log canonical map hydrate rejection`

I09 已满足关闭条件。R8 下一项按底层优先级进入 I01：prepare revision 与最终 canonical revision 的双成功事实。
