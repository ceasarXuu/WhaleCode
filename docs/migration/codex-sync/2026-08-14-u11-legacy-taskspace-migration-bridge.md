# U11：旧 Whale TaskSpace migration bridge

- 日期：2026-08-14
- 上游 substrate：Codex CLI `rust-v0.147.0`
- 结论：`verified`
- 真实模型请求：0
- 真实用户数据库读写：0

## 1. 问题与边界

旧 Whale state DB 已把 TaskSpace 表占用为 migration `0030/0031`，而 Codex 0.147 用相同版本号执行 `threads.thread_source` 和删除 `device_key_bindings`。SQLx 会因同版本 checksum 不同返回 `VersionMismatch`，导致旧数据库无法启动。

U11 只修复这一数据兼容阻塞：不恢复 TaskSpace runtime，不创建第二数据库，不读取或修改本机真实用户数据库。验证全部使用临时合成 SQLite fixture。

## 2. 实现结果

- 在 state migrator 之前检测 `_sqlx_migrations`；
- 仅接受两个已知旧 Whale SHA-384 checksum，并要求三张旧 TaskSpace 表完整存在、`threads.thread_source` 尚未存在；
- 在同一事务中补执行 0.147 的两项 schema 动作，再把 `0030/0031` 的 description/checksum 更新为当前 migrator 元数据；
- TaskSpace 表及其中的 canonical JSON 原地保留；
- fresh/current DB 直接 no-op；缺少单条记录、未知 checksum 或部分迁移 schema 均不改写，继续交给 SQLx fail-closed。

已知旧 checksum：

- `0030`: `e8b7fd4f72e583f648a6eac076ae000e63b3ffc7fca86e64321b7a60293beb9c9441d8cab044fd92a298d8cb90c387eb`
- `0031`: `edf1e09cca8f6c1340648cba32c0dd11e2129fa6763c846304aa9c40eb8341fb1aace4b9c0b4b3b6ba6e86691c7fe7f1`

## 3. 验证结果

| 验证 | 结果 |
| --- | --- |
| 精确 checksum fixture | 两个 SQL fixture 的 SHA-384 与旧 Whale migration 完全一致 |
| 已知旧库升级 | passed；0.147 `StateRuntime` 成功打开，canonical JSON 原值保留，当前 `0030/0031` checksum 生效 |
| fresh/current DB | passed；repair 前后 migration metadata 不变 |
| 未知/部分历史 | passed；missing `0031`、未知 checksum、部分 schema 均未被 repair，SQLx 继续拒绝 |
| U11 focused tests | 3 passed |
| `cargo test -p codex-state --lib` | 173 passed |
| sync replay / metadata | 42 tests passed；inventory/replay/metadata checks passed；当前 overlay 36 路径 |
| cache regression index gate | passed；指纹 `64a753dc368395160c04f81223fbeb8c3b93fad081b524ddf040ce8c24b408ae` 未变；最近一次 live 回归仍为失败且未晋升 |
| 生产代码规模 | 99 行新增，小于单工作单元 500 行门禁 |
| 外部请求 | 0 |

## 4. 结论与下一步

U11 已解除已知旧 Whale state DB 升级到 0.147 时的 migration 版本碰撞，同时保留未知历史的安全阻断语义。下一步按唯一计划先完成 U12 canonical kernel 的精确保留/淘汰清单与生产代码预算；在预算获得明确批准前不开始大规模移植。
