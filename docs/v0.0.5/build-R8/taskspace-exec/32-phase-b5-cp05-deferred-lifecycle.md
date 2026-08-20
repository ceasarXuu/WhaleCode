# Phase B5 CP-05：延迟能力生命周期

- Date: 2026-08-10
- Scope: TaskSpace Catalog、Tool Search nested result、Router effective surface
- Status: verified offline / provider cache revalidation pending

## 1. 问题

Standard 首轮只暴露 `tool_search`，Provider 在后续请求中依据成功的 Tool Search output 加载所选 deferred schema。
TaskSpace 把 Tool Search 作为 `taskspace_exec` 的内部 client Tool 后，Provider 只看到 outer Function output，不会替
Runtime 自动扩展 outer schema。此前 Catalog 又直接读取所有注册 spec，导致 deferred dynamic Tool 在首轮被提前展开；
deferred MCP 则只有 handler、没有 Catalog schema，两类能力的生命周期不一致。

## 2. 实现

1. `taskspace_exec` 的成功 Tool Search 继续通过 CP-09 的中性 nested result 忠实返回完整 `LoadableToolSpec`；
2. 每次构建 TaskSpace Router 时，从本次自然输入历史中查找与真实 `taskspace_exec` call_id 配对的 outer output；
3. 只读取 `kind=taskspace_exec_result`、单项 outcome 为 succeeded、nested result 为 completed client Tool Search 的
   deferred schema；失败、畸形、未配对输出均不产生能力；
4. 当前 Router 必须仍有对应精确 `ToolName` handler。历史中已失效或伪造的 schema 会被机械过滤；
5. 已注册的 deferred dynamic Tool 使用当前 Registry 的原生 spec；仅 deferred MCP 使用自然历史里 Tool Search 返回的
   schema。Catalog、decoder、identity 与 dispatch 仍由同一个 request-local Catalog 驱动。

没有新增 session ledger、第二 Registry、兼容 reader、字符串名称猜测或 Runtime 选 Tool 逻辑。压缩或裁剪后若完整搜索
结果不再存在于自然上下文，能力会 fail closed，Agent 可再次调用 `tool_search`；Runtime 不暗中保留模型不可见能力。

## 3. 行为结果

| 场景 | TaskSpace 行为 |
|---|---|
| 首次请求 | 暴露 `tool_search`，不展开 deferred dynamic/MCP schema |
| 成功搜索 dynamic Tool | 下一请求只展开被返回且当前仍注册的 Tool |
| 成功搜索 MCP Tool | 下一请求按原生 namespace + leaf identity 展开并复用现有 MCP handler |
| 搜索失败或结果畸形 | 不加载任何能力，不伪造空成功 |
| 历史 schema 已失效 | 因当前无精确 handler 被过滤 |
| Standard / Code Mode | 构建和执行路径不变 |

能力集合变化会自然改变 Runtime-only capability identity；该 identity 不进入 Agent schema、Map 或普通 Tool 参数。

## 4. 验证

- `cargo test -p codex-tools responses_api --lib`：6 PASS；
- `cargo test -p codex-core taskspace_exec --lib`：65 PASS；
- `cargo test -p codex-core taskspace_cp01_records_deepseek_effective_surface --lib`：1 PASS；
- 覆盖 Function 与 Namespace Tool Search schema 反序列化、配对/失败/畸形拒绝、首轮隐藏、按选择展开、失效 handler
  过滤、dynamic + MCP 真实 DeepSeek effective surface；
- `python3 scripts/cache-regression/check_cache_regression_gate.py --source index`：免费 final-wire PASS，候选指纹
  `ef04461a4ea5f563d33f8a41faa675182b0a01f2951cff08967fc0c3157027d5`；发布继续阻断，等待真实回归。

## 5. 后续

CP-07 完整化 outer 输入合同，CP-08 从 CP-09 固定结果类型生成同源 output schema。CP-12/CP-13 汇总 final-wire、
缓存和零基线门禁；在此之前不启动 VA-02/VA-03。
