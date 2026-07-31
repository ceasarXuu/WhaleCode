# CR21.1 Final-Wire 变化报告结果

- Date: 2026-08-01
- Status: completed
- Commit: `e35cf681b`
- API usage: 无真实 Whale Agent run，无 DeepSeek API 请求

## 1. 产品结果

免费缓存合同现在会对受保护的生产 final-wire 快照做规范化 JSON 精确比较，并明确返回三种状态：

- `unchanged`：所有受保护场景均成功产出，且字段、值和数组顺序完全一致；
- `changed`：成功产出但至少一个场景发生变化，同时给出首个 JSON Pointer 差异和新旧 payload digest；
- `uncomparable`：基线或候选缺失、出现额外候选、快照无效或根对象不合法，保持阻断而不猜测结论。

比较器不忽略未知字段，不分析日志文本，也不读取 Insta 的 `.snap.new` 临时文件。候选由现有生产 Session、Responses
serializer 和测试中的同一份 JSON 值直接写入临时目录，没有建立第二套请求构造或序列化路径。报告只陈述请求语义
是否变化或不可比较，不判断变化是否符合产品预期，也不选择后续 benchmark。

## 2. 覆盖与接线

11 个受保护场景均接入结构化报告：

| 命令 | 场景数 | 覆盖对象 | 最终状态 |
|---|---:|---|---|
| `final_wire_matrix` | 10 | Standard、三种 TaskSpace、权限、Skill、Apps、Plugin、MCP、压缩 | `unchanged` |
| `tool_wire_contract` | 1 | TaskSpace control 与普通 Tool 的生产 wire schema | `unchanged` |

实现时发现 Tool wire 快照不在原 `cache_payload_` 测试过滤器内，因此新增一条精确的本地合同命令。曾尝试扩大为
`cache_` 过滤器，但它会额外运行 4 个无关的模型 cache TTL 测试；该方案未保留，避免扩大验证范围和混淆失败归因。

## 3. 验证证据

- Python 缓存控制面测试：`75 passed`；
- Ruff lint 与 format：通过；
- Rust workspace format：通过，仅有既有 nightly 配置提示；
- 最终免费生产矩阵：7 条命令全部通过，总耗时 `5157 ms`；
- final-wire 报告：11 个场景，`changed=0`，`uncomparable=0`；
- 本地结构化证据：`/tmp/r8-cr21-1-final-free-validation.json`。

反例测试覆盖 JSON 格式噪声不报警、嵌套字段变化及未知字段精确报警、候选缺失和额外候选均判为不可比较。
提交时现有发布门仍报告“待验证政策变更；发布保持阻断”，说明 CR21.1 没有越权恢复发布。

## 4. 后续边界

CR21.1 已完成变化发现和判别。下一单元 CR21.2 只接收人明确选择的 model、sample、arm、repeat、停止条件和
重试上限，机械生成零 API、零账本副作用的预算提案；它不得推荐测试配置或把代表性 smoke 扩大解释为其他路径。
