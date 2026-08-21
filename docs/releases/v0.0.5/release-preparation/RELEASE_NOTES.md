# WhaleCode v0.0.5 发布说明（草稿）

> 状态：工程候选已通过离线预检；Whale 自有六平台构建入口已建立但尚未运行，发布者人工签核和实际发布授权仍待完成。

WhaleCode v0.0.5 是以 DeepSeek 为核心的终端 coding agent 候选版本。该版本基于 OpenAI Codex CLI `0.149.0` substrate，但 Whale 产品版本、CLI 版本和产品 tag 均为 `0.0.5` / `v0.0.5`。

## 主要内容

- DeepSeek provider、模型目录、Responses API、reasoning stream 与缓存记账。
- TaskSpace relational state、execute/fork/reload/final-wire 路径及相关可观测性。
- workspace 隔离 bootstrap、二进制 attestation 和本地开发安全门禁。
- Codex 0.149 upstream 同步、overlay provenance 与隔离回归入口。

## 已知边界

- 当前发布准备面向 Whale + DeepSeek + TaskSpace 主路径。
- OpenAI/ChatGPT 登录、OpenAI remote catalog/plugin sharing、Bedrock 和相关上游专属测试不属于当前产品发布合同。
- npm 是既有 Whale 独立分发渠道，但本次实际发布尚未授权；WinGet、R2 和网站渠道仍未建立，vendor 内上游发布工作流不得用于 Whale 发布。
- Whale npm 元包已按 `@ceasarxuu/whalecode@0.0.5` 完成离线 staging/pack 验证；六个平台原生包仍必须来自明确批准的 WhaleCode `whale-native-artifacts` run。
- 六平台 workflow 仅生成未签名候选归档、校验值和制品合同，不创建 tag、不发布 npm、不创建 GitHub Release；首次 run 与跨系统安装 smoke 完成前不能进入实际 npm 发布。
- 全量 upstream suite 存在已分类的非产品面失败，不能描述为全绿。

## 版本身份

- WhaleCode：`v0.0.5`
- Codex upstream substrate：`rust-v0.149.0`

两者不得互换登记。
