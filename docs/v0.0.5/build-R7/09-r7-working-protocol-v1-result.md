# R7 TaskSpace 核心工作协议 v1.0.0 结果

- 日期：2026-07-19
- 协议版本：`1.0.0`
- 规则哈希：`d79723097841f2555c981663fb28bdca9099bbf7fd32246d81c609e21bd35efa`
- 实现提交：`daf6b4787`
- 状态：已评估，不接受为默认版本
- 机器结果：`benchmarks/taskspace/r7/working-protocol-v1.0.0-result.json`

## 1. 结论

核心工作协议应继续保留，但 `v1.0.0` 不能直接定版。

正向信号是：Phase D 中 `map-request` 复杂样本只初始化 Map、最终未闭合；本轮 Agent 主动完成全部
生命周期动作，Root、Finish 和 3 个 Work 节点均闭合，样本 solved。协议身份在 TaskSpace 的 45/45
个 provider request 中均为 `1.0.0` 且哈希匹配，Standard 的 17/17 个请求均无协议，证明交付边界正确。

负向信号是：简单、复杂样本都产生 3 次单独的非终态 `complete`，下一次请求才 `bind + continuation`
或 `finish_end`。当前工具序列本来支持 Agent 在同一 provider response 中声明有序 sibling calls；协议只说
“完成、选择并绑定”，没有明确说明应在同一 response 中表达，造成固定流程请求没有被合并。

## 2. 同期 Docker 对照

每个样本只执行 1 次，属于工程诊断，不是稳定性结论。

| 样本 / 指标 | Current Standard | TaskSpace map-request + v1.0.0 |
|---|---:|---:|
| simple 结果 | solved | solved |
| simple provider request | 6 | 24 |
| simple ordinary / control | 8 / 0 | 25 / 7 |
| simple wall time | 14.08s | 47.58s |
| simple input / uncached input | 39,694 / 1,422 | 320,269 / 14,733 |
| simple request 2+ cache hit | 96.13% | 97.75% |
| complex 结果 | solved | solved |
| complex provider request | 11 | 21 |
| complex ordinary / control | 17 / 0 | 33 / 7 |
| complex wall time | 33.68s | 65.13s |
| complex input / uncached input | 108,252 / 7,132 | 352,779 / 16,651 |
| complex request 2+ cache hit | 93.22% | 96.90% |

TaskSpace 的 message prefix preservation 均为 100%。协议约 396 estimated tokens/request，位于固定前缀，
未破坏后续缓存；总 input 放大主要来自 request 数量和每次重复携带的固定 tools schema，而不是协议文本本身
未命中缓存。

## 3. 请求放大拆解

两组 Map 均为 `Root -> Work -> Work -> Work -> Finish`，生命周期 control 固定为：

```text
initialize_map x1
complete Work x3
bind next Work x2
finish_end x1
```

其中 3 个 `complete` 都独占一次 provider response，随后才发下一次 lifecycle/ordinary action。这是协议表达缺口，
不是状态机必须付出的成本，也不应由 Runtime 自动合并。正确方向是让 Agent 通过现有工具 schema 在同一 response
中显式提交 `complete -> bind + continuation` 或 `complete -> finish_end`。

其余放大不能归因给协议：simple 的首次普通工具仍被 bootstrap gate 拒绝，且一次错误 patch 引发多轮恢复；
complex 多次生成上下文不匹配的 patch。两者都需要更多重复样本观察，不能用单次结果要求 Runtime 增加语义约束。

## 4. 版本决策

`v1.0.0` 永久保留其版本、哈希、提交和运行产物，不覆盖、不改名。下一候选为 `v1.0.1`，只改变一项：

1. 明确同一 provider response 中的有序 lifecycle sibling calls；
2. 同时压缩措辞，减少固定协议成本；
3. 不修改 Runtime、Map、projection policy 或 `taskspace_control` schema；
4. 用同一 simple/complex + contemporaneous Standard 各 1 次重新验证。
