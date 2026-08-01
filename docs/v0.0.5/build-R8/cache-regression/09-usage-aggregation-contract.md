# CR-11：usage 聚合一致性合同

- Verified: 2026-07-31
- Status: completed
- Code evidence: `c008cab58`
- Contract version: `whalecode-provider-usage-v1`
- Real Whale Agent runs: 0

## 1. 已实现行为

Python 报告层通过 `scripts/cache-regression/cache_usage_contract.py` 直接读取 CR-10 的同一 provider fixture，不再维护
第二份 hit/miss 数字。合同负责：

- 校验所有 token 为非负整数；
- 校验 cached input 不超过 input；
- 分别汇总全程 token 与 request 2+ cached/uncached token；
- 从 request 2+ token 重算并核对 summary hit rate；
- 校验 request 2+ 明细不超过全程总量；
- 对 usage 缺失、类型错误、明细缺失或恒等式矛盾明确失败；
- 在每个 arm 中记录 `provider_usage_contract_version`。

晋升器将合同版本和 request 2+ cached/uncached token 纳入 artifact 重算键，手工篡改不能通过晋升。

## 2. 跨语言证据

Rust Chat Completions、Rust Responses API 和 Python 读取同一个
`provider_usage_contract.json`。两个 wire 的 `hit`、`miss`、`missing_details` 归一化结果逐值相同；
`invalid_cached_type` 在 Python 中统一标记为不可比较。

确定性聚合用例把 miss 作为 request 1、hit 作为 request 2，得到：

| 指标 | 结果 |
|---|---:|
| provider requests | 2 |
| total input | 190 |
| total cached input | 80 |
| total output | 30 |
| request 2+ cached | 80 |
| request 2+ uncached | 20 |
| request 2+ hit rate | 0.8 |

## 3. 验证

```bash
python3 -m unittest discover -s scripts/cache-regression -p 'test_*.py'
```

结果：`50 passed; 0 failed`。未调用真实 provider。

Phase B 已满足：生产 final-wire 有唯一捕获边界、证据可复算、受保护字段变化可发现、Tool wire 可见、usage 解码与
报告口径一致。下一步进入 Phase C 的逐类免费场景，不提前接入发布门。
