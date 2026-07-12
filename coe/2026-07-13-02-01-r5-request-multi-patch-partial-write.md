# Problem P-001: 单响应多 Patch 允许部分写入
- Status: confirmed
- Created: 2026-07-13 02:01
- Updated: 2026-07-13 02:01
- Objective: 让Standard与TaskSpace的单个provider response最多声明一个`apply_patch`，并保证单patch的validation failure零文件副作用
- Symptoms:
  - R5复杂样本在一个response中生成5个顶层`apply_patch`，前4个成功、第5个失败，workspace进入部分提交状态。
  - 单个multi-file patch也会在后序hunk validation失败时保留前序文件写入。
- Expected behavior:
  - 完整response在任何工具或状态副作用前完成patch数量预检；超过一个patch时整组零执行。
  - 单patch全部hunk在首个文件写入前完成解析、路径、上下文和目标内容计算。
- Actual behavior:
  - 普通工具段直接并行dispatch，未做request-wide patch count预检。
  - `apply_hunks_to_files`逐hunk计算并立即写入，后序失败不撤销前序写入。
  - `initialize_then_actions.actions[]`允许重复出现`apply_patch`。
- Impact:
  - Standard与TaskSpace均可能产生部分文件修改、失败反馈与workspace真实状态不一致的恢复成本。
  - 多个patch并行修改同一文件时还存在覆盖、上下文竞争和不可确定结果风险。
- Reproduction:
  - 重放`target/r5-final-loop-fix-repeat3/subscription-billing-repair/20260713-002149-397/pair-002/left/artifacts/rollout.jsonl`中的5个兄弟patch response。
  - 运行`test_apply_patch_cli_failure_after_partial_success_leaves_changes`。
- Environment:
  - Linux，branch `whalecode-alpha`，J6.7.7完成后的R5 Docker benchmark与当前Codex Rust workspace。
- Known facts:
  - `pair-002/left`同一response含5个顶层patch，结果为4 success + 1 failure。
  - `pair-003/right`同一response含4个顶层patch且全部成功，证明该形态不只出现在错误恢复中。
  - `ExecutorFileSystem`只有read/write/create/remove/copy原语，没有rename或事务API。
  - DeepSeek stable和beta strict都接受`contains/maxContains`，但请求两个patch时均生成两个patch。
- Ruled out:
  - 不是projection语义扭曲导致；工具调用原样进入canonical trace。
  - 不能只靠TaskSpace carrier schema修复；最新失败来自顶层兄弟工具调用。
- Fix criteria:
  - request-wide patch count覆盖顶层、carrier和nested alias，超限时任何工具、Map状态和文件均无副作用。
  - 全部validation failure零文件副作用，Standard/TaskSpace共享同一patch实现。
  - carrier schema不再能表达第二个patch，同时保留单patch后的read/test动作。
  - 结构化日志可区分request preflight、patch prepare和commit failure。
- Current conclusion: H-001的shared patch substrate已通过J7.1修复验证；H-002/H-003仍待J7.2-J7.3关闭，Problem保持open。
- Related hypotheses:
  - H-001
  - H-002
  - H-003
  - H-004
- Resolution basis:
  - not satisfied
- Close reason:
  - not closed

## Hypothesis H-001: Patch边验证边写导致validation partial commit
- Status: confirmed
- Parent: P-001
- Claim: `apply_hunks_to_files`在遍历hunk时立即写入，因此后序hunk的路径或上下文校验失败会保留前序副作用。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - 既有CLI测试名称直接描述partial success，生产函数也在循环内调用write/remove。
- Falsifiable predictions:
  - If true: add成功后missing update失败时，新增文件仍存在。
  - If false: 失败后workspace应保持调用前状态。
- Diagnostic evidence plan:
  - Prediction or clause under test: 后序validation failure是否保留前序写入。
  - Signal: CLI测试断言与`apply_hunks_to_files`写入顺序。
  - Capture method: 读取既有测试和生产函数，随后运行focused test。
  - Event name or marker:
    - `test_apply_patch_cli_failure_after_partial_success_leaves_changes`
  - Correlation keys:
    - test name
  - Differentiates from:
    - response sequence只执行多个独立patch导致的partial commit
  - Supports if:
    - 测试断言失败后`created.txt`仍存在且生产代码在后序derive前已write。
  - Refutes if:
    - 生产代码先prepare全部hunk或测试断言workspace不变。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 修复后保留prepare/commit结构化事件
- Evidence gate: satisfied
- Related evidence:
  - E-001
  - E-002
- Conclusion: confirmed
- Repair design readiness: ready；用户已明确授权执行到J7.4
- Next step: 将patch处理拆为无副作用prepare和显式commit
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-002: 共享工具序列缺少request-wide patch预检
- Status: confirmed
- Parent: P-001
- Claim: `execute_response_tool_sequence`在完整response级别没有统计canonical patch身份，普通工具segment会直接并行dispatch多个patch。
- Layer: root-cause
- Factor relation: all_of
- Depends on:
  - none
- Rationale:
  - 最新R5 active Map通过顶层native tools生成多个兄弟patch，没有进入carrier schema。
- Falsifiable predictions:
  - If true: response含多个顶层patch时会全部进入`join_all`，不存在执行前整组reject。
  - If false: sequence入口应先构建manifest并在count大于1时返回零执行结果。
- Diagnostic evidence plan:
  - Prediction or clause under test: 顶层多patch是否在完整序列校验前被dispatch。
  - Signal: sequence生产代码和pair-002真实call/output顺序。
  - Capture method: 检查sequence入口并解析canonical rollout事件。
  - Event name or marker:
    - `tool_response_parallel_segment_started`
  - Correlation keys:
    - provider logical request id
    - tool call ids
  - Differentiates from:
    - 仅TaskSpace nested carrier重复patch
  - Supports if:
    - 多个patch处于同一ordinary segment并由`join_all`执行，trace出现多个success后failure。
  - Refutes if:
    - sequence入口已有request patch manifest硬门禁。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 增加request patch count validated/rejected事件
- Evidence gate: satisfied
- Related evidence:
  - E-003
  - E-004
- Conclusion: confirmed
- Repair design readiness: ready；用户已明确授权执行到J7.4
- Next step: 在共享dispatcher任何segment/state/tool执行前建立并验证manifest
- Blocker:
  - none
- Close reason:
  - not closed

## Hypothesis H-003: Carrier工具契约允许重复Patch
- Status: confirmed
- Parent: P-001
- Claim: `TaskSpaceControlArgs::InitializeThenActions.actions`是无数量/身份约束的普通action数组，validator只禁止嵌套control/update_plan，没有排除多个`apply_patch`。
- Layer: sub-cause
- Factor relation: part_of
- Depends on:
  - H-002
- Rationale:
  - J6 carrier失败曾在一个actions数组中连续声明三个patch。
- Falsifiable predictions:
  - If true: parser可接受actions中两个`apply_patch`。
  - If false: typed schema/parser应有singular patch slot且ordinary union排除patch。
- Diagnostic evidence plan:
  - Prediction or clause under test: carrier parser是否拒绝第二个patch。
  - Signal: typed args定义、validator和schema。
  - Capture method: 静态检查并增加修复前负例fixture。
  - Event name or marker:
    - `multiple_apply_patch_actions_not_allowed`
  - Correlation keys:
    - outer call id
  - Differentiates from:
    - 顶层兄弟patch计数缺口
  - Supports if:
    - validator遍历actions时只检查空名称和control/update_plan。
  - Refutes if:
    - schema/parser已将patch抽成唯一slot。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - 保留carrier validation日志
- Evidence gate: satisfied
- Related evidence:
  - E-005
- Conclusion: confirmed
- Repair design readiness: ready；用户已明确授权执行到J7.4
- Next step: carrier schema/parser改为singular patch continuation
- Blocker:
  - none
- Close reason:
  - not closed

## Evidence E-001: 生产Patch循环在验证完成前写文件
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/apply-patch/src/lib.rs:261`
- Prediction or plan link:
  - H-001生产代码写入顺序
- Matched signal:
  - for hunk循环内直接调用write/remove，update的derive只在到达该hunk时执行
- Correlation keys:
  - `apply_hunks_to_files`
- Raw content:
  ```text
  for hunk in hunks { ... write_file/remove ... }
  ```
- Interpretation: 全部hunk validation未在首个副作用前完成。
- Time: 2026-07-13 02:01

## Evidence E-002: 既有测试固化失败后保留新增文件
- Related hypotheses:
  - H-001
- Direction: supports
- Type: test
- Source: `third_party/codex-cli/codex-rs/apply-patch/tests/suite/tool.rs:261`
- Prediction or plan link:
  - H-001失败后workspace状态
- Matched signal:
  - missing update失败后断言`created.txt == hello`
- Correlation keys:
  - `test_apply_patch_cli_failure_after_partial_success_leaves_changes`
- Raw content:
  ```text
  assert_eq!(fs::read_to_string(&new_file)?, "hello\n");
  ```
- Interpretation: partial write是当前显式契约，不是偶发I/O故障。
- Time: 2026-07-13 02:01

## Evidence E-003: 普通工具segment未经整组预检直接并行执行
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/tools/sequence.rs:32`
- Prediction or plan link:
  - H-002 request-wide manifest缺失
- Matched signal:
  - `sequence_segments`后ordinary segment直接构造futures并`join_all`
- Correlation keys:
  - response call vector
- Raw content:
  ```text
  calls[start..end].iter().cloned().map(...handle_tool_call_for_sequence...)
  join_all(futures).await
  ```
- Interpretation: 同段多个patch在任何一个结果返回前已经全部启动。
- Time: 2026-07-13 02:01

## Evidence E-004: 真实Response产生5个Patch的部分提交
- Related hypotheses:
  - H-002
- Direction: supports
- Type: reproduction
- Source: `target/r5-final-loop-fix-repeat3/subscription-billing-repair/20260713-002149-397/pair-002/left/artifacts/rollout.jsonl`
- Prediction or plan link:
  - H-002真实执行结果
- Matched signal:
  - sequence 34-38为5个apply_patch；sequence 39-43为4 success + 1 failure
- Correlation keys:
  - `call_00_3wDerKgvzWIPSW2IbKhZ8232`至`call_04_UWwY3tI1241DUUVlq3Go1201`
- Raw content:
  ```text
  apply_patch x5 -> true, true, true, true, false
  ```
- Interpretation: 缺失的request预检已在真实Docker样本造成部分状态。
- Time: 2026-07-13 02:01

## Evidence E-005: Carrier Validator未限制Patch身份和数量
- Related hypotheses:
  - H-003
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/tools/handlers/taskspace_control_args.rs:234`
- Prediction or plan link:
  - H-003 parser约束范围
- Matched signal:
  - validator只拒绝空名称、`taskspace_control`和`update_plan`
- Correlation keys:
  - outer taskspace control call id
- Raw content:
  ```text
  if matches!(name.as_str(), "taskspace_control" | "update_plan") { ... }
  ```
- Interpretation: 多个apply_patch在typed parser层合法。
- Time: 2026-07-13 02:01

## Evidence E-006: Provider未执行maxContains计数约束
- Related hypotheses:
  - H-003
- Direction: supports
- Type: experiment
- Source: `target/r5-j7-schema-probe/singular-patch-capability.json`
- Prediction or plan link:
  - J7.0 Option A provider能力门禁
- Matched signal:
  - stable和beta strict均HTTP 200，但requested_two_patches的patch_count均为2
- Correlation keys:
  - schema `r5-j7-singular-patch-provider-capability-v1`
- Raw content:
  ```text
  stable: http=200 patch_count=2
  beta_strict: http=200 patch_count=2
  ```
- Interpretation: `contains/maxContains`不能承担tool contract，必须使用显式singular schema和本地preflight。
- Time: 2026-07-13 02:12

## Evidence E-007: Standard与TaskSpace共享response sequence入口
- Related hypotheses:
  - H-002
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/core/src/session/turn.rs:2683`
- Prediction or plan link:
  - H-002共享修复位置
- Matched signal:
  - provider completed及stream尾部pending calls均进入`execute_response_tool_sequence`
- Correlation keys:
  - response tool call vector
- Raw content:
  ```text
  execute_response_tool_sequence(tool_runtime, pending_tool_calls, cancellation_token)
  ```
- Interpretation: request manifest可以在一个共享入口覆盖顶层与TaskSpace nested声明。
- Time: 2026-07-13 02:12

## Evidence E-008: 文件系统substrate无事务原语
- Related hypotheses:
  - H-001
- Direction: supports
- Type: code-location
- Source: `third_party/codex-cli/codex-rs/exec-server/src/file_system.rs:131`
- Prediction or plan link:
  - J7.0 I/O atomicity能力审计
- Matched signal:
  - trait只有read/write/create/get_metadata/read_directory/remove/copy，无rename/transaction
- Correlation keys:
  - `ExecutorFileSystem`
- Raw content:
  ```text
  read_file, write_file, create_directory, get_metadata, read_directory, remove, copy
  ```
- Interpretation: validation atomicity可实现；跨文件I/O transaction不能宣称，commit failure必须rollback并忠实报告。
- Time: 2026-07-13 02:12

## Hypothesis H-004: Prepare/commit可消除validation partial write
- Status: confirmed
- Parent: P-001
- Claim: 若全部hunk内容计算、路径检查和precondition在commit前完成，则所有validation failure可以零文件副作用；I/O failure可由pre-image执行best-effort rollback并忠实报告残留。
- Layer: fix-validation
- Factor relation: part_of
- Depends on:
  - H-001
- Rationale:
  - J7.0确认底层缺少transaction，但validation与commit可明确分层。
- Falsifiable predictions:
  - If true: valid add/update后接missing/context failure时workspace hash不变；故障注入第二次write/remove后pre-image恢复。
  - If false: 任一validation负例留下文件，或commit错误缺失committed/rollback facts。
- Diagnostic evidence plan:
  - Prediction or clause under test: validation zero side effect与I/O rollback事实完整性。
  - Signal: CLI/scenario workspace断言、fault injection和structured error字段。
  - Capture method: 运行codex-apply-patch全套与codex-core apply_patch focused tests。
  - Event name or marker:
    - `PatchCommitError`
  - Correlation keys:
    - patch operation paths
  - Differentiates from:
    - request-wide multi-patch preflight
  - Supports if:
    - 全部validation/fault tests通过且旧partial fixture变为empty expected。
  - Refutes if:
    - 任一validation失败产生文件副作用或rollback失败被隐藏。
  - Instrumentation status: permanent-observability-candidate
  - Instrumentation lifecycle:
    - J7.4接入prepare/commit事件
- Evidence gate: satisfied
- Related evidence:
  - E-009
- Conclusion: confirmed
- Repair design readiness: complete
- Next step: J7.2/J7.3关闭carrier与request-wide缺口
- Blocker:
  - none
- Close reason:
  - J7.1 verified

## Evidence E-009: J7.1 validation atomicity与rollback门禁通过
- Related hypotheses:
  - H-001
  - H-004
- Direction: supports
- Type: fix-validation
- Source: `codex-apply-patch` tests、`core apply_patch` tests、locked Whale build
- Prediction or plan link:
  - H-004全部预测
- Matched signal:
  - 64 lib + 22 CLI/scenario + 16 core tests passed；write/remove/rollback failure injection passed
- Correlation keys:
  - J7.1
- Raw content:
  ```text
  cargo test -p codex-apply-patch: 86 passed
  cargo test -p codex-core apply_patch --lib: 16 passed
  cargo build -p codex-cli --bin whale --locked: passed
  ```
- Interpretation: H-001所述validation partial write已在shared substrate关闭；跨request多patch仍需J7.2/J7.3。
- Time: 2026-07-13 02:25
