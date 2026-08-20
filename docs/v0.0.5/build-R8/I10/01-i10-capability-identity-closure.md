# I10 TaskSpace 能力身份关闭结算

- Date: 2026-08-18
- Status: closed
- Issue: R8-I10
- Runtime change: none
- Paid run: none

## 1. 产品验收条件

1. 能力身份从实际生效的同一 Catalog 快照机械生成。
2. 相同能力得到相同身份；Tool 名称、描述、类型、schema、deferred 或 Hosted 配置变化会改变身份。
3. Catalog、dispatch、请求快照、Provider wire、Exec trace 和性能报告引用同一个值。
4. 身份只作为 Runtime metadata，不进入 Agent schema、Map、Tool 参数、聊天上下文或 Provider payload。
5. Standard 不携带 TaskSpace 能力身份。

## 2. 关闭证据

- `TaskSpaceExecCatalog::build` 从 outer declaration、原生 Hosted declarations、client capabilities 和 Map capabilities
  一次性计算 SHA-256，Router、response scope、Prompt metadata、Exec trace 和 wire trace 只传递该值。
- Catalog 确定性、语义变化、deferred capability、Router 单一身份、HTTP/WS trace、Standard 空身份、缺失/冲突
  observer fixture 均有确定性测试。
- Provider request 等值测试证明只设置 Runtime identity 不改变序列化请求，因此不会污染缓存前缀或 Agent 上下文。
- 最新三次 TaskSpace 生产运行共 21 个 Provider wire 请求，全部使用
  `05b41a6b15e1dac3f2dff181288be5eee451cda1c15cfb18374767d7a093a3bf`，无缺失或冲突；同批 observer 未报告
  identity finding。
- 当前 TaskSpace Exec 定向套件 `77 passed / 0 failed`；既有 Core、workspace、zero-base、性能 observer 和缓存门禁
  证据继续有效。

## 3. Projection 验收边界

三种 projection 只决定 Map 如何进入消息上下文，不参与 Catalog 构建或 capability identity 计算。只要 Tool 配置相同，
三种 projection 理应引用同一身份；分别执行三次付费运行只会重复验证同一机械不变量。因此三 projection 不再作为 I10
关闭门槛，后续三臂测试归入 I01 的上下文一致性和 I08 的成本验收。

## 4. 结论

I10 已关闭。以后只有 effective Tool capability 变化却未改变身份、同一请求链出现身份冲突、身份进入 Agent/Provider payload，
或 Standard 非空时才重新打开。

