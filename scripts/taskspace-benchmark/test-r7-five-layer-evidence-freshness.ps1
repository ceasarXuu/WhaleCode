param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/r7-five-layer-evidence-freshness.ps1")
if ([string]::IsNullOrWhiteSpace($RunRoot)) {
    $RunRoot = Join-Path $repoRoot "target/r7-five-layer-evidence-freshness-selftest"
}
$fixtureRoot = Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff")
$runDir = Join-Path $fixtureRoot "sample/run-001"
$pairDir = Join-Path $runDir "pair-001"
New-Item -ItemType Directory -Path (Join-Path $pairDir "left/artifacts") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $pairDir "right/artifacts") -Force | Out-Null

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

function Write-FixtureJson([string]$Path, $Value) {
    $Value | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $Path -Encoding UTF8
}

$baseContract = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $repoRoot "benchmarks/taskspace/r7/base-instructions-contract.json") | ConvertFrom-Json -Depth 100
$manifestPath = Join-Path $repoRoot "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"
$manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath | ConvertFrom-Json -Depth 100
$manifestSha = Get-R7EvidenceSha256 $manifestPath
$sourceCommit = ((& git -C $repoRoot log -1 --format=%H -- third_party/codex-cli) | Select-Object -First 1).Trim()
$binaryPath = Join-Path $fixtureRoot "whale"
"fixture whale" | Set-Content -LiteralPath $binaryPath -Encoding ASCII
$binarySha = Get-R7EvidenceSha256 $binaryPath
$attestationPath = "$binaryPath.build-attestation.json"
Write-FixtureJson $attestationPath ([ordered]@{
        schema_version = 1
        status = "pass"
        repo_root = $repoRoot
        current_git_head = $sourceCommit
        codex_source_latest_commit = $sourceCommit
        whale_binary_sha256 = $binarySha
    })
Write-FixtureJson (Join-Path $runDir "whale-binary-preflight-health.json") ([ordered]@{
        status = "pass"
        run_validity = "valid"
        build_attestation_status = "pass"
        whale_binary_sha256 = $binarySha
        codex_source_latest_commit = @{ hash = $sourceCommit }
    })
Write-FixtureJson (Join-Path $pairDir "logical-mode-map.json") ([ordered]@{ repeat = 1; left = "standard"; right = "taskspace" })

$standardTrace = [ordered]@{
    base_instructions_identity = [ordered]@{
        count = 1; profile = "standard"; version = $baseContract.profiles.standard.version
        sha256 = $baseContract.profiles.standard.sha256; matches_current_contract = $true
    }
    taskspace_core_protocol_identity = @{ count = 0 }
    taskspace_contract_manifest_identity = @{ count = 0 }
    taskspace_wire_contract_identity = @{ map_handle_count = 0 }
}
$taskspaceTrace = [ordered]@{
    base_instructions_identity = [ordered]@{
        count = 1; profile = "taskspace"; version = $baseContract.profiles.taskspace.version
        sha256 = $baseContract.profiles.taskspace.sha256; matches_current_contract = $true
    }
    taskspace_core_protocol_identity = [ordered]@{
        count = 1; message_index = 1; version = $baseContract.taskspace_core_protocol.version
        sha256 = $baseContract.taskspace_core_protocol.sha256; matches_current_contract = $true
    }
    taskspace_contract_manifest_identity = [ordered]@{
        count = 1; version = $manifest.manifest_version; sha256 = $manifestSha; matches_current_contract = $true
    }
    taskspace_wire_contract_identity = [ordered]@{
        system_message_count = 2; expected_system_message_count = 2; map_handle_count = 1
        map_handle_wire_role = "user"; map_handle_is_request_tail = $true; matches_current_contract = $true
    }
}
($standardTrace | ConvertTo-Json -Compress -Depth 100) | Set-Content -LiteralPath (Join-Path $pairDir "left/artifacts/provider-wire-trace.jsonl") -Encoding UTF8
($taskspaceTrace | ConvertTo-Json -Compress -Depth 100) | Set-Content -LiteralPath (Join-Path $pairDir "right/artifacts/provider-wire-trace.jsonl") -Encoding UTF8

$resultPath = Join-Path $fixtureRoot "result.json"
$result = [ordered]@{
    binary = [ordered]@{ path = $binaryPath; sha256 = $binarySha; attested_codex_source_commit = $sourceCommit; attestation_path = $attestationPath }
    contracts = [ordered]@{
        standard_base = [ordered]@{ version = $baseContract.profiles.standard.version; sha256 = $baseContract.profiles.standard.sha256 }
        taskspace_base = [ordered]@{ version = $baseContract.profiles.taskspace.version; sha256 = $baseContract.profiles.taskspace.sha256 }
        taskspace_core_protocol = [ordered]@{ version = $baseContract.taskspace_core_protocol.version; sha256 = $baseContract.taskspace_core_protocol.sha256 }
        production_manifest = [ordered]@{ version = $manifest.manifest_version; sha256 = $manifestSha }
    }
    runs = @([ordered]@{ run_root = $runDir })
}
Write-FixtureJson $resultPath $result

$pass = Test-R7FiveLayerEvidenceFreshness -RepoRoot $repoRoot -WhaleBin $binaryPath -ResultPath $resultPath -RunRoots @($runDir)
Assert-True ([string]$pass.status -eq "pass") "Fresh evidence fixture did not pass: $(@($pass.findings.stable_code) -join ',')"

$staleResult = Get-Content -Raw -Encoding UTF8 -LiteralPath $resultPath | ConvertFrom-Json -Depth 100
$staleResult.contracts.taskspace_base.version = "stale"
Write-FixtureJson $resultPath $staleResult
$staleContract = Test-R7FiveLayerEvidenceFreshness -RepoRoot $repoRoot -WhaleBin $binaryPath -ResultPath $resultPath -RunRoots @($runDir)
Assert-True ([string]$staleContract.status -eq "fail") "Stale result contract unexpectedly passed"
Assert-True (@($staleContract.findings | Where-Object stable_code -eq "result_taskspace_base_identity_mismatch").Count -eq 1) "Stale result contract did not emit the stable finding"

Write-FixtureJson $resultPath $result
$staleTrace = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $pairDir "right/artifacts/provider-wire-trace.jsonl") | ConvertFrom-Json -Depth 100
$staleTrace.base_instructions_identity.version = "stale"
($staleTrace | ConvertTo-Json -Compress -Depth 100) | Set-Content -LiteralPath (Join-Path $pairDir "right/artifacts/provider-wire-trace.jsonl") -Encoding UTF8
$staleWire = Test-R7FiveLayerEvidenceFreshness -RepoRoot $repoRoot -WhaleBin $binaryPath -ResultPath $resultPath -RunRoots @($runDir)
Assert-True ([string]$staleWire.status -eq "fail") "Stale provider trace unexpectedly passed"
Assert-True (@($staleWire.findings | Where-Object stable_code -eq "taskspace_base_identity_mismatch").Count -eq 1) "Stale provider trace did not emit the stable finding"

Write-Output "R7 five-layer evidence freshness self-test passed."
