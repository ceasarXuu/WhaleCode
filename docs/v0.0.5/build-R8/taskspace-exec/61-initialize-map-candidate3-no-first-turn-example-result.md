# initialize_map 候选 3：移除首次初始化示例实验

- Date: 2026-08-16
- Subject implementation: `035ffc23f`
- Candidate: 恢复原始 `$ref` schema，仅移除 `taskspace_exec` 描述中的首次初始化完整 JSON 示例
- Model: `deepseek-v4-flash`
- Sample: `single-file-fast-fix`
- Arm: `map-request`
- Effective repeats: 5 个独立 `repeat=1`
- Retry: 0

## 1. 变量边界

候选 3 没有修改 Tool schema、Map 数据模型、Runtime 解码、合法序列或拒绝反馈。唯一变化是 Tool 描述不再携带完整的首次 `initialize_and_work` JSON 示例；handoff、read 和 finish 示例继续保留。

该候选检验完整示例是否诱发 Agent 把 `initialize_map` 的 object 再编码成 JSON string。验收同时检查类型和完整序列，避免只减少一种错误却引入另一种错误。

## 2. 五轮结果

| Run | 首次 `initialize_map` | 首次序列 | Agent | External | Requests | Input | Cached | Uncached | Output | Wall |
|---:|---|---|---|---|---:|---:|---:|---:|---:|---:|
| 1 | object | 缺少 work，被拒绝 | complete | passed | 7 | 97,717 | 92,032 | 5,685 | 1,822 | 15.852s |
| 2 | object | 合法 | complete | passed | 7 | 101,922 | 87,424 | 14,498 | 2,112 | 18.940s |
| 3 | object | 缺少 work，被拒绝 | complete | passed | 8 | 118,958 | 103,040 | 15,918 | 3,147 | 24.620s |
| 4 | object | 合法 | complete | passed | 8 | 118,990 | 110,720 | 8,270 | 2,847 | 22.760s |
| 5 | object | 缺少 `type` 和 work，被拒绝 | complete | passed | 9 | 135,355 | 119,424 | 15,931 | 3,492 | 27.302s |
| Total | object 5 / string 0 | 合法 2/5 | complete 5/5 | passed 5/5 | 39 | 572,942 | 512,640 | 60,302 | 13,420 | 109.474s |

- Request 2+ 加权缓存命中率：`92.95%`。
- 估算费用：`0.0973948 CNY`。
- 3 次非法首发都在下一次请求恢复，未造成 Map 副作用。

## 3. 结论

1. 候选 3 没有出现 `initialize_map` string，但候选 1、2 同样是 0/5，不能把类型结果归因于移除示例。
2. 移除示例后，完整合法的首次初始化序列只有 2/5；另外 3 次需要拒绝和补发。该回归直接对应被移除示例所表达的“初始化必须同时开始工作”。
3. 因此完整示例不是可以独立删除的冗余文本。它可能与 schema 重复，但当前承担了合法序列的操作示范。
4. 候选 3 判定失败，不进入生产基线；也不能作为 H-003 的支持证据。

## 4. 证据

- Run roots: `target/r8-initialize-map-candidates/candidate3-no-example-{1..5}`
- Ledger: `WAR-20260816-185310-INIT-MAP-CANDIDATE3`
- Capability identity: `88043c22fda58510800793438d4557b32abdb0f0560672bc1fac7bd525ff740e`
- First-request Tool hash: `8862dfcb5709c486e7ceae71bf1fde62f4f58351ea61627222115639a0088114`
