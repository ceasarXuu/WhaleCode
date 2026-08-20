# I02 Tool 事实单次表达关闭结算

- Date: 2026-08-18
- Status: closed
- Issue: R8-I02
- Runtime change: none
- Paid run: none

## 1. 产品验收条件

1. 一个 `taskspace_exec` 调用只向 Agent 返回一个同 `call_id` 的 outer Tool 反馈。
2. client Tool 原生结果只作为该 outer 反馈的结构化字段出现，不再建立 system/developer 高优先级副本。
3. 成功、拒绝和内部失败都遵守同一单次表达原则。

## 2. 关闭证据

- 当前 handler 只构造一个 `TaskSpaceExecResult` 并返回一个 `FunctionToolOutput`；client 结果只进入
  `client_results[]`。
- stream 转换将一个 `ResponseInputItem::FunctionCallOutput` 原样转换为一个同 ID 的历史项，没有 TaskSpace 专用复制路径。
- 旧高优先级 carrier 和 TaskSpace Event Store 已从生产链删除；源码搜索不存在 TaskSpace 反馈 developer/system 注入。
- 确定性测试覆盖 JSON syntax、顶层合同、preflight、内部 Fatal 与成功 sibling，错误只返回一次且不混入第二种解释。
- 最新三次 TaskSpace 生产运行各有 6 个 `taskspace_exec` call、6 个同 ID output，合计 `18 = 18`；未匹配 ID、重复
  output body 和 TaskSpace developer 反馈均为 0。

## 3. 结论

I02 已关闭。后续若出现同一 Tool 事实的 system/developer 副本、同 ID 多个 output，或内部结果在 outer 反馈之外再次进入
模型上下文，应按 I02 回归处理；结果内容是否准确、错误分类是否忠实继续由 I05 管理。

