param(
    [string]$RepoRoot = ""
)

$ErrorActionPreference = "Stop"
if ([string]::IsNullOrWhiteSpace($RepoRoot)) {
    $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
}

$scriptRoot = Join-Path $RepoRoot "scripts"
$violations = New-Object System.Collections.Generic.List[string]
$forbidden = @(
    [ordered]@{ pattern = "debug\s+taskspace-replay"; reason = "deleted rollout replay command" },
    [ordered]@{ pattern = "Invoke-ActionMapCanonicalReplay"; reason = "deleted replay helper" },
    [ordered]@{ pattern = "source\.replay"; reason = "retired observability source" },
    [ordered]@{ pattern = "action-map-replay-proof-lib"; reason = "retired replay library" }
)

foreach ($file in Get-ChildItem -LiteralPath $scriptRoot -Recurse -File | Where-Object {
    $_.Extension -in @(".ps1", ".py", ".sh")
}) {
    if ($file.FullName -eq $PSCommandPath) { continue }
    $relative = [System.IO.Path]::GetRelativePath($RepoRoot, $file.FullName)
    $lines = Get-Content -LiteralPath $file.FullName -Encoding UTF8
    for ($index = 0; $index -lt $lines.Count; $index++) {
        $line = [string]$lines[$index]
        foreach ($rule in $forbidden) {
            if ($line -match [string]$rule.pattern) {
                $violations.Add("${relative}:$($index + 1): $($rule.reason)")
            }
        }
        if ($file.Extension -eq ".py" -and $line -match "snapshot_(updated|delta)") {
            $violations.Add("${relative}:$($index + 1): retired rollout Map snapshot parser in active Python tooling")
        }
        $invokesExporter = $line -match "&\s+\`$exportScript" -or
            $line -match "&\s+\(Join-Path.+export-action-map-observability\.ps1" -or
            $line -match '"-File".+\`$exportScript'
        if ($invokesExporter -and $line -notmatch "-ThreadId") {
            $violations.Add("${relative}:$($index + 1): observability exporter call omitted -ThreadId")
        }
    }
}

$rustRoots = @(
    "third_party/codex-cli/codex-rs/core/src",
    "third_party/codex-cli/codex-rs/protocol/src",
    "third_party/codex-cli/codex-rs/cli/src",
    "third_party/codex-cli/codex-rs/state/src",
    "third_party/codex-cli/codex-rs/app-server/src",
    "third_party/codex-cli/codex-rs/tui/src"
)
$rustForbidden = @(
    "taskspace_replay",
    "TaskSpaceReplayProofR6V1",
    "debug taskspace-replay",
    "SnapshotUpdated",
    "SnapshotDelta",
    "action_map_checkpoint",
    "build_snapshot_delta",
    "apply_snapshot_delta",
    "taskspace_snapshot_restore",
    "sess.action_map_snapshot()"
)
foreach ($relativeRoot in $rustRoots) {
    $root = Join-Path $RepoRoot $relativeRoot
    foreach ($file in Get-ChildItem -LiteralPath $root -Recurse -File -Filter "*.rs") {
        $relative = [System.IO.Path]::GetRelativePath($RepoRoot, $file.FullName)
        $lines = Get-Content -LiteralPath $file.FullName -Encoding UTF8
        for ($index = 0; $index -lt $lines.Count; $index++) {
            $line = [string]$lines[$index]
            foreach ($symbol in $rustForbidden) {
                if ($line.IndexOf($symbol, [System.StringComparison]::Ordinal) -ge 0) {
                    $violations.Add("${relative}:$($index + 1): retired canonical Map recovery symbol '$symbol'")
                }
            }
        }
    }
}

if ($violations.Count -gt 0) {
    throw "R7 Map Store reference gate failed:`n$($violations -join "`n")"
}

Write-Host "R7 Map Store reference gate: PASS"
Write-Host "Overall: PASS"
