# 与最初理念相关的目录地图

## 结构与责任

```text
README.md                         当前产品入口与原则摘要
docs/plans/                       产品、架构和实验方案；包含首轮设计原稿
docs/adr/                         技术路线决策与被替代关系
docs/testing/                     方案是否有效的验证设计与运行记录
docs/v0.0.5/                      TaskSpace 多轮实现和收敛材料
coe/                              故障案例与根因证据
benchmarks/                       TaskSpace、缓存和真实运行证据
scripts/                          开发、验证、benchmark 与发布入口
third_party/codex-cli/            Codex upstream substrate 与当前 Rust workspace
patches/codex-cli/                不可避免的 vendor 修改记录
archive/deprecated/               可恢复的早期实现，包括已废弃 Rust demo
summary-my-repo/                  派生的仓库理解材料，不是产品权威源
```

## 权威边界

- 产品身份和原则以 `README.md`、根 `AGENTS.md` 与仍有效的 ADR 为当前入口。
- 2026-04-24/25 的计划文档是“最初理念”的主要历史证据，但其中部分技术路线和数值已被更新。
- `docs/adr/2026-04-27-codex-cli-upstream-substrate.md` 是当前 substrate 路线的权威决策。
- `third_party/codex-cli/codex-rs/` 是当前可执行 Rust 核心；不能仅从旧架构伪代码推断现状。
- `benchmarks/`、`docs/testing/` 和 `coe/` 用于判断理念是否真的带来效益，体现“默认启用必须靠证据获得”的原则。
- `target/` 是构建和实验派生产物，不是设计或实现权威源。

## 新工作应放在哪里

- Whale 差异化逻辑应优先进入 bridge、overlay、独立 module 或脚本层。
- 修改 vendor 文件前应确认无法通过外围层实现，并维护 patch/sync 证据。
- 非显而易见且可复用的操作经验进入 `docs/runbooks/`。
- 设计演进应通过 ADR 或当前计划文档明确替代关系，避免旧方案继续被误当成实现依据。

