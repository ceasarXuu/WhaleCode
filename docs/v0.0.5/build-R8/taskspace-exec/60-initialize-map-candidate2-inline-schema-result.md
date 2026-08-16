# initialize_map 候选 2：内联 object schema 实验

- Date: 2026-08-16
- Subject implementation: `847da1c37`
- Candidate: 仅把 `initialize_map` 从 `$ref` 改为同合同的就地 object schema
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arm: `map-request`
- Effective repeats: 5 个独立 `repeat=1`
- Retry: 0

## 1. 变量边界

候选 2 保持 `initialize_map` 的字段、required、additionalProperties 和 Runtime 解码逻辑不变。唯一变化是 Provider 不再跨 `$ref` 查找 `initialize_map_input`；同一 object schema 直接出现在 `initialize_and_work.properties.initialize_map`，不再保留无消费者定义。

相对原始基线，生产代码只修改 `sequence_schema.rs`。聚焦测试确认：

1. `initialize_map.type == object`；
2. `initialize_map` 不含 `$ref`；
3. `$defs.initialize_map_input` 已删除；
4. 八种合法序列仍全部解码，旧 wire 仍被拒绝。

## 2. 五轮结果

| Run | 首次 `initialize_map` | Agent | External | Requests | Input | Cached | Uncached | Output | Wall |
|---:|---|---|---|---:|---:|---:|---:|---:|---:|
| 1 | object | complete | passed | 9 | 137,157 | 121,600 | 15,557 | 2,653 | 21.925s |
| 2 | object | complete | passed | 8 | 115,542 | 107,008 | 8,534 | 2,356 | 19.822s |
| 3 | object | complete | passed | 7 | 102,901 | 94,592 | 8,309 | 2,142 | 18.445s |
| 4 | object | complete | passed | 6 | 87,575 | 82,304 | 5,271 | 2,142 | 18.222s |
| 5 | object | complete | passed | 10 | 149,570 | 140,672 | 8,898 | 2,605 | 24.388s |
| Total | object 5 / string 0 | complete 5/5 | passed 5/5 | 40 | 592,745 | 546,176 | 46,569 | 11,898 | 102.802s |

- Request 2+ 加权缓存命中率：`92.90%`。
- Tool section：基线 `25,001 bytes`，候选 2 `24,938 bytes`，减少 `63 bytes`。
- 估算费用：`0.08128852 CNY`。
- 没有类型拒绝、顶层 client Tool 逃逸、自动重试或超时。

## 3. 对比

| 版本 | 首发 object | 首发 string | External passed | Requests | Input | Request 2+ cache |
|---|---:|---:|---:|---:|---:|---:|
| 既有基线 | 4/5 | 1/5 | 5/5 | 43 | 666,392 | 92.41% |
| 候选 1：反馈增强 | 5/5 | 0/5 | 4/5 | 34 | 502,795 | 92.18% |
| 候选 2：schema 内联 | 5/5 | 0/5 | 5/5 | 40 | 592,745 | 92.90% |

候选 2 与候选 1 都是 0/5 string，因此当前 5 轮样本不能把候选 2 的结果与自然随机波动区分开。它与基线相比没有可见成本或缓存回归，但不能据此宣称 `$ref` 已被坐实为首发错误根因。

## 4. 结论

1. 候选 2 是结构合法、范围很小且无明显回归的 schema 表达候选。
2. 五轮结果与“减少嵌套边界有帮助”一致，但因未改 schema 的候选 1 同样 0/5，因果证据不足。
3. 候选 2 只减少 63 bytes，收益若存在应来自表达边界，不是显著降低 token 负担。
4. H-003 继续保持 unverified；不应把候选 2 直接升级为正式根因修复。

## 5. 证据

- Run roots: `target/r8-initialize-map-candidates/candidate2-inline-{1..5}`
- Ledger: `WAR-20260816-184324-INIT-MAP-CANDIDATE2`
- Capability identity: `a95be2ff3edf5911780794843ddee89f4348358e206f69227f675d0cc041ef11`
- First-request Tool hash: `848829796c7fb90ba7b0f48d0c21784459cb0c5d1c8e7f23c597f4a96ca825bf`
