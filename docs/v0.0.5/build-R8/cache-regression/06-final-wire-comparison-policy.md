# CR-08：final-wire 差异分类合同

- Verified: 2026-07-31
- Status: completed
- Code evidence: `2dc401d50`
- Real Whale Agent runs: 0

## 1. 判定原则

缓存相关变化以 provider 实际接收的结构化请求语义和路由身份为准：

- `messages` 的角色、顺序和内容精确保护；
- `tools` 的顺序、描述和参数 schema 精确保护；
- `tool_choice`、`model` 精确保护；
- `provider_id`、`wire_api`、`endpoint_path` 精确保护；
- 所有未知 body 字段默认保护；
- 数组顺序和字符串内容不得归一化；
- 当前没有任何忽略字段，新增忽略项必须升级 policy schema 并重新审查。

原始 body SHA 是证据完整性信息。若 SHA 变化但完整 JSON 值与 provider 身份相同，结果记录
`raw_only_change=true`，不把 JSON 空白或对象键序变化误报为缓存语义变化。字段、字符串或数组顺序发生变化时仍会
阻断。

## 2. 工程对象

| 对象 | 职责 |
|---|---|
| `benchmarks/cache-regression/final-wire-comparison-policy.json` | 版本化保存保护面和默认政策 |
| `scripts/cache-regression/cache_payload_contract.py` | 校验政策并返回首个 body 差异、身份差异和原始字节状态 |
| `scripts/cache-regression/test_cache_payload_contract.py` | 对每类受保护变化执行 mutation test |

比较器对完整 `structured_body` 做递归精确比较，不只抽取四个已知字段。因此新增 provider 字段不会落入盲区。
`required_body_pointers` 只负责拒绝残缺证据，不限制完整比较范围。

## 3. 验证结果

新合同测试共 10 个，覆盖：

- 相同证据；
- 仅原始字节变化；
- 消息角色、顺序、内容；
- Tool schema 和顺序；
- `tool_choice`、模型和未知字段；
- 三项 provider 身份；
- body 或 provider 身份缺失；
- 静默删除保护面或新增忽略字段。

```bash
python3 -m unittest discover -s scripts/cache-regression -p 'test_*.py' -v
```

完整缓存门禁离线测试结果：`45 passed; 0 failed`。提交时 pre-commit 将其识别为待验证政策变更，并保持发布阻断。

## 4. 边界

CR-08 只定义比较语义，尚未把真实生产 Tool schema 场景接入合同，也未替换 v1 源码指纹门禁。CR-I08 仍需
CR-20 的门禁编排才能关闭；CR-I04、CR-I05 仍需后续生产场景覆盖。
