# WhaleCode

English | [简体中文](README.md)

WhaleCode is an open-source terminal AI coding agent built around DeepSeek V4. It can read files, run commands, modify code, execute tests, and organize complex work with TaskSpace inside real repositories.

The current stable release is `v0.0.5`. Its default model is `deepseek-v4-flash`.

## Installation

### Requirements

- Node.js 16 or later
- A DeepSeek API key
- Linux, macOS, or Windows

### Install with npm

Use the same command on Linux, macOS, and Windows:

```bash
npm install -g @ceasarxuu/whalecode@latest --include=optional
```

npm automatically installs the native Whale binary for your operating system and CPU.

| Platform | Supported architectures |
| --- | --- |
| Linux | x64, ARM64 |
| macOS | Intel x64, Apple Silicon ARM64 |
| Windows | x64, ARM64 |

Verify the installation:

```bash
whale --version
whale doctor
```

Upgrade or uninstall:

```bash
npm install -g @ceasarxuu/whalecode@latest --include=optional
npm uninstall -g @ceasarxuu/whalecode
```

If your shell cannot find `whale`, restart the terminal and confirm that npm's global bin directory is on `PATH`.

## Login and quick start

Create an API key in the [DeepSeek Platform](https://platform.deepseek.com/api_keys), then pass it to Whale through standard input. Do not put the key in your repository or command arguments.

Linux / macOS:

```bash
export DEEPSEEK_API_KEY="your-api-key"
printf '%s' "$DEEPSEEK_API_KEY" | whale login --with-api-key
```

Windows PowerShell:

```powershell
$env:DEEPSEEK_API_KEY = "your-api-key"
$env:DEEPSEEK_API_KEY | whale login --with-api-key
```

Check the login and open Whale in a project:

```bash
whale login status
cd path/to/your-project
whale
```

You can also run a one-off, non-interactive task:

```bash
whale exec "Explain the entry point and core modules in this repository"
```

## Model selection

WhaleCode currently includes three DeepSeek models:

| Model | Recommended use |
| --- | --- |
| `deepseek-v4-flash` | Default model for everyday coding and general tasks |
| `deepseek-v4-pro` | Complex design, diagnosis, and high-quality reasoning |
| `deepseek-v4-flash-vision-exp` | Tasks with screenshots, UI images, or other image input |

Select a model at startup:

```bash
whale -m deepseek-v4-pro
whale exec -m deepseek-v4-flash "Fix this test"
```

In the interactive UI, use `/model` to change the model and reasoning effort. The selection applies to the current session and is saved as the default for future sessions.

The Vision model also works with text-only prompts. When attaching an image, use `--` to separate image arguments from the prompt:

```bash
whale exec \
  -m deepseek-v4-flash-vision-exp \
  --image ./screenshot.png \
  -- "Find the problem shown in this screenshot"
```

## Featured capability: TaskSpace

TaskSpace is designed for complex work that spans multiple files, stages, and validation steps. It represents a task as a persistent structured Map containing goals, work nodes, dependencies, tool actions, result references, and completion state. Unlike conversation-only state, the Map can continue to be read and validated across long tasks, child threads, resumes, and restarts.

TaskSpace is useful when:

- implementing a feature across multiple modules;
- investigating, editing, testing, and reviewing in several stages;
- using multiple agents or resuming work later;
- inspecting current work nodes, dependencies, and progress.

For simple tasks such as explaining one function or changing a string, the default Standard mode is usually faster and cheaper.

### Interactive mode

Start Whale and enter `/taskspace` before submitting the task:

```text
$ whale
> /taskspace
> Refactor the authentication module, preserve compatibility, and add regression tests
```

`/taskspace` enables TaskSpace and opens a local read-only view of the current Map in your browser. During the task, enter:

```text
/task-show
```

`/task-show` displays the Map without changing the runtime mode. The viewer binds to `127.0.0.1` and refreshes task nodes, dependencies, actions, and result state.

### Non-interactive mode

```bash
whale exec --taskspace "Refactor the authentication module, preserve compatibility, and add regression tests"
```

Combine TaskSpace with Pro for harder reasoning:

```bash
whale exec --taskspace -m deepseek-v4-pro "Find and fix this cross-module concurrency bug"
```

Sessions are persisted by default. Resume the most recent one with:

```bash
whale resume --last
```

After resuming, use `/task-show` to inspect the current TaskSpace state.

## Known risks

- **TaskSpace is experimental.** Its Map schema, interactions, and internal protocol may evolve. Do not treat the current internal JSON format as a stable public API.
- **Complex work costs more.** TaskSpace adds state and tool-protocol overhead. Combining it with multiple agents, Pro, or long-running work usually increases API requests, token use, and elapsed time compared with Standard mode.
- **Models can produce invalid actions.** The runtime rejects calls that violate the TaskSpace contract before execution, preventing invalid client tool side effects, but the task may be interrupted and require a resume or retry.
- **Validation depth differs by platform.** npm installation smoke tests pass on all six supported platform/architecture targets. Full DeepSeek + TaskSpace end-to-end regression coverage is deepest on Linux; the complete native-terminal and TUI matrix has not yet been validated on Windows.
- **The local viewer contains task metadata.** It listens only on localhost, but displays goals, nodes, and source references. Do not expose it to untrusted networks through port forwarding or proxies.
- **The agent can modify and execute code.** TaskSpace does not bypass Whale's sandbox or approval settings, but it is not a substitute for Git, code review, and tests. Run it in a clean Git worktree and inspect the diff before committing.
- **Native binaries are not code-signed yet.** Current npm and GitHub Release artifacts include integrity information, but do not carry platform code signatures.

When troubleshooting, start with:

```bash
whale doctor --summary
```

## Project status and documentation

WhaleCode is under active development. Release `v0.0.5` uses Codex CLI `0.149.0` as its substrate; these are separate version identities.

- [v0.0.5 Release](https://github.com/ceasarXuu/WhaleCode/releases/tag/v0.0.5)
- [Release notes](docs/releases/v0.0.5/release-preparation/RELEASE_NOTES.md)
- [Development workflow](docs/runbooks/development-workflow.md)
- [Local workspace safety](runbooks/local-workspace-safety.md)
- [System architecture](docs/plans/2026-04-24-system-architecture.md)
- [Codex upstream substrate ADR](docs/adr/2026-04-27-codex-cli-upstream-substrate.md)

Main repository directories:

```text
third_party/codex-cli/   Codex CLI upstream substrate
patches/codex-cli/       Required vendor patch queue
docs/                    Design, release, and engineering documentation
scripts/                 Development, validation, and release scripts
benchmarks/              TaskSpace and cache-regression evidence
```

Before building from source, read the [local workspace safety guide](runbooks/local-workspace-safety.md). Do not substitute a global Whale installation or a binary from another worktree for the current workspace build.
