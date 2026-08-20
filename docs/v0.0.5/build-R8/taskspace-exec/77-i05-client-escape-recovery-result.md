# I05 顶层 Client Tool 逃逸恢复结果

- Date: 2026-08-18
- Commit: `e596d2f27`
- Status: 工程修复完成；真实正常路径无回归，逃逸恢复分支未自然触发，保持 verifying
- API usage: 关联账本 `WAR-20260818-013746-R8-I05-I07-ACCEPT-R3`

## 1. 产品问题

TaskSpace 已经能在副作用前拒绝 Agent 错误生成的顶层 client Tool，但此前把整个响应直接升级为 Fatal。Agent 看不到一条
可继续使用、与原调用配对的失败结果，因此无法在下一请求自行改正。

## 2. 修复边界

- 在原生 Tool 参数解析和 dispatch 前识别 TaskSpace 顶层 client Tool 逃逸；
- 保留 Agent 原始调用，并以相同 `call_id` 写入一次准确失败结果，明确该 Tool 未执行；
- 把该已知零副作用协议错误降为可继续请求，不自动包装 Tool、不推断节点、不执行错误动作；
- 同响应已完成的 Provider-hosted action 仍按原始事实机械保存；
- 多 Exec、缺失 Map 快照、响应身份不完整和无法可靠配对的特殊 Tool 继续 Fatal。

Standard 模式、合法 `taskspace_exec`、原生 client Tool schema 和 Map 状态机均未改变。

## 3. 验证

| 证据 | 结果 |
|---|---|
| malformed `exec_command(arguments={})` 顶层逃逸 | 在参数解析前返回同 `call_id` 失败反馈，无 Tool future |
| 顶层 client handler 计数 | 0 次执行 |
| client 逃逸 + 两个 Exec | 仍返回多 Exec 完整性错误，不被恢复分类遮蔽 |
| client 逃逸 + 已完成 Web Search | Hosted action 保留，逃逸仍可纠正 |
| TaskSpace Exec 完整单元集 | 77/77 passed |
| `cargo fmt --check` | passed |
| 缓存敏感面门禁 | 免费 final-wire 验证通过；未修改 Agent-visible Tool 合同 |

以上只证明 Runtime 恢复链正确。真实验收仍需确认目标模型收到配对反馈后会继续使用 `taskspace_exec` 完成任务，且不会产生
新的重复请求或上下文异常。

## 4. 真实验收补充

2026-08-18 的 `standard / map-request × repeat=3` 共 6 次运行全部完成业务、公开测试和隐藏 oracle；3 次 TaskSpace
均正确闭合 Map，没有 Fatal、顶层 client Tool 逃逸或反馈重复。该结果证明正常生产路径无回归，但没有自然触发逃逸，
因此不能替代“Agent 收到配对失败后继续完成任务”的在线分支证据。完整数据见
[`78-i05-i07-repeat3-acceptance-result.md`](78-i05-i07-repeat3-acceptance-result.md)。
