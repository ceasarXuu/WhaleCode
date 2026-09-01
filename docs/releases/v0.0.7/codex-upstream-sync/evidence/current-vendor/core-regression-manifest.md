# Codex 0.151 当前 vendor core 非绿清单

- Subject: final-wire commit `b631eb7e67` + W6 metadata closeout index
- Upstream baseline: `rust-v0.151.0` / `78c290807ce710180111df227df3b7a4fe845452`
- Exact targeted run: `83e27242-db88-4ac4-812c-36a46a9bfdd7`
- Result: 8 tests；7 failed；1 timed out；0 unexpected tests
- Raw evidence: [`core-approved-deferrals.junit.xml.gz`](core-approved-deferrals.junit.xml.gz)
- Full controlled run: `7612801b-9552-45fa-929d-e4d04697efbc`
- Full result: 3969 tests；3948 passed（1 flaky）；7 failed；14 timed out；9 skipped
- Full raw evidence: [`core-full-j4.junit.xml.gz`](core-full-j4.junit.xml.gz)

## 已批准留到 TaskSpace 专项的精确 7+1

用户决定为“这个问题先记录下来跳过，后续在 TaskSpace 分支中解决”。本表只把该决定绑定到精确测试与签名，不把失败写成通过，也不修改 TaskSpace 产品逻辑。对抗性审查的冻结 non-goal 同样明确：本轮不强行修复已批准的 W9/TaskSpace 延期项。

| Test | 当前签名 | pristine 0.151 | 生产路径 | 延期簇与权威 |
| --- | --- | --- | --- | --- |
| `cyber_access_program_is_inherited_by_child_turns` | `fork_turns=none` 的 child request metadata 为 `Null`，期望 `daybreak_red` | PASS | child/fork turn metadata inheritance | TaskSpace/W9 会话继承；用户明确延期，review frozen non-goal |
| `cyber_access_program_survives_mid_turn_remote_compaction` | remote compact 后 request metadata 为 `Null`，期望 `daybreak_blue` | PASS | remote compaction v1 metadata inheritance | 同上 |
| `cyber_access_program_survives_mid_turn_remote_compaction_v2` | 三次 request metadata 均为 `Null` | PASS | remote compaction v2 metadata inheritance | 同上 |
| `recover_turn_restores_cyber_access_program_without_making_it_sticky` | recover 后为 `None`，期望 `DaybreakBlue` | FAIL，同签名簇 | recover/session restoration | 上游已知同签名 + TaskSpace 会话恢复专项；用户明确延期 |
| `cyber_access_program_changes_on_one_websocket_with_response_reuse` | response reuse 后错误尝试把 `ws://.../v1/responses` 作为 HTTP endpoint | PASS | websocket response reuse + turn metadata | TaskSpace/W9 websocket lifecycle；用户明确延期 |
| `queue_only_agent_mail_wakes_sleeping_root_with_previous_turn_context` | queue-only wake 的 previous-turn metadata 为 `Null` | PASS | pending input / agent mail previous-turn context | TaskSpace/W9 agent lifecycle；用户明确延期 |
| `opted_in_executor_provider_skips_host_discovery_but_injects_discovered_skill` | observed request count `2`，fixture expected `3` | PASS | extension executor / skill request orchestration | extension 请求编排专项；用户明确延期 |
| `websocket_turn_state_persists_within_turn_and_resets_after` | server 完成 3 个 response batch 后 test 仍未结束，60s timeout | PASS | websocket turn-state reset | TaskSpace/W9 session lifecycle；用户明确延期 |

Pristine 对照来自 [`04-core-tests.log`](../rust-v0.151.0/attempt-1-isolated-qualification/04-core-tests.log)。其中 1 项为上游同签名，另外 7 项是 Whale 集成后的真实非绿项；“批准延期”不等于“上游原本失败”。

## 当前宿主的额外 zsh-fork 超时

受控 `-j 4` 全量还出现 13 个 zsh-fork / exec-wrapper 测试超时。它们不属于上表 7+1，也未被冒充为 TaskSpace 延期：

- 相同生产代码此前完整隔离运行没有这些超时，说明不是由本次 W6 文档、metadata 或 snapshot 变更引入。
- 单独重跑 `env_zsh_script_spawned_by_python_can_request_escalation_under_zsh_fork` 仍在 60s 超时，证明当前宿主对 DotSlash zsh + exec-wrapper intercept 的运行条件不满足，而不是全量并发独有。
- 本轮保存完整 JUnit，不将这些测试标记为通过；它们属于宿主 zsh-fork 验证限制，不能用于推翻已通过的非 zsh 生产矩阵。

如果未来把 zsh-fork 作为 Whale Linux 发布矩阵的强制产品面，必须在具备对应 exec-wrapper intercept 条件的独立宿主重新验证，不能沿用本清单豁免。
