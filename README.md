# WhaleCode

WhaleCode 是一个以 DeepSeek V4 为核心的开源终端 AI coding agent。它可以在真实代码仓库中读取文件、执行命令、修改代码、运行测试，并通过 TaskSpace 组织复杂任务。

当前稳定版本为 `v0.0.5`，默认模型是 `deepseek-v4-flash`。

## 安装

### 环境要求

- Node.js 16 或更高版本
- DeepSeek API Key
- 受支持的平台：Linux、macOS、Windows

### 通过 npm 安装

Linux、macOS 和 Windows 使用同一条命令：

```bash
npm install -g @ceasarxuu/whalecode@latest --include=optional
```

npm 会根据当前系统和 CPU 自动安装对应的 Whale 原生二进制。

| 系统 | 支持的架构 |
| --- | --- |
| Linux | x64、ARM64 |
| macOS | Intel x64、Apple Silicon ARM64 |
| Windows | x64、ARM64 |

验证安装：

```bash
whale --version
whale doctor
```

升级或卸载：

```bash
npm install -g @ceasarxuu/whalecode@latest --include=optional
npm uninstall -g @ceasarxuu/whalecode
```

如果终端找不到 `whale`，请重新打开终端，并确认 npm 的全局 bin 目录已经加入 `PATH`。

## 登录与快速开始

从 [DeepSeek 开放平台](https://platform.deepseek.com/api_keys) 创建 API Key，然后通过标准输入交给 Whale。Key 不应写入仓库或命令参数。

Linux / macOS：

```bash
export DEEPSEEK_API_KEY="your-api-key"
printf '%s' "$DEEPSEEK_API_KEY" | whale login --with-api-key
```

Windows PowerShell：

```powershell
$env:DEEPSEEK_API_KEY = "your-api-key"
$env:DEEPSEEK_API_KEY | whale login --with-api-key
```

检查登录状态并进入项目：

```bash
whale login status
cd path/to/your-project
whale
```

也可以直接执行一次非交互任务：

```bash
whale exec "解释这个仓库的入口和核心模块"
```

## 模型选择

WhaleCode 当前内置三个 DeepSeek 模型：

| 模型 | 建议用途 |
| --- | --- |
| `deepseek-v4-flash` | 默认选择，适合日常编码和常规任务 |
| `deepseek-v4-pro` | 复杂设计、诊断和高质量推理 |
| `deepseek-v4-flash-vision-exp` | 包含截图、界面或其他图片输入的任务 |

启动时选择模型：

```bash
whale -m deepseek-v4-pro
whale exec -m deepseek-v4-flash "修复这个测试"
```

进入交互界面后，可用 `/model` 切换模型和 reasoning effort。选择结果会用于当前会话，并保存为后续新会话的默认值。

Vision 模型也可以处理纯文本。附加图片时，在图片参数后使用 `--` 分隔提示词：

```bash
whale exec \
  -m deepseek-v4-flash-vision-exp \
  --image ./screenshot.png \
  -- "分析这张截图中的问题"
```

## 重点功能：TaskSpace

TaskSpace 面向多文件、长链路和需要持续验证的复杂任务。它把任务表示为可持久化的结构化 Map，包含目标、工作节点、依赖、工具行动、结果引用和完成状态。与只依赖对话文本相比，这种状态可以在长任务、子线程、恢复和重启过程中继续被读取与校验。

适合使用 TaskSpace 的场景：

- 跨多个模块实现一项功能；
- 需要调查、修改、测试和复核多个阶段；
- 任务会使用多个 Agent 或需要中途恢复；
- 希望查看当前工作节点、依赖和完成情况。

对于解释一段代码、修改单个字符串等简单任务，默认 Standard 模式通常更快、更省。

### 交互模式

先启动 Whale，在提交任务前输入 `/taskspace`：

```text
$ whale
> /taskspace
> 重构认证模块，保持兼容并补齐回归测试
```

`/taskspace` 会启用 TaskSpace，并在浏览器中打开当前 Map 的本地只读视图。任务执行期间可以再次输入：

```text
/task-show
```

`/task-show` 只查看当前 Map，不会改变运行模式。视图绑定在 `127.0.0.1`，会持续刷新任务节点、依赖、行动和结果状态。

### 非交互模式

```bash
whale exec --taskspace "重构认证模块，保持兼容并补齐回归测试"
```

需要更强推理时可以同时选择 Pro：

```bash
whale exec --taskspace -m deepseek-v4-pro "定位并修复这个跨模块并发问题"
```

会话默认会持久化。退出后可恢复最近一次会话：

```bash
whale resume --last
```

恢复后使用 `/task-show` 查看当前 TaskSpace 状态。

## 已知风险

- **TaskSpace 仍是实验能力。** Map schema、交互方式和内部协议可能在后续版本演进，不应将其当前内部 JSON 结构作为稳定的外部 API。
- **复杂任务会增加成本。** TaskSpace 本身会增加状态和工具协议开销；与多 Agent、Pro 模型或长任务组合时，API 请求数、token 消耗和耗时通常高于 Standard 模式。
- **模型可能生成无效行动。** Runtime 会在执行前拒绝不符合 TaskSpace 合同的调用，避免无效 client Tool 产生副作用，但当前任务可能因此中断并需要恢复或重试。
- **平台验证深度不同。** `v0.0.5` 的六个平台 npm 安装 smoke 已通过；完整的 DeepSeek + TaskSpace 端到端回归以 Linux 为主，Windows 原生终端和完整 TUI 矩阵仍未全部验证。
- **本地视图包含任务元数据。** TaskSpace viewer 只监听 localhost，但会展示目标、节点和 source refs。不要通过端口转发或代理把它暴露给不受信任的网络。
- **Agent 仍可能修改或执行代码。** TaskSpace 不会绕过 Whale 的 sandbox 和 approval 设置，但也不能替代 Git、代码审查和测试。建议在干净的 Git 工作区中运行，并在提交前检查 diff。
- **原生二进制尚未签名。** 当前 npm/GitHub Release 制品附带完整性信息，但不是经过平台代码签名的发行物。

遇到问题时先运行：

```bash
whale doctor --summary
```

## 项目状态与文档

WhaleCode 正在持续开发。`v0.0.5` 使用 Codex CLI `0.149.0` 作为底层 substrate；两者是独立的版本号。

- [v0.0.5 Release](https://github.com/ceasarXuu/WhaleCode/releases/tag/v0.0.5)
- [发布说明](docs/releases/v0.0.5/release-preparation/RELEASE_NOTES.md)
- [开发流程](docs/runbooks/development-workflow.md)
- [本地 workspace 安全](runbooks/local-workspace-safety.md)
- [系统架构](docs/plans/2026-04-24-system-architecture.md)
- [Codex upstream substrate ADR](docs/adr/2026-04-27-codex-cli-upstream-substrate.md)

仓库主要目录：

```text
third_party/codex-cli/   Codex CLI upstream substrate
patches/codex-cli/       必要的 vendor patch queue
docs/                    设计、发布和工程文档
scripts/                 开发、验证和发布脚本
benchmarks/              TaskSpace 与缓存回归证据
```

开发者从源码构建前，请先阅读 [本地 workspace 安全说明](runbooks/local-workspace-safety.md)，不要使用 PATH 中其他 worktree 或全局安装的 Whale 代替当前工作区构建产物。
