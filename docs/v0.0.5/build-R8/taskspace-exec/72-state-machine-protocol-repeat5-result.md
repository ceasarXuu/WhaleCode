# 状态机协议五轮真实运行结果

- Date: 2026-08-17
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arm: `map-request`
- Repeat: 5 个独立 `repeat=1`，并行执行
- Subject commit: `7a798abdb`
- Binary SHA-256: `17a3dbb0c5c91f7d55e76f3b825be71470ca862f6a2c7a6a2b1d1bb06776f11a`
- Ledger: `WAR-20260817-025312-STATE-MACHINE-R5`

## 1. 验证对象

相对上一轮 affected-state 反馈版本，本轮把 `taskspace_exec.description` 中分散的生命周期说明收敛为唯一完整状态机协议，
并修复三个 canonical 示例之间的断链。Runtime 状态转换、反馈、合法序列 schema 和普通 Tool 执行没有改变。

协议明确四种状态、三类触发者、唯一合法显式转换、Map update 先于 Tool dispatch，以及同一 Map update 不能把进入序列时
仍为 Waiting 的子节点直接改出 Waiting。

## 2. 逐轮结果

| Run | 业务/Map | Requests | Input | Cached | Uncached | Output | Agent wall | Waiting 误选 | 其他拒绝 |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---|
| 1 | passed | 8 | 121,069 | 99,072 | 21,997 | 1,836 | 19.416s | 1 | JSON syntax 1 |
| 2 | passed | 10 | 156,870 | 140,928 | 15,942 | 2,395 | 26.361s | 0 | TransitionInvalid 2 |
| 3 | passed | 8 | 127,547 | 101,504 | 26,043 | 2,787 | 27.306s | 0 | JSON syntax 1 |
| 4 | passed | 8 | 127,167 | 102,016 | 25,151 | 2,862 | 25.966s | 0 | TransitionInvalid 1 |
| 5 | passed | 7 | 106,844 | 83,968 | 22,876 | 2,052 | 17.627s | 1 | none |
| **Total** | **5/5** | **41** | **639,497** | **527,488** | **112,009** | **11,932** | **116.676s** | **2/5** | **5 rejects** |
| **Mean** | - | **8.2** | **127,899.4** | **105,497.6** | **22,401.8** | **2,386.4** | **23.335s** | - | - |
| **Median** | - | **8** | **127,167** | **101,504** | **22,876** | **2,395** | **25.966s** | - | - |

全量缓存命中率为 `82.48%`，Request 2+ 加权命中率为 `87.46%`。按冻结价格估算费用为 `0.14642276 CNY`。

## 3. 状态机行为

五轮都正确初始化同一个线性 Map：

```text
root -> inspect -> fix -> verify -> finish
```

每轮均为 5 nodes、4 edges，最终 Root、Work、Finish 全部闭合。状态异常为：

- Run 1、5 在 patch 成功且反馈明确 `fix=in_flight`、`verify=waiting` 后，仍先提交 `work@verify`；Waiting 误选为 `2/5`，
  与上一轮 `2/5` 相同。
- Run 2、4 尝试在完成 `inspect` 的同一个 Map update 中把原本 Waiting 的 `fix` 显式改为 InFlight，触发 2 次
  `TransitionInvalid`；Run 2 又对同样仍为 Waiting 的 `verify` 重复一次。
- Run 2 在 `fix` 已 Ready 后仍显式 patch 为 InFlight 并同时对它执行 Tool。Runtime 接受该合法但冗余转换；这违反
  description 中“不要同时 patch owner 为 InFlight”的效率协议，却没有违反 canonical 状态机硬约束。
- Run 1、3 各有一次独立 JSON syntax 错误，与节点状态协议没有直接因果证据。

## 4. 结论

完整状态机协议已正确进入 Agent 可见的 Tool description，连续示例也形成可执行路径；这两项工程修复成立。但真实行为没有证明
Agent 已稳定掌握状态机：Waiting 误选频率没有下降，且 2/5 runs 仍违反协议中最明确的 same-update Waiting 边界。

相对上一轮，requests 从 44 降到 41，input 从 676,592 降到 639,497，拒绝从 11 降到 7；样本只有五轮且 JSON 错误随机性
明显，不能把成本下降归因于状态机协议。与此同时 uncached input 从 80,240 升到 112,009，真实费用反而更高，也不能宣称成本收益。

因此当前结论是：**状态机协议是必要的信息完整性修复，但不是 Waiting frontier 的充分行为修复。** 下一步应分析为何 Agent 在已经
看到协议和精确状态反馈后仍选择不合法序列，而不是继续增加同义状态说明。

## 5. 证据

- `target/r8-state-machine-contract/repeat5-{1..5}/single-file-fast-fix/20260817-025457-*`
- 每个 root 均包含 `request-summary.json`、`provider-cache-trace-summary.json`、`rollout.jsonl`、Map、公开验证和隐藏 oracle 证据。
