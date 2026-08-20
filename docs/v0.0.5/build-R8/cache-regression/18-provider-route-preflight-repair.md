# R8 provider 路由预检修复结果

- 时间：2026-08-01
- 状态：Round 8 fresh closure PASS；离线工程闭环完成，真实缓存收益待单独预算验证
- 范围：provider transport alias 的启动前验证、最终请求等价和证据身份闭环
- 真实 Whale Agent run：未执行

## 1. 修复目标

上一轮真实缓存复验在任何 provider 请求发出前失败：benchmark 试图覆盖保留的 `deepseek` provider。改用
`deepseek-boundary` transport alias 后，对抗性审查仍指出三个可信性缺口：实际 CLI/完整配置入口没有前置验证，
alias 没有直接走 normal final-wire 测试，resolved route 没有绑定到 arm、result 和成本账本。

本轮只关闭这些缺口，不向 runtime wire event 注入第二套 logical provider 语义，也不扩大到 compaction、child-agent
或模型缓存设计。

## 2. 实现

1. 新增隐藏机械命令 `whale debug provider`。它通过生产 `Config` 加载链解析实际 CLI override；所有会改变 URL、
   header、认证、重试和传输行为的 provider 字段均进入安全 descriptor。Python 父进程创建一个私有临时目录和
   HMAC key，四次 CLI 解析共享该 key；成功、CLI 非零或外层 timeout 后均由父进程统一销毁。timeout 会终止并
   确认整个进程组退出，再按 RunId 标签确认 Docker 容器清空。API Key、token、
   header 值和原始 URL 不输出明文或无盐摘要，只输出 keyed fingerprint。
2. 缓存 runner 在读取凭证、认领授权和增加实际 sample 数之前，先在 Docker `--network none` 中分别加载
   Standard 与 map-request 配置。每个 profile 同时解析 alias 和内置 DeepSeek，除 provider ID 与 base URL 外必须
   逐字段等价；该命令只做 Config 解析，并在 `network=none` 容器中执行。每个容器的 Docker inspect 会转成
   不含主机路径和 secret 源路径的机械 receipt，promotion 会复核 receipt 的 SHA 和内容。原始 inspect 只写入
   父进程拥有的临时目录，并在解析后立即删除，不进入持久化证据目录；receipt 还证明 key 的目标挂载、宿主源
   挂载均唯一，并证明同一个 mount 同时匹配 Source、Destination 且只读。
3. 非 `deepseek-*` 模型在 cache preflight、通用 benchmark 参数入口和 provider boundary 启动函数三处机械拒绝。
4. normal final-wire 集成测试分别用内置 DeepSeek 与从完整配置加载的 alias 发出请求；稳定化环境路径后，两个
   JSON request body 必须完全相等。
5. route identity、descriptor SHA、预检 artifact SHA 和 arm 对应 profile 进入 result/observation/ledger。预检
   执行脚本本身同时进入授权 execution identity 与 cache control-plane。baseline
   promotion 从指定 Git source 重读 Standard/TaskSpace 的 alias 与内置 DeepSeek 四份原始产物，复算文件 SHA、
   descriptor 和字段等价性；缺失、跨 record、漂移或篡改均拒绝。
6. 正式 runner 把预检原件写入对应 record 的 `benchmarks/cache-regression/evidence/`，使后续 Git index/commit
   验收可复算；`target/` 只用于本机手工机械预检。

历史成本流水不伪造回填。Schema 只声明新增字段；新 cache runner 的创建与结算路径强制要求这些字段，正式
promotion 同时要求预检原件。

## 3. 验证

| 验证 | 结果 |
|---|---|
| Python cache regression | `215 passed` |
| CLI production config integration | `debug_provider`: `1 passed` |
| provider alias runtime/final-wire | `cache_provider_boundary_route`: `2 passed` |
| provider boundary / container contract | 全部通过 |
| 全局 Whale Agent 成本账本检查 | `10 entries` 通过 |
| Docker config-only preflight | `passed`；Standard + TaskSpace；alias + built-in；`network_mode=none` |
| failure-atomic 故障注入 | Python timeout、忽略 SIGTERM 的后代、CLI 非零和真实 Docker `/bin/false` 均未遗留 key/raw inspect/宿主路径 |
| runner 结构约束 | 入口 `342` 行，pair stage `499` 行；AST 与 `-PlanOnly` 通过 |
| 持久化失败日志 | lifecycle/cleanup 只保留稳定 reason code 与机械摘要，不保留 Docker 原始宿主路径 |

本机最新成功预检证据位于 `target/provider-route-round6-1785578747/provider-route-preflight.json`。它只执行机械配置加载，没有模型
请求，不计入真实 Whale Agent run 预算，也没有修改全局付费运行账本。

## 4. 边界

- 该结果证明下一次获批运行会在消耗 sample 计数前发现 route 配置错误；不证明真实 provider 可用或缓存收益达标。
- alias 的 transport ID 是 benchmark 寻址信息；产品 runtime 仍只处理实际 provider，不承担 logical/transport
  再解释。
- 固定 arm 顺序可能影响服务端缓存测量的问题仍是独立实验设计风险，本轮不处理。
- 新的真实缓存复验仍需用户单独批准预算。
