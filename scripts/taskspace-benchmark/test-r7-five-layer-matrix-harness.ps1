$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$runRoot = Join-Path ([IO.Path]::GetTempPath()) "r7-five-layer-matrix-plan-$([guid]::NewGuid().ToString('N'))"
. (Join-Path $PSScriptRoot "lib/r7-evaluation-authority.ps1")

function Copy-JsonObject($Value) {
    $Value | ConvertTo-Json -Depth 50 | ConvertFrom-Json -Depth 50
}

try {
    & (Join-Path $PSScriptRoot "run-r7-five-layer-matrix.ps1") -Stage initial -Repeats 3 -RunRoot $runRoot -PlanOnly | Out-Null
    $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $runRoot "run-manifest.json") | ConvertFrom-Json -Depth 50
    if ([string]$manifest.status -ne "planned") { throw "Plan-only manifest status drifted" }
    if ([int]$manifest.planned_run_count -ne 24) { throw "Initial matrix must contain exactly 24 runs" }
    $manifestPath = Join-Path $runRoot "run-manifest.json"
    $authorityCheck = Get-R7MatrixEvaluationAuthorityCheck $repoRoot $manifestPath $manifest
    if ([string]$authorityCheck.status -ne "valid") {
        throw "Plan-only manifest does not match frozen evaluation authority"
    }
    foreach ($attack in @(
            @{
                name = "contract"
                mutate = { param($value) $value.evaluation_contract_sha256 = "f" * 64 }
                finding = "matrix_evaluation_contract_identity_mismatch"
            },
            @{
                name = "model"
                mutate = { param($value) $value.model = "forged" }
                finding = "matrix_evaluation_environment_mismatch"
            },
            @{
                name = "sample"
                mutate = {
                    param($value)
                    $value.samples[0] = "forged"
                    foreach ($run in @($value.runs | Where-Object sample -eq "single-file-fast-fix")) {
                        $run.sample = "forged"
                    }
                }
                finding = "matrix_evaluation_design_mismatch"
            },
            @{
                name = "standard-projection"
                mutate = {
                    param($value)
                    @($value.runs | Where-Object arm -eq "standard")[0].projection_policy =
                        "map-append"
                }
                finding = "matrix_run_mode_identity_mismatch"
            }
        )) {
        $candidate = Copy-JsonObject $manifest
        & $attack.mutate $candidate
        $rejected = Get-R7MatrixEvaluationAuthorityCheck $repoRoot $manifestPath $candidate
        if ([string]$rejected.status -ne "invalid" -or
            [string]$attack.finding -notin @($rejected.findings)) {
            throw "Evaluation authority accepted $($attack.name) forgery"
        }
    }
    foreach ($sample in @("single-file-fast-fix", "subscription-billing-repair")) {
        foreach ($repeat in 1..3) {
            $rows = @($manifest.runs | Where-Object { $_.sample -eq $sample -and [int]$_.repeat -eq $repeat })
            if ($rows.Count -ne 4) { throw "$sample repeat $repeat does not contain four arms" }
            $arms = @($rows | ForEach-Object { [string]$_.arm } | Sort-Object)
            if (($arms -join ",") -ne "map-always,map-append,map-request,standard") { throw "$sample repeat $repeat arm set drifted" }
        }
    }
    $wirePath = Join-Path $runRoot "wire.jsonl"
    $profile = $authorityCheck.authority.tool_capability_profiles.standard
    $shape = @{
        schema_version = "provider-chat-wire-trace-v11"
        request_index = 1
        provider_wire_api = $authorityCheck.authority.provider_wire_api
        transport = $authorityCheck.authority.provider_transport
        tools_hash = $profile.tools_hash
        tools_count = [int64]$profile.tools_count
    }
    [IO.File]::WriteAllLines(
        $wirePath,
        @($shape | ConvertTo-Json -Compress),
        [Text.UTF8Encoding]::new($false)
    )
    $wireIdentity = Get-R7ProviderWireCapabilityIdentity $wirePath
    if ([string]$wireIdentity.tools_hash -ne [string]$profile.tools_hash) {
        throw "Provider wire tool identity was not observed"
    }
    $secondShape = Copy-JsonObject $shape
    $secondShape.request_index = 2
    $secondShape.tools_hash = "f" * 64
    [IO.File]::WriteAllLines(
        $wirePath,
        @(@($shape, $secondShape) | ForEach-Object { $_ | ConvertTo-Json -Compress }),
        [Text.UTF8Encoding]::new($false)
    )
    $unstableWireRejected = $false
    try {
        Get-R7ProviderWireCapabilityIdentity $wirePath | Out-Null
    } catch {
        $unstableWireRejected = $_.Exception.Message -match "unstable or missing"
    }
    if (-not $unstableWireRejected) {
        throw "Provider wire accepted multiple tool capability identities"
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
        Remove-Item -Force -Recurse -LiteralPath $runRoot
    }
}
