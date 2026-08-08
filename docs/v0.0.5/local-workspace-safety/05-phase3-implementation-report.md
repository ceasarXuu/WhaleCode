# Workspace Bootstrap Phase 3 实施结果报告

- 报告日期：2026-08-09
- 依据计划：`docs/v0.0.5/local-workspace-safety/01-local-workspace-safety-plan.md`
- 检查范围：branch `whalecode-codex`；W7-W12提交`d0af6fd51`、`330bcc949`、`8dfd93a80`、`1fc75e685`
- 结论：W7、W8、W12完成；W9a、W9b完成workspace门禁，但因仓库不存在既有付费运行授权合同而按计划停止，live路径保持fail-closed

## 1. 完成度

按16个计划工作单元等权计分；只有实现和声明验证均存在的单元计100%，部分实现不折算完成。

| 阶段 | 计划单元 | 已验证 | 完成度 | 依据 |
| --- | ---: | ---: | ---: | --- |
| Overall | 16 | 10 | 63% | D0、W1-W8、W12完成，10/16=62.5% |
| Phase 1 | 3 | 3 | 100% | D0、W1、W2 |
| Phase 2 | 4 | 4 | 100% | W3-W6 |
| Phase 3 | 5 | 3 | 60% | W7、W8、W12完成；W9a/W9b阻塞 |
| Phase 4 | 3 | 0 | 0% | W10、W11、W13未开始 |
| Deferred | 1 | 0 | 0% | W14 Windows/PowerShell延期 |

## 2. 模块结果

| 模块 | 实际结果 | 验证 | 状态 |
| --- | --- | --- | --- |
| W7 Linux installer | 强制显式`workspace/user` scope；workspace只写当前XDG binary slot并生成完整attestation；user scope保留显式legacy兼容 | 双临时repo安装到不同slot；缺scope、未bootstrap和越界目录均零目标写入；legacy sentinel不变 | complete |
| W8 cache runner | 删除Python runner的legacy binary默认；从Ready workspace解析并验证binary与attestation，且在合同、API key和ledger操作前失败 | 12项runner测试；workspace entrypoint测试；cache index gate PASS | complete |
| W9a active-prefix | `require_ready`位于输入读取、run root、Docker和请求之前；`plan-only`保留 | workspace失败零目录；live路径在输入和目录前稳定阻断 | blocked on authorization contract |
| W9b provider-wire | 新CLI边界在raw/output和HTTP前调用`require_ready` | workspace失败零文件；HTTP stub未调用；原probe 10项语义测试通过 | blocked on authorization contract |
| W12 reference gate | 阻止新增legacy whale bin、共享Cargo/Bazel override和四个已保护入口丢失preflight token；既有例外有数量上限与原因 | 5项正反fixture；当前仓库PASS | complete |

## 3. 安全停止与授权缺口

计划假定W9可“验证现有运行授权合同”，但源码盘点发现两个入口均没有可复用的账本认领或预算授权合同。仅新增workspace门禁会让入口仍可绕过项目的付费运行规则；自行增加一个字符串参数又无法证明真实用户授权。

因此本阶段采用最小安全行为：

- workspace未Ready时，在任何目录、Docker或HTTP副作用前失败；
- workspace Ready后，真实运行仍以`run_ledger_authorization_unavailable`失败；
- active-prefix的无模型`--plan-only`仍可用；
- 未创建账本记录、未读取API key、未发起模型请求，也未伪造用户预算授权。

解除阻塞需要单独确认统一授权合同，至少定义：planned账本记录的创建者、稳定run id/claim方式、授权证据表达、sample×arm×repeat计数、结算责任以及崩溃恢复。该决策不属于workspace门禁的局部实现，不在本阶段自行扩张。

## 4. 验证证据

| 验证 | 结果 | 说明 |
| --- | --- | --- |
| workspace-safety unittest | 57/57 passed | 包含四项W9零副作用测试与五项W12规则测试 |
| installer integration | passed | 双repo、双XDG slot、user scope及失败路径 |
| cache runner tests | 12/12 passed | 无真实请求 |
| active-prefix tests | 5/5 passed | 无Docker、无模型 |
| provider-wire tests | 10/10 passed | 本地payload分析，无HTTP |
| Python/shell syntax | passed | 相关Python `py_compile`与installer `bash -n` |
| workspace reference gate | PASS | 当前repo零未解释违规 |
| cache-sensitive index gate | PASS | 指纹`1921a127...34ae`未变；最近一次live回归仍失败，不构成基线晋升 |
| Git hygiene | 四个主题提交均已push | 最终文档提交后再次确认clean |

本阶段没有真实Whale Agent run，因此全局运行账本没有新增记录，也不消耗付费运行预算。

## 5. 范围与工程约束

- 新增手写生产代码约390行，低于本阶段500行上限；测试与文档不计入。
- 所有手写生产文件均低于500行；vendor未改动。
- legacy `~/.whale`未迁移、覆盖或删除。
- Windows/PowerShell路径仅作为W12有理由、有上限的allowlist保留，继续归W14。
- 当前真实codex/alpha workspace仍未apply；本机启用归W13，不在本阶段发生。

Phase 3的cycle decision为`retain completed guards, keep paid paths fail-closed`。在统一付费运行授权合同确定前，不应把W9标记完成，也不应通过临时参数恢复live路径。
