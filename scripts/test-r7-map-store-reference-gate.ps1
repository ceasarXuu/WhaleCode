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

foreach ($file in Get-ChildItem -LiteralPath $scriptRoot -Recurse -File -Filter "*.ps1") {
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
        $invokesExporter = $line -match "&\s+\`$exportScript" -or
            $line -match "&\s+\(Join-Path.+export-action-map-observability\.ps1" -or
            $line -match '"-File".+\`$exportScript'
        if ($invokesExporter -and $line -notmatch "-ThreadId") {
            $violations.Add("${relative}:$($index + 1): observability exporter call omitted -ThreadId")
        }
    }
}

if ($violations.Count -gt 0) {
    throw "R7 Map Store reference gate failed:`n$($violations -join "`n")"
}

Write-Host "R7 Map Store reference gate: PASS"
