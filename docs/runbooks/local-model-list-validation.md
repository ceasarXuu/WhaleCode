# Local Model List Validation

Use this check after local reinstall work or Codex upstream model-catalog syncs.
It verifies the installed Whale binary, not only the workspace build output.

## Reinstall

```powershell
cargo build -p codex-cli --bin whale --locked
.\scripts\install-whale-local.ps1 -BinaryPath "$env:CARGO_TARGET_DIR\debug\whale.exe" -PersistUserPath -BackupLegacyCopies
```

If `CARGO_TARGET_DIR` is unset, pass the workspace target path instead:

```powershell
.\scripts\install-whale-local.ps1 -BinaryPath .\third_party\codex-cli\codex-rs\target\debug\whale.exe -PersistUserPath -BackupLegacyCopies
```

## Validate

```powershell
$Whale = "$env:USERPROFILE\.whale\bin\whale.exe"
& $Whale --version
where.exe whale
& $Whale debug models
& $Whale debug models --bundled
```

Both model commands must list only:

- `deepseek-v4-pro`
- `deepseek-v4-flash`

Treat any `gpt-*`, `chatgpt`, `codex`, or `openai` model slug in those command
outputs as a regression. Test snapshots for model picker surfaces should use a
Whale-scoped catalog unless the test is explicitly validating upstream legacy
compatibility behavior.
