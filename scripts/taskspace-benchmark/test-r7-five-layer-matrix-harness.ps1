$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$runRoot = Join-Path ([IO.Path]::GetTempPath()) "r7-five-layer-matrix-plan-$([guid]::NewGuid().ToString('N'))"

try {
    & (Join-Path $PSScriptRoot "run-r7-five-layer-matrix.ps1") -Stage initial -Repeats 3 -RunRoot $runRoot -PlanOnly | Out-Null
    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $runRoot "run-manifest.json") | ConvertFrom-Json -Depth 50
    if ([string]$manifest.status -ne "planned") { throw "Plan-only manifest status drifted" }
    if ([int]$manifest.planned_run_count -ne 24) { throw "Initial matrix must contain exactly 24 runs" }
    foreach ($sample in @("single-file-fast-fix", "subscription-billing-repair")) {
        foreach ($repeat in 1..3) {
            $rows = @($manifest.runs | Where-Object { $_.sample -eq $sample -and [int]$_.repeat -eq $repeat })
            if ($rows.Count -ne 4) { throw "$sample repeat $repeat does not contain four arms" }
            $arms = @($rows | ForEach-Object { [string]$_.arm } | Sort-Object)
            if (($arms -join ",") -ne "map-always,map-append,map-request,standard") { throw "$sample repeat $repeat arm set drifted" }
        }
    }
    $blocked = $false
    try {
        & (Join-Path $PSScriptRoot "run-r7-five-layer-matrix.ps1") -Stage extended -Repeats 10 -RunRoot (Join-Path $runRoot "extended") -PlanOnly | Out-Null
    } catch {
        $blocked = $_.Exception.Message -match "explicit -AllowExtended"
    }
    if (-not $blocked) { throw "Extended matrix was not blocked without explicit approval" }
    Write-Output "R7 five-layer matrix harness passed."
} finally {
    if (Test-Path -LiteralPath $runRoot) {
        $backupRoot = Join-Path ([IO.Path]::GetTempPath()) "r7-five-layer-matrix-plan-complete"
        New-Item -ItemType Directory -Force -Path $backupRoot | Out-Null
        Move-Item -Force -LiteralPath $runRoot -Destination (Join-Path $backupRoot ([IO.Path]::GetFileName($runRoot)))
    }
}
