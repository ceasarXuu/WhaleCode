# DeepSeek Default Model Fix

Date: 2026-04-27

## Problem

Launching `whale` could show `gpt-5.5 xhigh` in the TUI header and display the
upstream GPT-5.5 availability notice, even though Whale is intended to default
to DeepSeek V4.

## Root Cause

There are two default-model paths:

- core config defaults `model_provider` to `deepseek` and `model` to
  `deepseek-v4-pro`;
- the TUI bootstrap also consumes `model/list`, where the default model is
  determined by the model catalog's `is_default` marker.

The model catalog path still allowed upstream GPT entries to compete with Whale
entries. `gpt-5.5` also retained an upstream availability NUX message, so any
accidental selection leaked OpenAI-specific copy into Whale startup.

## Fix

The model manager now explicitly re-marks `deepseek-v4-pro` as Whale's
default picker model after auth filtering and generic picker visibility
selection. The bundled catalog also gives the DeepSeek entries top picker
priority and removes the GPT-5.5 availability NUX copy.

Regression tests now cover:

- bundled default model resolves to `deepseek-v4-pro`;
- DeepSeek Flash and Pro remain the first two visible picker entries;
- GPT-5.5 does not carry OpenAI availability NUX copy in the Whale catalog.

## Operational Note

If the running TUI still shows an old model after reinstalling, close that TUI
process and start a fresh `whale` session. Existing processes keep the model
catalog that was loaded at their own startup.

On this Windows machine, `cargo install --path cli --bin whale --locked --force`
can spend a long time in the final release optimization/link phase for the
single `whale` binary. For local validation after small catalog or TUI changes,
the faster path is:

```text
cargo build -p codex-cli --bin whale --locked
copy D:\BuildCache\whalecode\cargo-target\debug\whale.exe %USERPROFILE%\.cargo\bin\whale.exe
```

Use release install as a separate performance/package build, not as the fastest
inner-loop smoke step.

## Follow-up: Model Listing Scope

Whale now keeps public model listings scoped to DeepSeek models. The model
manager filters picker/listing presets to `deepseek-*`, app-server `model/list`
tests assert that hidden-model requests still stay Whale-scoped, and
`whale debug models` applies the same filter before printing raw catalog data.

When validating local output, check both normal and bundled debug paths:

```text
whale debug models
whale debug models --bundled
```

Both outputs should contain only `deepseek-v4-pro` and `deepseek-v4-flash`.
`deepseek-v4-pro` should sort first and be the default.
