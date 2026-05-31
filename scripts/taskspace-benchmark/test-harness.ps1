param(
    [string]$Scenario = "single-file-fast-fix",
    [string]$RunRoot = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $repoRoot "scripts\action-map-real-user-e2e-lib.ps1")
. (Join-Path $repoRoot "scripts\action-map-graph-health-lib.ps1")
. (Join-Path $PSScriptRoot "lib\scenario-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\prompt-guard.ps1")
. (Join-Path $PSScriptRoot "lib\workspace.ps1")
. (Join-Path $PSScriptRoot "lib\oracle-runner.ps1")
. (Join-Path $PSScriptRoot "lib\metrics-extractor.ps1")
. (Join-Path $PSScriptRoot "lib\pair-report.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\paired-bench-selftest" }
$failures = New-Object System.Collections.Generic.List[string]

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { $script:failures.Add($Message) }
}

function Assert-Throws([scriptblock]$Body, [string]$Message) {
    try {
        & $Body
        $script:failures.Add($Message)
    } catch {}
}

$manifest = Read-TaskspaceScenarioManifest $repoRoot $Scenario
Assert-Throws { Assert-TaskspaceManifestField ([pscustomobject]@{ id = "x" }) "prompt_file" } "manifest validation did not reject missing field"

$hardGuard = Invoke-TaskspacePromptGuard -PromptText "Enable taskspace and split the work across multiple agents."
Assert-True ($hardGuard.invalid_prompt) "hard internal prompt token was not invalid"
$allowedGuard = Invoke-TaskspacePromptGuard "Please fix the Node.js source map issue and run parallel tests plus the performance benchmark."
Assert-True (-not $allowedGuard.invalid_prompt) "allowed engineering terms were marked invalid"
Assert-True (-not $allowedGuard.manual_review_required) "allowed engineering terms required manual review"
$mixedGuard = Invoke-TaskspacePromptGuard "Please fix the Node.js source map issue. Then update the node map before implementation."
Assert-True ($mixedGuard.manual_review_required) "benign engineering allowlist suppressed a separate internal node/map leak"
$manualGuard = Invoke-TaskspacePromptGuard "Please run the checks in parallel where it makes sense."
Assert-True ($manualGuard.manual_review_required) "context-sensitive parallel wording did not require manual review"

$runDir = New-TaskspaceBenchmarkRun $RunRoot $manifest.Id
$pairOne = New-TaskspacePairWorkspace $manifest $runDir 1
$pairTwo = New-TaskspacePairWorkspace $manifest $runDir 2
Assert-True ($pairOne.Left.LogicalMode -eq "standard" -and $pairOne.Right.LogicalMode -eq "taskspace") "repeat 1 mode mapping did not use left=standard/right=taskspace"
Assert-True ($pairTwo.Left.LogicalMode -eq "taskspace" -and $pairTwo.Right.LogicalMode -eq "standard") "repeat 2 mode mapping did not alternate"
Assert-True (Test-TaskspaceNeutralCwd $pairOne.Left.RepoDir) "left cwd contains treatment label"
Assert-True (Test-TaskspaceNeutralCwd $pairOne.Right.RepoDir) "right cwd contains treatment label"
Assert-True (-not (Test-TaskspaceNeutralCwd "D:\work\taskspace-benchmark\pair-001\left\repo")) "taskspace-benchmark path was treated as neutral"
$leftPrivateHits = @(Get-ChildItem -LiteralPath $pairOne.Left.RepoDir -Recurse -Force | Where-Object { $_.FullName -match 'private-oracle|reviewer-only' })
$rightPrivateHits = @(Get-ChildItem -LiteralPath $pairOne.Right.RepoDir -Recurse -Force | Where-Object { $_.FullName -match 'private-oracle|reviewer-only' })
Assert-True ($leftPrivateHits.Count -eq 0) "private oracle leaked into left repo"
Assert-True ($rightPrivateHits.Count -eq 0) "private oracle leaked into right repo"

$leakFile = Join-Path $pairOne.Left.ArtifactDir "leak.txt"
Write-Text $leakFile $pairOne.HiddenOraclePath
$leak = Test-TaskspaceOracleLeak $pairOne.Left.RepoDir $pairOne.Left.ArtifactDir $pairOne.HiddenOraclePath $manifest.HiddenOraclePath
Assert-True ($leak.leaked) "oracle path leak test did not detect leaked path"
$repoLeakFile = Join-Path $pairOne.Left.RepoDir "oracle-path-leak.txt"
Write-Text $repoLeakFile $pairOne.HiddenOraclePath
$repoLeak = Test-TaskspaceOracleLeak $pairOne.Left.RepoDir $pairOne.Left.ArtifactDir $pairOne.HiddenOraclePath $manifest.HiddenOraclePath
Assert-True ($repoLeak.leaked) "oracle path leak test did not detect repo-visible leaked path"

$standardArgv = New-TaskspaceWhaleArgv "standard" "model-x" "C:\neutral\left\repo" "C:\neutral\left\last.md"
$taskspaceArgv = New-TaskspaceWhaleArgv "taskspace" "model-x" "C:\neutral\right\repo" "C:\neutral\right\last.md"
$normalizedStandard = Get-NormalizedTaskspaceWhaleArgv $standardArgv
$normalizedTaskspace = @(Get-NormalizedTaskspaceWhaleArgv $taskspaceArgv | Where-Object { $_ -ne "--taskspace" })
Assert-True (($normalizedStandard -join "`n") -eq ($normalizedTaskspace -join "`n")) "standard/taskspace argv differ by more than --taskspace after path normalization"

$promptGuardOk = Invoke-TaskspacePromptGuard "Please fix the failing tax calculation test."
$evidenceRepeatOne = Get-TaskspaceEvidenceGate 1 $promptGuardOk "soft_denylist" "provider-default-or-unknown"
Assert-True ($evidenceRepeatOne.reported_evidence_level -ne "E2") "Repeats 1 + soft_denylist was promoted to E2"
Assert-True (@($evidenceRepeatOne.evidence_gate_failures) -contains "provider_params_unknown") "provider-param observability gap was not recorded"
$softAccepted = Get-TaskspaceEvidenceGate 3 $promptGuardOk "soft_denylist" "known" $false $true $true
Assert-True ($softAccepted.reported_evidence_level -ne "E2") "accepted soft isolation was promoted to E2"
Assert-True (@($softAccepted.evidence_gate_failures) -contains "accepted_soft_isolation_non_e2") "accepted soft isolation failure was not recorded"
$invalidPairEvidence = Get-TaskspaceEvidenceGate 3 $promptGuardOk "hard_sandbox" "known" $true $true
Assert-True ($invalidPairEvidence.reported_evidence_level -eq "E1") "invalid pair was not downgraded to E1"

$metrics = [pscustomobject]@{
    mode = "left"; logical_mode = "standard"; business_success = $true; exec_exit_code = 0
    public_validation_exit_code = 0; hidden_oracle_exit_code = 0; oracle_isolation_level = "hard_sandbox"
    wall_time_ms = 1; tool_call_count = 1; changed_paths = @("src/tax_calc.py")
    maps = 0; nodes = 0; edges = 0; edge_order_violations = 0; spawn_agent_calls = 0
    subagent_results = 0; open_leaf_nodes = 0; ordinary_before_binding = $false
}
$rightMetrics = $metrics.PSObject.Copy()
$rightMetrics.mode = "right"; $rightMetrics.logical_mode = "taskspace"
$reportPath = Join-Path $runDir "manual-review-report.md"
$varControl = [pscustomobject]@{ invalid_pair = $false; failures = @() }
$manualEvidence = Get-TaskspaceEvidenceGate 3 $manualGuard "hard_sandbox" "known"
Write-TaskspacePairReport $reportPath $manifest $manualGuard $varControl $manualEvidence $metrics $rightMetrics $pairOne
$reportText = Get-Content -Raw -Encoding UTF8 -LiteralPath $reportPath
Assert-True ($reportText -match "manual_review_required: True") "manual review requirement was not persisted in pair report"
$summaryPath = Join-Path $runDir "summary.md"
Write-TaskspaceRunSummary -Path $summaryPath -Reports @([pscustomobject]@{ pair_dir = $pairOne.PairDir; pair_report = $reportPath; evidence = [pscustomobject]@{ reported_evidence_level = "E2"; included_in_utility_aggregate = $true } })
$summaryText = Get-Content -Raw -Encoding UTF8 -LiteralPath $summaryPath
Assert-True ($summaryText -match "included_in_utility_aggregate: True") "run summary did not reflect evidence gate aggregate inclusion"

if ($failures.Count -gt 0) {
    Write-Host "TaskSpace benchmark harness self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "TaskSpace benchmark harness self-test: PASS"
Write-Host "RunRoot: $runDir"
