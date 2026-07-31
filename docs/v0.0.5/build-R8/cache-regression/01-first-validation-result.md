# 缓存回归门禁首次验证结果

- Date: 2026-07-31
- Subject commit: `2185befc5`
- Result: Gate correctly blocked
- Live samples consumed: 2
- Automatic retries: 0

## 1. 结论

专用脚本成功完成 Standard 与 map-request 各一次真实运行，并从 provider 最终 wire trace 中发现当前版本存在明确
缓存退化。两臂业务验证均通过，缓存 usage 覆盖均为 100%，因此失败不是任务失败、样本不具备多轮请求或遥测
缺失造成的误报。

当前敏感面基线状态已设为 `live_regression_failed`。结果没有晋升为 `live_verified`。pre-commit 继续允许与缓存
无关的修复和证据归档；v0.0.5 non-agent release gate 保持阻断。

## 2. 实际结果

| 指标 | Standard | map-request |
|---|---:|---:|
| 业务结果 | 通过 | 通过 |
| Provider requests | 6 | 7 |
| Request 2+ 数量 | 5 | 6 |
| Request 2+ 缓存命中率 | 96.62% | 35.79% |
| Provider usage 覆盖率 | 100% | 100% |
| Input token | 74,308 | 116,522 |
| Cached input | 61,312 | 44,160 |
| Uncached input | 12,996 | 72,362 |
| Output token | 1,302 | 2,890 |
| Wall time | 13.86 s | 25.38 s |

合计 13 个 provider request、190,830 input token、105,472 cached input、85,358 uncached input 和 4,192
output token。按运行时冻结的 DeepSeek 价格估算为 **USD 0.0134192016**，历时 51 秒。

## 3. 发现能力证据

Standard 的 5 次相邻请求比较全部保持稳定前缀，request 2+ 命中率达到 96.62%。map-request 的 6 次相邻请求
比较均在历史消息位置出现首个差异，稳定前缀保持率为 0%，最终缓存命中率只有 35.79%。这证明最简样本能够
区分健康的连续追加路径和当前 TaskSpace 路径，而不是只检查一个无判别力的总 token 数。

该证据与 R8-I02“同一工具事实被额外包装进入上下文”的已知问题方向一致，但本轮只验收门禁发现能力，不据此
直接关闭根因调查。后续修复仍需逐 request 比较发生变化的消息内容和来源。

## 4. 基础设施失败记录

真实运行前有两次预检失败，均为 `actual_sample_runs=0`、`api_requests=0`：

1. 账本先写入导致整个 worktree 变脏，旧 binary attestation 错把无关账本变化当成 Codex 源码变化。
2. runner 未从项目约定的 `.env.local` 加载 `DEEPSEEK_API_KEY`。

两个缺陷均已修复并增加确定性测试。失败记录保留在全局账本中，没有覆盖或删除。

## 5. 证据

- 结果：`benchmarks/cache-regression/results/WAR-20260731-121102-CACHE-REGRESSION-03B04648.json`
- 账本：`benchmarks/whale-agent-run-ledger.json`
- 运行产物：
  `target/cache-hit-regression/WAR-20260731-121102-CACHE-REGRESSION-03B04648/`
