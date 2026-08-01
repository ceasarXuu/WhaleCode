# R8 首次获批 revalidation 运行结果

- 时间：2026-08-01 12:52
- Record：`WAR-20260801-125219-CACHE-REGRESSION-34ACB825`
- Subject HEAD：`860373cfa39ae8a65d559168dfcf6a83ac13d80e`
- Proposal：`CBP-60338F2F86A4B693`
- Authorization：`CBA-20260801-60338F2F86A4B693`
- 状态：`partial`，runner exit `3`

## 1. 获批范围

| 项目 | 值 |
|---|---|
| 模型 | `deepseek-v4-flash` |
| Sample | `single-file-fast-fix` |
| Arms | `standard`、`map-request` |
| Repeat | 每臂 1 次 |
| 自动重试 | 0 |
| Provider 请求硬上限 | 每 run 10，总计 20 |
| Token 观测阈值 | 每 run input 150K、output 5K |
| 时间硬上限 | 每 run 600 秒，清理宽限 120 秒 |

## 2. 实际路径

1. gate report 的 10 个生产 final-wire 场景均为 `unchanged`，形成合法 `revalidation_requested`。
2. proposal 与用户授权逐值匹配，runner 原子认领全局账本记录。
3. Standard 首臂在 Whale binary preflight 阶段失败；attempted pair 为 0。
4. `after_any_run_failure` 立即停止批次；map-request 未执行，没有自动重试。
5. 容器、网络和 host secret 均验证为空。

## 3. 根因

`~/.whale/bin/whale` 及其 schema v2 attestation 仍对应旧构建身份：

- 已证明的 HEAD：`4c6f7a7ca`；
- 已证明的 Codex source commit：`923e8c945`；
- 当前要求的 Codex source commit：`0d3af4b54`；
- preflight reason：`codex_source_commit_mismatch,git_build_identity_mismatch`。

因此这是本机构建产物过期，不是 Standard、map-request、Agent 或 provider 的执行结果。

## 4. 成本与证据边界

账本记录 `api_requests=null`、`api_requests_minimum=0`、`api_requests_evidence_status=unavailable`。从执行顺序和
artifact 看，没有创建 pair 或 provider boundary；但正式监督计数证据不存在，因此不能把实际 API 请求和费用宣称为
精确 0。该批次不能晋升，也不能用于缓存命中率比较。

## 5. 后续动作

先从干净 HEAD 重建 Whale，安装到隔离目录并生成匹配的 binary attestation，再用离线 binary health probe 验证。
本次授权明确禁止重试且已被一次性认领；修复后如需再次运行，必须生成绑定新 HEAD 的 proposal 并重新取得用户授权。
