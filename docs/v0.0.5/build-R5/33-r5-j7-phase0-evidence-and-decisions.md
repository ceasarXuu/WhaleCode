# R5-J7.0 证据冻结与能力决策

- Date: 2026-07-13
- Status: Complete
- Scope: J7.0 trace fixture、provider schema probe、ToolSpec/sequence入口、文件系统原子性能力
- Related: `18-r5-single-patch-carrier-contract-plan.md`、
  `coe/2026-07-13-02-01-r5-request-multi-patch-partial-write.md`

## 1. 阶段结论

J7.0退出门禁通过，生产方向冻结为：

```text
TaskSpace carrier: explicit singular patch continuation
Request boundary: shared ToolSequenceManifest + request_patch_count <= 1
Patch execution: prepare all validation before commit
I/O failure: best-effort rollback + structured facts, no transaction claim
```

不采用`contains/maxContains`。DeepSeek stable和beta strict endpoint虽然都接受该schema，但在user明确要求两个
patch时都返回两个patch，证明目标endpoint没有执行可依赖的`maxContains`约束。本地`JsonSchema`也没有这两个
关键字，扩展serializer只会增加无效复杂度。

## 2. Provider能力探针

脚本：`scripts/taskspace-benchmark/probe-singular-patch-schema.ps1`  
artifact：`target/r5-j7-schema-probe/singular-patch-capability.json`

| Endpoint | Prompt | HTTP | Tool calls | Patch count | Conclusion |
|---|---|---:|---:|---:|---|
| stable non-strict | one patch | 200 | 1 | 1 | schema accepted |
| stable non-strict | request two patches | 200 | 1 | 2 | `maxContains`未约束生成 |
| beta strict | one patch | 200 | 1 | 1 | schema accepted |
| beta strict | request two patches | 200 | 1 | 2 | strict也未执行该计数约束 |

探针只记录HTTP接受性与生成形态，不把一次模型输出当成JSON Schema enforcement证明。生产正确性完全由显式
tool schema和本地preflight保证。

## 3. ToolSpec与Canonical Identity

| Surface | Current representation | J7 decision |
|---|---|---|
| Responses freeform patch | custom tool，raw `input` | canonical name=`apply_patch` |
| Function patch | function tool，`arguments.input` | canonical name=`apply_patch` |
| Chat mapping | custom patch转换为function patch | 不建立第二套计数逻辑 |
| TaskSpace nested | 从同轮visible ToolSpec生成function/custom action | singular slot继续复用原ToolSpec参数schema |

Request manifest只按ToolRouter解析后的canonical identity计数。它不解析reasoning、shell正文或patch内容来猜测
工具身份。

## 4. Shared Sequence入口

Standard与TaskSpace的完整provider tool calls最终都进入
`core/src/tools/sequence.rs::execute_response_tool_sequence`。当前ordinary segment会直接`join_all`，TaskSpace
barrier则先提交state再执行nested calls，因此request manifest必须放在：

```text
execute_response_tool_sequence
  -> build ToolSequenceManifest from all top-level + declared nested identities
  -> validate request_patch_count
  -> only then build segments / commit state / dispatch tools
```

不能把计数放进TaskSpace handler，也不能先执行顶层第一个patch再拒绝第二个。

## 5. Patch原子性边界

现有`apply_hunks_to_files`逐hunk读取、计算和写入。既有测试
`test_apply_patch_cli_failure_after_partial_success_leaves_changes`通过并确认：add成功后，后续missing update失败会
保留新增文件。

`ExecutorFileSystem`只提供read/write/create/remove/copy，没有rename、staging或transaction API。因此J7.1冻结
两层不同承诺：

| Layer | Required guarantee | Decision |
|---|---|---|
| Validation atomicity | parse/path/read/context/target calculation失败前零写入 | 必须实现，硬门禁 |
| Commit rollback | I/O failure后按pre-image尽力恢复 | 必须实现并故障注入验证 |
| Cross-file transaction | 任意系统故障后绝对原子 | 当前substrate不支持，不声明 |

prepare阶段为所有hunk构造完整操作计划并保存必要pre-image；commit阶段顺序执行。commit失败返回：

```text
committed_paths
pending_paths
rollback_attempted
rollback_restored_paths
rollback_failed_paths
rollback_status
```

rollback失败、目录残留或metadata无法恢复都必须忠实报告，不能返回模糊`patch failed`或伪装workspace未变。

## 6. 验证

| Gate | Result |
|---|---|
| J6.7 two-round adversarial review | passed；J7 unblocked |
| partial-write baseline test | passed；旧缺陷稳定复现 |
| provider stable schema probe | accepted；2 patch仍生成 |
| provider beta strict probe | accepted；2 patch仍生成 |
| shared sequence audit | Standard/TaskSpace同一入口 confirmed |
| filesystem capability audit | no rename/transaction confirmed |

## 7. 下一门禁

J7.1必须先完成shared `apply_patch` prepare/commit重构及validation零副作用测试。J7.1未通过前，不修改
TaskSpace carrier schema，不实施request-wide reject，避免把Agent压成单patch后仍暴露单patch内部partial write。
