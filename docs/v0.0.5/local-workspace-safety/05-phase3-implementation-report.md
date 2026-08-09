# Workspace Bootstrap Phase 3 实施结果报告

- 报告日期：2026-08-09
- 依据计划：`docs/v0.0.5/local-workspace-safety/01-local-workspace-safety-plan.md`
- 检查范围：branch `whalecode-codex`；W7-W12提交`d0af6fd51`、`330bcc949`、`8dfd93a80`、`1fc75e685`、边界修正`f030e335e`
- 结论：W7、W8、W9a、W9b、W12全部完成；workspace门禁属于runner基础安全，账本与预算批准属于开发流程规范，不进入产品运行逻辑

## 1. 完成度

按16个计划工作单元等权计分；只有实现和声明验证均存在的单元计100%，部分实现不折算完成。

| 阶段 | 计划单元 | 已验证 | 完成度 | 依据 |
| --- | ---: | ---: | ---: | --- |
| Overall | 16 | 12 | 75% | D0、W1-W9b、W12完成，12/16=75% |
| Phase 1 | 3 | 3 | 100% | D0、W1、W2 |
| Phase 2 | 4 | 4 | 100% | W3-W6 |
| Phase 3 | 5 | 5 | 100% | W7、W8、W9a、W9b、W12完成 |
| Phase 4 | 3 | 0 | 0% | W10、W11、W13未开始 |
| Deferred | 1 | 0 | 0% | W14 Windows/PowerShell延期 |

## 2. 模块结果

| 模块 | 实际结果 | 验证 | 状态 |
| --- | --- | --- | --- |
| W7 Linux installer | 强制显式`workspace/user` scope；workspace只写当前XDG binary slot并生成完整attestation；user scope保留显式legacy兼容 | 双临时repo安装到不同slot；缺scope、未bootstrap和越界目录均零目标写入；legacy sentinel不变 | complete |
| W8 cache runner | 删除Python runner的legacy binary默认；从Ready workspace解析并验证binary与attestation，且在合同、API key和ledger操作前失败 | 12项runner测试；workspace entrypoint测试；cache index gate PASS | complete |
| W9a active-prefix | `require_ready`位于输入读取、run root、Docker和请求之前；`plan-only`与既有live流程保留 | workspace失败零目录；Ready后继续输入校验，不增加授权协议 | complete |
| W9b provider-wire | 新CLI边界在raw/output和HTTP前调用`require_ready`，随后委托既有probe | workspace失败零文件零请求；Ready fixture验证委托；原probe 10项语义测试通过 | complete |
| W12 reference gate | 阻止新增legacy whale bin、共享Cargo/Bazel override和四个已保护入口丢失preflight token；既有例外有数量上限与原因 | 5项正反fixture；当前仓库PASS | complete |

## 3. 开发流程与产品逻辑边界

初版实现把AGENTS中的真实运行账本与预算要求误判成runner必须执行的本地授权协议，因此临时加入了`run_ledger_authorization_unavailable`硬阻断。用户确认该要求仅用于开发期间的Agent/开发者流程治理，不是产品逻辑要求；该阻断已移除。

最终边界为：

- workspace未Ready时，在任何目录、Docker或HTTP副作用前失败；
- workspace Ready后，runner继续执行原有plan/live逻辑，不要求产品级授权token；
- Agent和开发者在启动真实运行前，仍必须按AGENTS登记账本并取得适用的预算批准；
- 本阶段测试均为无模型fixture，未读取真实API key、未发起模型请求。

不新增统一授权服务、不可伪造token或runner内部预算状态机；如果未来需要自动化开发流程，应另立开发工具计划，不耦合产品运行路径。

## 4. 验证证据

| 验证 | 结果 | 说明 |
| --- | --- | --- |
| workspace-safety unittest | 57/57 passed | 包含四项W9边界测试与五项W12规则测试 |
| installer integration | passed | 双repo、双XDG slot、user scope及失败路径 |
| cache runner tests | 12/12 passed | 无真实请求 |
| active-prefix tests | 5/5 passed | 无Docker、无模型 |
| provider-wire tests | 10/10 passed | 本地payload分析，无HTTP |
| Python/shell syntax | passed | 相关Python `py_compile`与installer `bash -n` |
| workspace reference gate | PASS | 当前repo零未解释违规 |
| cache-sensitive index gate | PASS | 指纹`1921a127...34ae`未变；最近一次live回归仍失败，不构成基线晋升 |
| Git hygiene | 相关代码主题均已push | 最终文档提交后再次确认clean |

本阶段没有真实Whale Agent run，因此全局运行账本没有新增记录，也不消耗付费运行预算。

## 5. 范围与工程约束

- 新增手写生产代码约390行，低于本阶段500行上限；测试与文档不计入。
- 所有手写生产文件均低于500行；vendor未改动。
- legacy `~/.whale`未迁移、覆盖或删除。
- Windows/PowerShell路径仅作为W12有理由、有上限的allowlist保留，继续归W14。
- 当前真实codex/alpha workspace仍未apply；本机启用归W13，不在本阶段发生。

Phase 3的cycle decision为`retain workspace guards, keep development governance out of product logic`。W9已收口；任何真实运行仍需在命令外遵守AGENTS账本与预算规范。
