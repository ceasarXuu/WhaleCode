# I03 多 outer 可恢复拒绝结果

- Date: 2026-08-20
- Implementation commit: `8ae4f2759`
- Scope: I03 response recovery + I07 observation
- Source evidence: `WAR-20260820-214337-R8-MAP-REQUEST-R3`
- Status: offline verified; natural online recovery pending

## 1. 修复结果

同一 Provider response 出现多个 `taskspace_exec` 时，合同仍判定整批非法。Runtime 不执行、选择、合并或重排其中任何
Map 或 client Tool 动作，但不再把该错误升级为 session fatal：

1. response scope 把“多个 outer Exec”分类为可恢复的 response-level 拒绝；
2. turn 协调器先排空已经建立的原始 Tool futures；
3. 每个原始 `call_id` 都沿 Codex 原生 Tool output 路径得到同一合同错误；
4. 两条拒绝进入正式 conversation history，下一请求由 Agent 自行纠正；
5. Provider-hosted 已完成事实仍按原机械路径记录，不被此次 client 合同错误丢弃。

可恢复只表示“把失败忠实返回给 Agent 并继续会话”，不表示请求合法。`claim_response` 对两个 outer 仍均拒绝，Map 和
client Tool 副作用保持为零。

## 2. Observer 修复

拒绝文本使用既有 `invalid top-level contract` 分类，两个 outer 分别发出
`response_cardinality_rejected` trace。离线双 outer fixture 证明：

| 指标 | 结果 |
|---|---:|
| `exec_count` | 2 |
| `rejected_call_count` | 2 |
| `rejected_contract_call_count` | 2 |
| `correlated_request_count` | 1 |
| `correlated_outer_call_count` | 2 |
| `exec_result_missing` | 0 |
| availability | `measured` |

Observer 没有为该错误建立专用平行统计协议，而是消费 Runtime 已产生的原生逐 call output 和现有合同分类。

## 3. 验证

- `cargo test -p codex-core tools::taskspace_exec -- --nocapture`：85 passed；
- `cargo check -p codex-core`：passed；
- `pwsh -NoProfile -File scripts/taskspace-benchmark/test-taskspace-exec-observation.ps1`：passed；
- `git diff --check`：passed。

未启动真实 Whale Agent run，因此不能宣称目标模型在收到两条拒绝后已经在线纠正，也不能据此降低多 outer 的自然复发率。
下一次获批自然样本若再次命中该分支，应验收下一请求继续、两个输出身份完整、零副作用和 observer `measured`。

## 4. 全局约束检查

- Runtime 只守一响应一个 outer 的硬合同，不替 Agent 决定合法动作；
- 失败语义通过原 Tool output 忠实进入上下文，没有 system/developer 再包装；
- client Tool schema、合法序列、Map 状态和节点归属均未改变；
- 没有兼容旧协议、自动合并或特殊执行分支；
- Standard 路径不创建 TaskSpace response scope，因此行为不变。
