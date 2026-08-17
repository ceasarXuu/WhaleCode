# I06 Tool 不可绕过边界关闭结算

- Date: 2026-08-18
- Status: closed
- Issue: R8-I06
- Runtime change: none
- Paid run: none

## 1. 产品验收条件

1. TaskSpace 中所有 client Tool 动作必须先通过同一请求级预检。
2. 顶层 client Tool 旁路必须在参数解析、Map 写入和真实 Tool 执行前拒绝。
3. 同一请求最多执行一个 Patch。
4. 通过预检后的普通 Tool 继续走原生 Router、权限、Hook、串并行和结果转换，不建立 TaskSpace 专用执行器。

## 2. 关闭证据

- TaskSpace Router 对模型只暴露 `taskspace_exec` 和原生 Hosted Tool；client Tool 只保留在内部 Catalog。
- `production_router_exposes_only_exec_and_hosted_and_blocks_client_bypass` 证明顶层 client Tool 在零调用、零 Map
  副作用下返回同 `call_id` 拒绝。
- `failed_preflight_has_no_map_or_client_tool_side_effect` 证明完整计划未通过时不会部分提交 Map 或执行 client Tool。
- preflight 对 Map 操作、节点归属、DAG frontier、revision、Tool 身份和单 Patch 限制统一检查；dispatch 仅消费通过后的
  prepared calls。
- 当前 TaskSpace Exec 定向套件 `77 passed / 0 failed`，覆盖顶层旁路、复合完整性失败、动态节点参数、单 Patch、
  原生串并行和错误反馈。
- 历史生产证据中曾自然出现顶层 `exec_command`，Runtime 在副作用前拒绝；最新三次成功 TaskSpace 运行未出现旁路，
  每个请求最多一个 Patch。

## 3. 结论

I06 的产品边界已经由入口结构、统一 preflight、原生 dispatch 和生产证据共同闭合。目标模型未来仍可能生成非法动作，
但那属于 I03 的 Agent 行为观察；只要 Runtime 保持零副作用拒绝，就不构成 I06 重新打开条件。

