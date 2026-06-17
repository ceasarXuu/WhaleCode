param(
    [string]$Scenario = "single-file-fast-fix",
    [string]$RunRoot = ""
)
$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $repoRoot "scripts\action-map-real-user-e2e-lib.ps1")
. (Join-Path $repoRoot "scripts\action-map-graph-health-lib.ps1")
. (Join-Path $PSScriptRoot "lib\scenario-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\routing-decision.ps1")
. (Join-Path $PSScriptRoot "lib\prompt-guard.ps1")
. (Join-Path $PSScriptRoot "lib\workspace.ps1")
. (Join-Path $PSScriptRoot "lib\oracle-runner.ps1")
. (Join-Path $PSScriptRoot "lib\graph-health.ps1")
. (Join-Path $PSScriptRoot "lib\metrics-extractor.ps1")
. (Join-Path $PSScriptRoot "lib\audit-report.ps1")
. (Join-Path $PSScriptRoot "lib\failure-taxonomy.ps1")
. (Join-Path $PSScriptRoot "lib\audit-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\pair-artifact-classifier.ps1")
. (Join-Path $PSScriptRoot "lib\e3-proof.ps1")
. (Join-Path $PSScriptRoot "lib\pair-report.ps1")
. (Join-Path $PSScriptRoot "lib\report-summary.ps1")
. (Join-Path $PSScriptRoot "lib\aggregate-report.ps1")
. (Join-Path $PSScriptRoot "lib\run-state.ps1")
. (Join-Path $PSScriptRoot "lib\matrix-report.ps1")
. (Join-Path $PSScriptRoot "adapters\external-benchmark-common.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\paired-bench-selftest" }
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { $script:failures.Add($Message) } }
function Assert-Throws([scriptblock]$Body, [string]$Message) {
    try {
        & $Body
        $script:failures.Add($Message)
    } catch {}
}
$aggregateCommand = Get-Command Write-TaskspaceAggregateReport
Assert-True ([string]$aggregateCommand.ScriptBlock.File -like "*lib\aggregate-report.ps1") "aggregate report writer was not loaded from lib\aggregate-report.ps1"

$manifest = Read-TaskspaceScenarioManifest $repoRoot $Scenario
$manifestByPath = Read-TaskspaceScenarioManifest $repoRoot "" $manifest.ScenarioRoot
Assert-True ($manifestByPath.Id -eq $manifest.Id) "ScenarioPath manifest read did not preserve id"
Assert-True ($manifestByPath.ScenarioRoot -eq $manifest.ScenarioRoot) "ScenarioPath manifest read resolved a different root"
Assert-Throws { Assert-TaskspaceManifestField ([pscustomobject]@{ id = "x" }) "prompt_file" } "manifest validation did not reject missing field"
$singleFileRouting = New-TaskspaceRoutingDecision $manifest "Fix the implementation and run tests."
Assert-True ([string]$singleFileRouting.schema_version -eq "TaskShapeRouterV1") "routing decision schema version was not written"
Assert-True ([string]$singleFileRouting.status -eq "report_only") "routing decision should be report-only"
Assert-True ([string]$singleFileRouting.recommended_mode -eq "thin") "single-file scenario did not route to thin"
Assert-True (-not [bool]$singleFileRouting.initial_constraints.subagent_allowed) "thin routing allowed subagents by default"
$verificationManifest = [pscustomobject]@{
    Id = "count-call-stack"; Level = "L1"; HiddenOracleStrategy = "count-call-stack-format-v1"
    PublicValidation = [pscustomobject]@{ command = "python"; args = @("validator.py") }
    Expected = [pscustomobject]@{ max_taskspace_nodes = 4; max_taskspace_spawn_agent_calls = 0 }
}
$verificationRouting = New-TaskspaceRoutingDecision $verificationManifest "Produce exact output format."
Assert-True ([string]$verificationRouting.recommended_mode -eq "verification_first") "format-sensitive scenario did not route to verification_first"
Assert-True ([bool]$verificationRouting.initial_constraints.must_read_validator_first) "verification_first did not require validator-first"

$hardGuard = Invoke-TaskspacePromptGuard -PromptText "Enable taskspace and split the work across multiple agents."
Assert-True ($hardGuard.invalid_prompt) "hard internal prompt token was not invalid"
$allowedGuard = Invoke-TaskspacePromptGuard "Please fix the Node.js source map issue and run parallel tests plus the performance benchmark."
Assert-True (-not $allowedGuard.invalid_prompt) "allowed engineering terms were marked invalid"
Assert-True (-not $allowedGuard.manual_review_required) "allowed engineering terms required manual review"
$mixedGuard = Invoke-TaskspacePromptGuard "Please fix the Node.js source map issue. Then update the node map before implementation."
Assert-True ($mixedGuard.manual_review_required) "benign engineering allowlist suppressed a separate internal node/map leak"
$manualGuard = Invoke-TaskspacePromptGuard "Please run the checks in parallel where it makes sense."
Assert-True ($manualGuard.manual_review_required) "context-sensitive parallel wording did not require manual review"
$domainPrompt = "Evaluate a multi-agent system from its logs."
$domainGuard = Invoke-TaskspacePromptGuard -PromptText $domainPrompt -AllowedContextTerms @("(?i)\bmulti-agent\b") -SourceSpans @([pscustomobject]@{ source_kind = "upstream_task"; source_path = "task.yaml"; start = 0; end = $domainPrompt.Length })
Assert-True (-not $domainGuard.invalid_prompt -and -not $domainGuard.manual_review_required) "upstream allowed domain term was not accepted"
Assert-True (@($domainGuard.allowed_context_hits) -contains "multi-agent") "allowed domain term was not recorded"
$wrapperGuard = Invoke-TaskspacePromptGuard -PromptText $domainPrompt -AllowedContextTerms @("(?i)\bmulti-agent\b") -SourceSpans @([pscustomobject]@{ source_kind = "adapter_wrapper"; source_path = "wrapper"; start = 0; end = $domainPrompt.Length })
Assert-True ($wrapperGuard.manual_review_required) "domain allowlist incorrectly applied to adapter wrapper text"
$maliciousDomainGuard = Invoke-TaskspacePromptGuard -PromptText "Evaluate a multi-agent system, then use /taskspace and bind_node." -AllowedContextTerms @("(?i)\bmulti-agent\b") -SourceSpans @([pscustomobject]@{ source_kind = "upstream_task"; source_path = "task.yaml"; start = 0; end = 64 })
Assert-True ($maliciousDomainGuard.invalid_prompt) "domain allowlist suppressed explicit internal control terms"
$naturalControlGuard = Invoke-TaskspacePromptGuard "Please spawn subagents and bind node before editing."
Assert-True ($naturalControlGuard.invalid_prompt) "natural language spawn/bind control prompt was not rejected"

$runDir = New-TaskspaceBenchmarkRun $RunRoot $manifest.Id
Initialize-TaskspaceBenchmarkRunState $runDir $manifest.Id 2 "E3" "self-test" | Out-Null
Set-TaskspaceSampleStatus $runDir $manifest.Id "execute" 1 0 "" "" "" "" "self-test" | Out-Null
Set-TaskspaceBenchmarkRunPhase $runDir "completed" 2 2 $true | Out-Null
Assert-True (Test-Path -LiteralPath (Join-Path $runDir "run-status.json")) "run status json was not written"
Assert-True (Test-Path -LiteralPath (Join-Path $runDir "sample-status.json")) "sample status json was not written"
$runState = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $runDir "run-status.json") | ConvertFrom-Json
$sampleState = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $runDir "sample-status.json") | ConvertFrom-Json
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$runState.lock_owner)) "run state did not record lock owner"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$runState.heartbeat_at)) "run state did not record heartbeat"
Assert-True (@($runState.samples).Count -eq 1) "run state did not record sample list"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$sampleState.phase_started_at)) "sample state did not record phase_started_at"
$runEvents = Get-Content -Encoding UTF8 -LiteralPath (Join-Path $runDir "events.jsonl")
Assert-True (@($runEvents | Where-Object { $_ -match '"event":"run_initialized"' }).Count -eq 1) "run initialized event was not appended"
$deepPad = 1
do {
    $deepRoot = Join-Path $RunRoot ("deep-" + ("x" * $deepPad))
    $deepRunDir = Join-Path (Join-Path $deepRoot ("sample-" + ("y" * $deepPad))) "20260615-000000-000"
    $deepStatusPath = Join-Path $deepRunDir "sample-status.json"
    $deepPad += 1
} while ($deepStatusPath.Length -lt 224 -and $deepPad -lt 90)
$deepStatusPath = Join-Path $deepRunDir "sample-status.json"
Assert-True ($deepStatusPath.Length -lt 248 -and ("$deepStatusPath.tmp.$([guid]::NewGuid().ToString('N'))").Length -ge 260) "deep path fixture does not exercise atomic temp path boundary"
Write-TaskspaceAtomicJson ([pscustomobject]@{ ok = $true }) $deepStatusPath
Write-TaskspaceRunEvent $deepRunDir "deep_path_event" @{ ok = $true }
Assert-True (Test-Path -LiteralPath $deepStatusPath) "atomic json writer failed on deep run-state path"
Assert-True (Test-Path -LiteralPath (Join-Path $deepRunDir "events.jsonl")) "run event writer failed to create deep run directory"
$resumeRunDir = Join-Path (Join-Path $RunRoot "resume-sample") "20260607-000000-000"
New-Item -ItemType Directory -Path (Join-Path $resumeRunDir "pair-001") -Force | Out-Null
Initialize-TaskspaceBenchmarkRunState $resumeRunDir "resume-sample" 1 "E2" "resume-test" | Out-Null
Set-TaskspaceBenchmarkRunPhase $resumeRunDir "completed" 1 1 $false | Out-Null
"# existing" | Set-Content -LiteralPath (Join-Path $resumeRunDir "pair-001\pair-report.md") -Encoding UTF8
$foundResume = Find-TaskspaceLatestRunDir $RunRoot "resume-sample"
$expectedResume = (Resolve-Path -LiteralPath $resumeRunDir).Path
Assert-True ($foundResume -eq $expectedResume) "latest run finder did not return existing run"
$resumeStatus = Read-TaskspaceRunStatus $resumeRunDir
Assert-True (-not (Test-TaskspaceRunLockStale $resumeStatus)) "fresh completed run was treated as stale"
$resumeStatus.heartbeat_at = (Get-Date).AddHours(-2).ToString("o")
$resumeStatus.stale_after_seconds = 1
Assert-True (Test-TaskspaceRunLockStale $resumeStatus) "stale run lock was not detected"
$pairOne = New-TaskspacePairWorkspace $manifest $runDir 1
$pairTwo = New-TaskspacePairWorkspace $manifest $runDir 2
Assert-True ($pairOne.Left.LogicalMode -eq "standard" -and $pairOne.Right.LogicalMode -eq "taskspace") "repeat 1 mode mapping did not use left=standard/right=taskspace"
Assert-True ($pairTwo.Left.LogicalMode -eq "taskspace" -and $pairTwo.Right.LogicalMode -eq "standard") "repeat 2 mode mapping did not alternate"
Assert-True (-not (Test-Path -LiteralPath $pairOne.ReviewerOracleDir)) "reviewer-only oracle directory was materialized before agent execution"
Assert-True (Test-TaskspaceNeutralCwd $pairOne.Left.RepoDir) "left cwd contains treatment label"
Assert-True (Test-TaskspaceNeutralCwd $pairOne.Right.RepoDir) "right cwd contains treatment label"
Assert-True (-not (Test-TaskspaceNeutralCwd "D:\work\taskspace-benchmark\pair-001\left\repo")) "taskspace-benchmark path was treated as neutral"
$terminalBenchManifest = [pscustomobject]@{
    FixtureDir = $manifest.FixtureDir
    HiddenOracleStrategy = $manifest.HiddenOracleStrategy
    ExternalBenchmark = [pscustomobject]@{
        validator_fidelity = [pscustomobject]@{ validator_runtime = "terminal_bench_docker_app" }
    }
}
$terminalBenchBudgetRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("tbe3-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
$terminalBenchRunDir = Join-Path (Join-Path $terminalBenchBudgetRoot ("suite-" + ("s" * 15))) "samples\analyze-access-logs\runs\terminal_bench__analyze-access-logs\20260615-000000-000"
$terminalBenchPair = New-TaskspacePairWorkspace $terminalBenchManifest $terminalBenchRunDir 1
$worstLooseObjectPath = Join-Path $terminalBenchPair.Left.RepoDir ".git\objects\3e\40a9fe4e99fc548b9421b013c75c8cc706cb9d"
$probeResultPath = Join-Path (Join-Path $terminalBenchPair.Left.ArtifactDir "vprobe") "validator-probe-result.json"
$runtimeResultPath = Join-Path (Join-Path $terminalBenchPair.Left.ArtifactDir "vrun") "validation-cleanup-result.json"
Assert-True ($terminalBenchPair.Left.ExecutionAliasRoot -eq (Split-Path -Parent $terminalBenchPair.Left.RepoDir)) "terminal-bench alias root should map side root directly"
Assert-True ($terminalBenchPair.Left.RepoDir.EndsWith("\left\app") -and -not ($terminalBenchPair.Left.RepoDir -match "terminal-bench-drive")) "terminal-bench repo path was not shortened"
Assert-True ($worstLooseObjectPath.Length -lt 260) "terminal-bench workspace path exceeds Git object path budget: $($worstLooseObjectPath.Length)"
Assert-True ($probeResultPath.Length -lt 260) "terminal-bench probe proof path exceeds Windows path budget: $($probeResultPath.Length)"
Assert-True ($runtimeResultPath.Length -lt 260) "terminal-bench runtime proof path exceeds Windows path budget: $($runtimeResultPath.Length)"
$leftPrivateHits = @(Get-ChildItem -LiteralPath $pairOne.Left.RepoDir -Recurse -Force | Where-Object { $_.FullName -match 'private-oracle|reviewer-only' })
$rightPrivateHits = @(Get-ChildItem -LiteralPath $pairOne.Right.RepoDir -Recurse -Force | Where-Object { $_.FullName -match 'private-oracle|reviewer-only' })
Assert-True ($leftPrivateHits.Count -eq 0) "private oracle leaked into left repo"
Assert-True ($rightPrivateHits.Count -eq 0) "private oracle leaked into right repo"

$leakFile = Join-Path $pairOne.Left.ArtifactDir "leak.txt"
Write-Text $leakFile $pairOne.HiddenOraclePath
$leak = Test-TaskspaceOracleLeak $pairOne.Left.RepoDir $pairOne.Left.ArtifactDir $pairOne.HiddenOraclePath
Assert-True ($leak.leaked) "oracle path leak test did not detect leaked path"
$repoLeakFile = Join-Path $pairOne.Left.RepoDir "oracle-path-leak.txt"
Write-Text $repoLeakFile $pairOne.HiddenOraclePath
$repoLeak = Test-TaskspaceOracleLeak $pairOne.Left.RepoDir $pairOne.Left.ArtifactDir $pairOne.HiddenOraclePath
Assert-True ($repoLeak.leaked) "oracle path leak test did not detect repo-visible leaked path"
$untrackedPath = Join-Path $pairOne.Left.RepoDir "new-output.txt"
Write-Text $untrackedPath "new file"
$nestedUntrackedPath = Join-Path $pairOne.Left.RepoDir "app\hello.txt"
New-Item -ItemType Directory -Path (Split-Path -Parent $nestedUntrackedPath) -Force | Out-Null
Write-Text $nestedUntrackedPath "Hello, world!"
$changedWithUntracked = @(Get-TaskspaceChangedPaths $pairOne.Left.RepoDir "")
Assert-True ($changedWithUntracked -contains "new-output.txt") "changed path detection missed untracked files"
Assert-True ($changedWithUntracked -contains "app/hello.txt") "changed path detection collapsed nested untracked files"
Assert-True (-not ($changedWithUntracked -contains "app/")) "changed path detection reported untracked directory instead of files"
$changedInventory = @(Get-TaskspaceChangedFileInventory $pairOne.Left.RepoDir "")
$helloInventory = @($changedInventory | Where-Object { $_.path -eq "app/hello.txt" })
Assert-True ($helloInventory.Count -eq 1 -and -not [string]::IsNullOrWhiteSpace([string]$helloInventory[0].sha256)) "changed file inventory did not include sha256 for nested untracked file"
$lockedCriticalPath = Join-Path $pairOne.Left.RepoDir "oewn.sqlite"
$lockedStream = [System.IO.File]::Open($lockedCriticalPath, [System.IO.FileMode]::Create, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
try {
    $bytes = [System.Text.Encoding]::UTF8.GetBytes("locked")
    $lockedStream.Write($bytes, 0, $bytes.Length)
    $lockedInventory = @(Get-TaskspaceChangedFileInventory $pairOne.Left.RepoDir "")
    $lockedRow = @($lockedInventory | Where-Object { $_.path -eq "oewn.sqlite" } | Select-Object -First 1)
    Assert-True ($lockedRow.Count -eq 1) "locked critical file was not included in changed inventory"
    Assert-True ([string]$lockedRow[0].hash_status -eq "unavailable_locked") "locked critical file did not record unavailable_locked"
    Assert-True ([bool]$lockedRow[0].critical_artifact) "locked sqlite asset was not marked critical"
} finally {
    $lockedStream.Dispose()
}
$emptyDiffPath = Join-Path $pairOne.Left.ArtifactDir "empty-diff.patch"
Get-TaskspaceDiffText $pairOne.Left.RepoDir $emptyDiffPath | Out-Null
Assert-True (Test-Path -LiteralPath $emptyDiffPath) "empty git diff artifact was not written"
$dockerProofDir = Join-Path $runDir "docker-proof"
New-Item -ItemType Directory -Path $dockerProofDir -Force | Out-Null
$dockerResultPath = Join-Path $dockerProofDir "docker-build-result.json"
@{ phases = @(@{ phase = "build"; exit_code = 17; classification = "docker_build_environment_failure" }) } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $dockerResultPath -Encoding UTF8
$dockerStdout = Join-Path $dockerProofDir "stdout.log"
"docker_build_result_path=$dockerResultPath" | Set-Content -LiteralPath $dockerStdout -Encoding UTF8
$dockerParsed = Get-TaskspaceDockerValidationResult ([pscustomobject]@{ stdout_path = $dockerStdout; stderr_path = "" })
Assert-True (@($dockerParsed.classifications) -contains "docker_build_environment_failure") "docker result classification was not parsed"
$activeSentinelReasons = @(Get-TaskspaceEngineeringUncleanReasons ([pscustomobject]@{
            active_sentinel_warning_count = 1
            active_sentinel_warning_types = @("validator_failure")
        }))
Assert-True ($activeSentinelReasons -contains "active_sentinel_warning:validator_failure") "active sentinel warning was not treated as engineering-unclean"
$cleanupFailurePath = Join-Path $dockerProofDir "validation-cleanup-result.json"
@{ classification = "docker_cleanup_container_failure"; container_name = "whale-tbench-0123456789abcdef" } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $cleanupFailurePath -Encoding UTF8
$cleanupStdout = Join-Path $dockerProofDir "cleanup-stdout.log"
"validation_cleanup_result_path=$cleanupFailurePath" | Set-Content -LiteralPath $cleanupStdout -Encoding UTF8
$cleanupParsed = Get-TaskspaceDockerValidationResult ([pscustomobject]@{ stdout_path = $cleanupStdout; stderr_path = "" })
Assert-True (@($cleanupParsed.classifications) -contains "docker_cleanup_container_failure") "validation cleanup failure classification was not parsed"
Assert-True ([string]$cleanupParsed.cleanup_path -eq $cleanupFailurePath) "validation cleanup result path was not preserved"
$timeoutParsed = Get-TaskspaceDockerValidationResult ([pscustomobject]@{ exit_code = 124; stdout_path = ""; stderr_path = "" })
Assert-True (@($timeoutParsed.classifications) -contains "public_validation_timeout") "public validation timeout classification was not parsed"

$graphObs = [pscustomobject]@{
    nodes = @(
        [pscustomobject]@{
            id = "node-1"; kind = "inspect_code_context"; status = "completed"; title = "Investigate"
            leases = @([pscustomobject]@{ agentThreadId = "agent-1" })
            results = @(
                [pscustomobject]@{ resultId = "result-1"; kind = "result"; validity = "accepted"; sourceThreadId = "agent-1"; evidencePackage = [pscustomobject]@{ adoptionState = "accepted_not_adopted" } },
                [pscustomobject]@{ resultId = "result-2"; kind = "main_tool_call"; validity = "unreviewed"; sourceThreadId = "agent-1"; evidencePackage = [pscustomobject]@{} },
                [pscustomobject]@{ resultId = "result-3"; kind = "result"; validity = "unreviewed"; sourceThreadId = "agent-1"; evidencePackage = [pscustomobject]@{} }
            )
            events = @([pscustomobject]@{ to = "completed"; at = "2026-06-11T00:00:00Z" })
        },
        [pscustomobject]@{
            id = "node-2"; kind = "final_synthesis"; status = "ready"; title = "Synthesize"
            leases = @()
            results = @()
            events = @()
        }
    )
    edges = @([pscustomobject]@{ from = "node-1"; to = "node-2" })
    toolCalls = @([pscustomobject]@{ tool = "spawn_agent"; status = "completed" })
    timeline = @()
}
$graphReport = New-TaskspaceGraphHealthReport $graphObs "right" "taskspace"
Assert-True ([string]$graphReport.schema_version -eq "taskspace-graph-health-v1") "graph health report schema version missing"
Assert-True ($graphReport.node_count -eq 2 -and $graphReport.edge_count -eq 1) "graph health report did not count nodes/edges"
Assert-True (@($graphReport.warnings) -contains "high_unreviewed_result_ratio") "graph health did not flag high unreviewed result ratio"
Assert-True ([int]$graphReport.reviewable_result_count -eq 2) "graph health did not isolate reviewable semantic results"
Assert-True ([int]$graphReport.reviewable_unreviewed_result_count -eq 1) "graph health counted non-reviewable tool traces as reviewable"
Assert-True (@($graphReport.warnings) -contains "subagent_no_adoption") "graph health did not flag unused subagent result"
Assert-True (@($graphReport.warnings) -contains "subagent_no_decision_yield") "graph health did not flag missing subagent decision yield"
Assert-True ([double]$graphReport.subagent_decision_yield -eq 0.0) "graph health counted ordinary adoption as decision yield"
Assert-True ([string]$graphReport.metric_availability.result_adoption -eq "measured") "graph health did not expose result adoption metric availability"
$nonReviewableObs = [pscustomobject]@{
    nodes = @(
        [pscustomobject]@{
            id = "node-1"; kind = "inspect_code_context"; status = "completed"; title = "Investigate"
            leases = @()
            results = @(
                [pscustomobject]@{ resultId = "result-1"; kind = "main_tool_call"; validity = "unreviewed"; evidencePackage = [pscustomobject]@{} }
            )
            events = @()
        },
        [pscustomobject]@{
            id = "node-2"; kind = "final_synthesis"; status = "completed"; title = "Synthesize"
            leases = @()
            results = @(
                [pscustomobject]@{ resultId = "result-2"; kind = "result"; validity = "unreviewed"; evidencePackage = [pscustomobject]@{} }
            )
            events = @()
        }
    )
    edges = @([pscustomobject]@{ from = "node-1"; to = "node-2" })
    toolCalls = @()
    timeline = @()
}
$nonReviewableReport = New-TaskspaceGraphHealthReport $nonReviewableObs "right" "taskspace"
Assert-True (-not (@($nonReviewableReport.warnings) -contains "high_unreviewed_result_ratio")) "graph health counted tool/final summary results as reviewable debt"
Assert-True ([int]$nonReviewableReport.reviewable_result_count -eq 0) "graph health reviewable result count included tool/final summary results"
$graphDecisionObs = [pscustomobject]@{
    nodes = @(
        [pscustomobject]@{
            id = "node-1"; kind = "inspect_code_context"; status = "completed"; title = "Investigate"
            leases = @()
            results = @(
                [pscustomobject]@{
                    resultId = "result-1"; validity = "accepted"; sourceThreadId = "agent-1"; subagentPlanId = "subagent-plan-1"
                    evidencePackage = [pscustomobject]@{
                        adoptionState = "accepted_adopted"
                        adoption = [pscustomobject]@{ adoptedByDecisions = @("decision-1") }
                    }
                },
                [pscustomobject]@{
                    resultId = "result-2"; validity = "accepted"; sourceThreadId = "agent-2"; subagentPlanId = "subagent-plan-2"
                    evidencePackage = [pscustomobject]@{ adoptionState = "accepted_adopted" }
                }
            )
            events = @()
        }
    )
    maps = @([pscustomobject]@{ subagentPlans = @([pscustomobject]@{ id = "subagent-plan-1" }, [pscustomobject]@{ id = "subagent-plan-2" }) })
    edges = @()
    decisions = @([pscustomobject]@{ id = "decision-1"; dependsOnResults = @("result-1") })
    toolCalls = @()
    timeline = @()
}
$graphDecisionReport = New-TaskspaceGraphHealthReport $graphDecisionObs "right" "taskspace"
Assert-True ([double]$graphDecisionReport.subagent_decision_yield -eq 0.5) "graph health decision yield should count only decision-supported accepted subagent results"
Assert-True (-not (@($graphDecisionReport.warnings) -contains "subagent_no_decision_yield")) "graph health emitted missing-yield warning despite decision support"
$graphStaleDecisionObs = [pscustomobject]@{
    nodes = @(
        [pscustomobject]@{
            id = "node-1"; kind = "inspect_code_context"; status = "completed"; title = "Investigate"
            leases = @()
            results = @([pscustomobject]@{
                    resultId = "result-1"; validity = "accepted"; sourceThreadId = "agent-1"; subagentPlanId = "subagent-plan-1"
                    evidencePackage = [pscustomobject]@{
                        adoptionState = "accepted_adopted"
                        adoption = [pscustomobject]@{ adoptedByDecisions = @("deleted-decision") }
                    }
                })
            events = @()
        }
    )
    maps = @([pscustomobject]@{ subagentPlans = @([pscustomobject]@{ id = "subagent-plan-1" }) })
    edges = @()
    decisions = @([pscustomobject]@{ id = "decision-1"; dependsOnResults = @("result-1") })
    toolCalls = @()
    timeline = @()
}
$graphStaleDecisionReport = New-TaskspaceGraphHealthReport $graphStaleDecisionObs "right" "taskspace"
Assert-True ([double]$graphStaleDecisionReport.subagent_decision_yield -eq 0.0) "graph health counted stale decision adoption as decision yield"
Assert-True (@($graphStaleDecisionReport.warnings) -contains "subagent_no_decision_yield") "graph health did not warn on stale decision adoption"
$legacyGraphObs = [pscustomobject]@{
    nodes = @([pscustomobject]@{
            id = "legacy-node"; kind = "inspect_code_context"; status = "completed"; title = "Legacy"
            leases = @([pscustomobject]@{ agentThreadId = "legacy-agent" })
            results = @([pscustomobject]@{ resultId = "legacy-result"; validity = "accepted"; sourceThreadId = "legacy-agent"; evidencePackage = [pscustomobject]@{} })
            events = @()
        })
    edges = @()
    toolCalls = @([pscustomobject]@{ tool = "spawn_agent"; status = "completed" })
    timeline = @()
}
$legacyGraphReport = New-TaskspaceGraphHealthReport $legacyGraphObs "right" "taskspace"
Assert-True ([string]$legacyGraphReport.metric_availability.result_adoption -eq "unsupported_legacy") "legacy graph health did not mark adoption as unsupported"
Assert-True ($null -eq $legacyGraphReport.result_adoption_rate) "legacy graph health reported unsupported adoption as measured zero"
Assert-True (-not (@($legacyGraphReport.warnings) -contains "subagent_no_adoption")) "legacy graph health emitted subagent adoption warning without adoption support"
$thinGraphReport = New-TaskspaceGraphHealthReport ([pscustomobject]@{
        nodes = @([pscustomobject]@{ id = "node-1"; kind = "inspect_code_context"; status = "completed"; leases = @(); results = @(); events = @() })
        edges = @()
        toolCalls = @()
        timeline = @()
    }) "right" "taskspace"
Assert-True (-not (@($thinGraphReport.warnings) -contains "thin_mode_violation")) "thin taskspace run without spawn was incorrectly warned"

$costDir = Join-Path $runDir "cost-instrumentation"
New-Item -ItemType Directory -Path $costDir -Force | Out-Null
$costJsonl = Join-Path $costDir "exec.jsonl"
$costObs = Join-Path $costDir "observability.json"
@{
    events = @(
        @{ kind = "tool_result"; text = "OutputReferenceV1:`nartifact_ref: output-ref://sha256/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" },
        @{ kind = "tool_result"; text = "OutputSliceV1:`nartifact_ref: output-ref://sha256/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa" }
    )
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $costDir "taskspace.graph.final.json") -Encoding UTF8
@(
    (@{ type = "response.completed"; response = @{ usage = @{ input_tokens = 120; output_tokens = 30; input_tokens_details = @{ cached_tokens = 20 } } } } | ConvertTo-Json -Compress -Depth 8),
    (@{ payload = @{ name = "taskspace_control"; arguments = '{"action":"start_task","title":"x"}' } } | ConvertTo-Json -Compress -Depth 8),
    (@{ payload = @{ name = "taskspace_control"; arguments = '{"action":"finish_node","node_id":"node-1"}' } } | ConvertTo-Json -Compress -Depth 8),
    (@{ type = "response.completed"; response = @{ usage = @{ output_tokens = 7 } } } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath $costJsonl -Encoding UTF8
@(
    (@{ type = "event_msg"; payload = @{ type = "token_count"; info = @{ last_token_usage = @{ input_tokens = 100; cached_input_tokens = 10; output_tokens = 11 } } } } | ConvertTo-Json -Compress -Depth 8),
    (@{ type = "event_msg"; payload = @{ type = "token_count"; info = @{ last_token_usage = @{ input_tokens = 300; cached_input_tokens = 240; output_tokens = 21 } } } } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath (Join-Path $costDir "rollout.jsonl") -Encoding UTF8
[pscustomobject]@{
    timeline = @(
        [pscustomobject]@{ kind = "task_created" },
        [pscustomobject]@{ kind = "cognitive_state_updated"; details = [pscustomobject]@{ updateKind = "state_commit.partial" } },
        [pscustomobject]@{ kind = "output_ref.created" },
        [pscustomobject]@{ kind = "output_ref.slice_read" },
        [pscustomobject]@{ kind = "node_status_changed" },
        [pscustomobject]@{ kind = "tool:spawn_agent" }
    )
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $costObs -Encoding UTF8
$costArtifacts = Write-TaskspaceCostInstrumentationArtifacts $costDir $costJsonl $costObs
Assert-True (Test-Path -LiteralPath $costArtifacts.token_summary_path) "token-summary.json was not written"
Assert-True (Test-Path -LiteralPath $costArtifacts.request_summary_path) "request-summary.json was not written"
Assert-True (Test-Path -LiteralPath $costArtifacts.taskspace_control_usage_path) "taskspace-control-usage.json was not written"
Assert-True (Test-Path -LiteralPath $costArtifacts.context_projection_summary_path) "context-projection-summary.json was not written"
Assert-True (Test-Path -LiteralPath $costArtifacts.projection_events_path) "projection-events.jsonl was not written"
Assert-True ([string]$costArtifacts.token_summary.availability -eq "partial") "partial token usage was not marked partial"
Assert-True ([int]$costArtifacts.request_summary.model_request_count -eq 2) "model request count did not come from usage events"
Assert-True ([int]$costArtifacts.request_summary.max_input_tokens_per_request -eq 120) "max input tokens per request was not reported"
Assert-True ([int]$costArtifacts.request_summary.p95_input_tokens_per_request -eq 120) "p95 input tokens per request was not reported"
Assert-True ([int]$costArtifacts.request_summary.first_input_tokens_per_request -eq 120) "first input tokens per request was not reported"
Assert-True ([int]$costArtifacts.request_summary.max_output_tokens_per_request -eq 30) "max output tokens per request was not reported"
Assert-True ([int]$costArtifacts.request_summary.p95_output_tokens_per_request -eq 30) "p95 output tokens per request was not reported"
Assert-True ([int]$costArtifacts.request_summary.last_output_tokens_per_request -eq 7) "last output tokens per request was not reported"
Assert-True ([string]$costArtifacts.request_summary.rollout_trace.availability -eq "measured") "rollout request trace was not measured"
Assert-True ([int]$costArtifacts.request_summary.rollout_trace.model_request_count -eq 2) "rollout request trace count was not parsed"
Assert-True ([int]$costArtifacts.request_summary.rollout_trace.input_tokens -eq 400) "rollout request trace input tokens were not summed"
Assert-True ([int]$costArtifacts.request_summary.rollout_trace.max_input_tokens_per_request -eq 300) "rollout max input tokens per request was not reported"
Assert-True ([int]$costArtifacts.request_summary.rollout_trace.last_input_tokens_per_request -eq 300) "rollout last input tokens per request was not reported"
Assert-True ([int]$costArtifacts.taskspace_control_usage.taskspace_control_count -eq 2) "taskspace_control count was not parsed"
Assert-True ([int]$costArtifacts.taskspace_control_usage.action_counts.start_task -eq 1) "taskspace_control start_task action was not counted"
Assert-True ([int]$costArtifacts.taskspace_control_usage.action_counts.finish_node -eq 1) "taskspace_control finish_node action was not counted"
Assert-True ([int]$costArtifacts.taskspace_control_usage.taskspace_runtime_event_count -eq 5) "taskspace runtime event count was not parsed from observability"
Assert-True ([int]$costArtifacts.taskspace_control_usage.runtime_state_commit_count -eq 1) "runtime state_commit event count was not parsed from observability"
Assert-True ([int]$costArtifacts.taskspace_control_usage.runtime_output_ref_created_count -eq 1) "runtime output_ref.created count was not parsed from observability"
Assert-True ([int]$costArtifacts.taskspace_control_usage.runtime_output_ref_slice_read_count -eq 1) "runtime output_ref.slice_read count was not parsed from observability"
Assert-True ([int]$costArtifacts.taskspace_control_usage.runtime_event_counts.node_status_changed -eq 1) "runtime event kind was not counted"
Assert-True ([int]$costArtifacts.replay_summary.output_reference_count -eq 1) "output reference count was not parsed"
Assert-True ([int]$costArtifacts.replay_summary.output_slice_count -eq 1) "output slice count was not parsed"
Assert-True ([int]$costArtifacts.replay_summary.large_output_replay_count -eq 0) "output reference artifact was incorrectly treated as raw replay"
$projectionJsonl = Join-Path $costDir "projection-source.jsonl"
$activeProjectionBlock = @"
ContextProjectionV1 active replacement:
- projection_id: projection-active-task-1-map-1
- task_id: task-1
- mode: default_compact
- active_objective: Verify projection metrics.
- sections:
  success_criteria:
    - projected criteria
  current_node: node-1
  blockers:
    - none
  decisions:
    - decision: use active compact profile
  facts:
    - fact: output refs active
  relevant_results:
    - result:abc
  next_valid_actions:
    - inspect_code_context
  hidden_refs_available:
    - result:abc
- estimated_tokens: 123
"@
$shadowProjectionBlock = @"
ContextProjectionV1 shadow (not active replacement):
- projection_id: projection-shadow-task-1-map-1
- task_id: task-1
- mode: default_compact
- active_objective: Verify projection metrics.
- sections:
  success_criteria:
    - projected criteria
  current_node: node-1
  blockers:
    - none
  decisions:
    - decision: keep shadow mode
  facts:
    - fact: output refs active
  relevant_results:
    - result:abc
  next_valid_actions:
    - inspect_code_context
  hidden_refs_available:
    - result:abc
- estimated_tokens: 123
"@
@(
    (@{ type = "response.created"; input = $activeProjectionBlock } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath $projectionJsonl -Encoding UTF8
$projectionArtifacts = Write-TaskspaceCostInstrumentationArtifacts (Join-Path $costDir "projection") $projectionJsonl ""
Assert-True ([string]$projectionArtifacts.context_projection_summary.availability -eq "measured") "context projection summary was not marked measured"
Assert-True ([int]$projectionArtifacts.context_projection_summary.projection_count -eq 1) "context projection block was not counted"
Assert-True ([int]$projectionArtifacts.context_projection_summary.projection_tokens_total -eq 123) "context projection tokens were not parsed"
Assert-True ([int]$projectionArtifacts.context_projection_summary.protected_miss_count -eq 0) "context projection protected sections were reported missing"
$projectionEventLine = Get-Content -LiteralPath $projectionArtifacts.projection_events_path -Encoding UTF8 | Select-Object -First 1
$projectionEvent = $projectionEventLine | ConvertFrom-Json
Assert-True ([string]$projectionEvent.projection_id -eq "projection-active-task-1-map-1") "active projection event id was not parsed"
Assert-True ([string]$projectionEvent.projection_kind -eq "active_replacement") "active projection kind was not parsed"
$shadowProjectionJsonl = Join-Path $costDir "projection-shadow-source.jsonl"
@(
    (@{ type = "response.created"; input = $shadowProjectionBlock } | ConvertTo-Json -Compress -Depth 8)
) | Set-Content -LiteralPath $shadowProjectionJsonl -Encoding UTF8
$shadowProjectionArtifacts = Write-TaskspaceCostInstrumentationArtifacts (Join-Path $costDir "projection-shadow") $shadowProjectionJsonl ""
Assert-True ([int]$shadowProjectionArtifacts.context_projection_summary.projection_count -eq 1) "legacy shadow projection block was not counted"
$shadowProjectionEventLine = Get-Content -LiteralPath $shadowProjectionArtifacts.projection_events_path -Encoding UTF8 | Select-Object -First 1
$shadowProjectionEvent = $shadowProjectionEventLine | ConvertFrom-Json
Assert-True ([string]$shadowProjectionEvent.projection_kind -eq "shadow") "legacy shadow projection kind was not parsed"
$costObsFallback = Join-Path $costDir "observability-output-ref-fallback.json"
[pscustomobject]@{
    timeline = @(
        [pscustomobject]@{
            kind = "tool_result"
            body = "OutputReferenceV1:`nartifact_ref: output-ref://sha256/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        }
    )
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $costObsFallback -Encoding UTF8
$costFallbackArtifacts = Write-TaskspaceCostInstrumentationArtifacts (Join-Path $costDir "fallback") $costJsonl $costObsFallback
Assert-True ([int]$costFallbackArtifacts.taskspace_control_usage.runtime_output_ref_created_count -eq 1) "runtime output_ref.created fallback was not inferred from observability OutputReferenceV1"
$rawReplayDir = Join-Path $runDir "raw-replay-cost"
New-Item -ItemType Directory -Path $rawReplayDir -Force | Out-Null
@{ events = @(@{ text = ("middle-secret-marker`n" * 350) }) } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $rawReplayDir "taskspace.graph.final.json") -Encoding UTF8
$rawReplayArtifacts = Write-TaskspaceCostInstrumentationArtifacts $rawReplayDir "" ""
Assert-True ([int]$rawReplayArtifacts.replay_summary.raw_large_marker_count -eq 1) "raw large marker replay was not counted"
Assert-True ([bool]$rawReplayArtifacts.replay_summary.raw_output_in_prompt_violation) "raw large marker replay did not set violation flag"
$missingCostArtifacts = Write-TaskspaceCostInstrumentationArtifacts (Join-Path $runDir "missing-cost") ""
Assert-True ([string]$missingCostArtifacts.token_summary.availability -eq "source_missing") "missing usage source was not marked source_missing"
Assert-True ($null -eq $missingCostArtifacts.request_summary.model_request_count) "missing usage source was treated as zero requests"
$costAggregateRoot = Join-Path $runDir "cost-aggregate"
New-Item -ItemType Directory -Path (Join-Path $costAggregateRoot "pair-001\left\artifacts") -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $costAggregateRoot "pair-001\right\artifacts") -Force | Out-Null
[pscustomobject]@{
    logical_mode = "standard"; token_summary_availability = "measured"; model_request_count = 10
    input_tokens = 1000; output_tokens = 200; cached_input_tokens = 100; uncached_input_tokens = 900
    wall_time_ms = 1000; taskspace_control_count = 0; state_commit_count = 0; runtime_state_commit_count = 0; runtime_output_ref_created_count = 0; runtime_output_ref_slice_read_count = 0; large_output_replay_count = 0
    projection_count = 0; projection_tokens = 0; projection_protected_miss_count = 0
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $costAggregateRoot "pair-001\left\artifacts\metrics.json") -Encoding UTF8
[pscustomobject]@{
    logical_mode = "taskspace"; token_summary_availability = "measured"; model_request_count = 20
    input_tokens = 1800; output_tokens = 500; cached_input_tokens = 300; uncached_input_tokens = 1500
    wall_time_ms = 1900; taskspace_control_count = 8; state_commit_count = 2; runtime_state_commit_count = 3; runtime_output_ref_created_count = 4; runtime_output_ref_slice_read_count = 2; large_output_replay_count = 0
    taskspace_runtime_event_count = 17
    projection_count = 3; projection_tokens = 240; projection_protected_miss_count = 0
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $costAggregateRoot "pair-001\right\artifacts\metrics.json") -Encoding UTF8
$aggregateCost = Write-TaskspaceCostAggregateArtifacts -RootDir $costAggregateRoot -Scope "sample"
Assert-True (Test-Path -LiteralPath $aggregateCost.token_summary_path) "aggregate token-summary.json was not written"
Assert-True (Test-Path -LiteralPath $aggregateCost.request_summary_path) "aggregate request-summary.json was not written"
Assert-True (Test-Path -LiteralPath $aggregateCost.taskspace_control_usage_path) "aggregate taskspace-control-usage.json was not written"
Assert-True (Test-Path -LiteralPath $aggregateCost.context_projection_summary_path) "aggregate context-projection-summary.json was not written"
Assert-True (Test-Path -LiteralPath $aggregateCost.suite_cost_gate_path) "suite-cost-gate.json was not written"
Assert-True ([string]$aggregateCost.gate.status -eq "PASS") "cost gate did not pass when direct and walltime ratios were <= 2x"
$aggregateControl = Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregateCost.taskspace_control_usage_path | ConvertFrom-Json
Assert-True ([int]$aggregateControl.taskspace_runtime_event_count -eq 17) "aggregate runtime event count was not summed"
Assert-True ([int]$aggregateControl.runtime_state_commit_count -eq 3) "aggregate runtime state_commit count was not summed"
Assert-True ([int]$aggregateControl.runtime_output_ref_created_count -eq 4) "aggregate runtime output_ref.created count was not summed"
Assert-True ([int]$aggregateControl.runtime_output_ref_slice_read_count -eq 2) "aggregate runtime output_ref.slice_read count was not summed"
$aggregateProjection = Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregateCost.context_projection_summary_path | ConvertFrom-Json
Assert-True ([int]$aggregateProjection.taskspace_projection_count -eq 3) "aggregate projection count was not summed"
Assert-True ([int]$aggregateProjection.taskspace_projection_tokens -eq 240) "aggregate projection tokens were not summed"
Assert-True ([int]$aggregateProjection.taskspace_projection_protected_miss_count -eq 0) "aggregate projection protected miss count was not summed"
$partialMetricPath = Join-Path $costAggregateRoot "pair-001\right\artifacts\metrics.json"
[pscustomobject]@{
    logical_mode = "taskspace"; token_summary_availability = "measured"; model_request_count = 24
    input_tokens = 2600; output_tokens = 500; cached_input_tokens = 300; uncached_input_tokens = 2300
    wall_time_ms = 2600; taskspace_control_count = 8; state_commit_count = 2; runtime_state_commit_count = 3; large_output_replay_count = 0
    projection_count = 3; projection_tokens = 280; projection_protected_miss_count = 0
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $partialMetricPath -Encoding UTF8
$partialGate = (Write-TaskspaceCostAggregateArtifacts -RootDir $costAggregateRoot -Scope "sample").gate
Assert-True ([string]$partialGate.status -eq "PARTIAL") "cost gate did not return PARTIAL for engineering partial thresholds"
[pscustomobject]@{
    logical_mode = "taskspace"; token_summary_availability = "partial"; model_request_count = 30
    output_tokens = 500; wall_time_ms = 4000; taskspace_control_count = 8; state_commit_count = 2; runtime_state_commit_count = 3; large_output_replay_count = 0
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $partialMetricPath -Encoding UTF8
$missingGate = (Write-TaskspaceCostAggregateArtifacts -RootDir $costAggregateRoot -Scope "sample").gate
Assert-True ([string]$missingGate.status -eq "FAIL" -and [string]$missingGate.reason -eq "missing_cost_data") "cost gate did not fail closed on missing input tokens"

$standardArgv = New-TaskspaceWhaleArgv "standard" "model-x" "C:\neutral\left\repo" "C:\neutral\left\last.md"
$taskspaceArgv = New-TaskspaceWhaleArgv "taskspace" "model-x" "C:\neutral\right\repo" "C:\neutral\right\last.md"
$normalizedStandard = Get-NormalizedTaskspaceWhaleArgv $standardArgv
$normalizedTaskspace = @(Get-NormalizedTaskspaceWhaleArgv $taskspaceArgv | Where-Object { $_ -ne "--taskspace" })
Assert-True (($normalizedStandard -join "`n") -eq ($normalizedTaskspace -join "`n")) "standard/taskspace argv differ by more than --taskspace after path normalization"

$promptGuardOk = Invoke-TaskspacePromptGuard "Please fix the failing tax calculation test."
$evidenceRepeatOne = Get-TaskspaceEvidenceGate 1 $promptGuardOk "soft_denylist" "provider-default-or-unknown"
Assert-True ($evidenceRepeatOne.reported_evidence_level -ne "E2") "Repeats 1 + soft_denylist was promoted to E2"
Assert-True (@($evidenceRepeatOne.evidence_gate_failures) -contains "provider_params_incomplete") "provider-param observability gap was not recorded"
$softAccepted = Get-TaskspaceEvidenceGate 3 $promptGuardOk "soft_denylist" "known" $false $true $true
Assert-True ($softAccepted.reported_evidence_level -ne "E2") "accepted soft isolation was promoted to E2"
Assert-True (@($softAccepted.evidence_gate_failures) -contains "accepted_soft_isolation_non_e2") "accepted soft isolation failure was not recorded"
$invalidPairEvidence = Get-TaskspaceEvidenceGate 3 $promptGuardOk "hard_sandbox" "known" $true $true
Assert-True ($invalidPairEvidence.reported_evidence_level -eq "E1") "invalid pair was not downgraded to E1"
$partialProvider = [pscustomobject]@{ complete = $false; missing = @("model_reasoning_effort") }
$partialProviderEvidence = Get-TaskspaceEvidenceGate 3 $promptGuardOk "hard_sandbox" $partialProvider
Assert-True ($partialProviderEvidence.reported_evidence_level -ne "E2") "partial provider config was promoted to E2"
$deferredStrictEvidence = Get-TaskspaceEvidenceGate 3 $promptGuardOk "hard_deferred_materialization" "known" $false $true $false $true "hard_sandbox_only"
Assert-True (@($deferredStrictEvidence.evidence_gate_failures) -contains "oracle_isolation_deferred_not_allowed") "deferred oracle isolation was not distinct in strict policy"
$e3Origin = [pscustomobject]@{
    type = "historical_whale_failure"
    source = "sanitized_user_session"
    source_date = "2026-06-02"
    sanitized = $true
    privacy_review_completed = $true
    sanitization_summary = "Removed local user paths and private project identifiers."
    privacy_risk_summary = "No secrets or private business data remain."
    original_prompt_sha256 = "abc123"
}
$e3Config = [pscustomobject]@{ claim_scope = "historical Whale runtime failure sample"; minimum_repeats = 5 }
$e3Candidate = Get-TaskspaceEvidenceGate 3 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $e3Origin $null $e3Config $true $false 5 "" $false
Assert-True ($e3Candidate.reported_evidence_level -eq "E3-candidate") "incomplete E3 evidence was not downgraded to E3-candidate"
Assert-True (@($e3Candidate.e3_gate_failures) -contains "e3_repeats_lt_5") "E3 repeats gate failure was not recorded"
Assert-True (@($e3Candidate.e3_gate_failures) -contains "e3_human_review_not_completed") "E3 human review gate failure was not recorded"
$e3WeakOrigin = [pscustomobject]@{ type = "historical_whale_failure"; source = "sanitized_user_session" }
$e3Weak = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $e3WeakOrigin $null $e3Config $true $true 5 "include_taskspace_better" $false
Assert-True (@($e3Weak.e3_gate_failures) -contains "e3_original_prompt_sha_missing") "E3 prompt checksum gate failure was not recorded"
Assert-True (@($e3Weak.e3_gate_failures) -contains "e3_historical_sample_not_sanitized") "E3 sanitized gate failure was not recorded"
Assert-True (@($e3Weak.e3_gate_failures) -contains "e3_privacy_review_not_completed") "E3 privacy review gate failure was not recorded"
Assert-True (@($e3Weak.e3_gate_failures) -contains "e3_sanitization_summary_missing") "E3 sanitization summary gate failure was not recorded"
Assert-True (@($e3Weak.e3_gate_failures) -contains "e3_privacy_risk_summary_missing") "E3 privacy risk summary gate failure was not recorded"
$e3InvalidOrigin = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $null $null $e3Config $true $true 5 "include_taskspace_better" $false
Assert-True (@($e3InvalidOrigin.e3_gate_failures) -contains "e3_sample_origin_missing_or_invalid") "E3 sample origin gate failure was not recorded"
$e3MissingScope = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $e3Origin $null $null $true $true 5 "include_taskspace_better" $false
Assert-True (@($e3MissingScope.e3_gate_failures) -contains "e3_claim_scope_missing") "E3 claim scope gate failure was not recorded"
$e3LowRepeat = Get-TaskspaceEvidenceGate 4 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $e3Origin $null ([pscustomobject]@{ claim_scope = "scope"; minimum_repeats = 3 }) $true $true 3 "include_taskspace_better" $false
Assert-True (@($e3LowRepeat.e3_gate_failures) -contains "e3_repeats_lt_5") "E3 minimum repeats was allowed below 5"
$e3BadReviewDecision = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $e3Origin $null $e3Config $true $true 5 "" $false
Assert-True (@($e3BadReviewDecision.e3_gate_failures) -contains "e3_human_review_decision_missing_or_invalid") "E3 review decision gate failure was not recorded"
$e3ExcludedReviewDecision = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $e3Origin $null $e3Config $true $true 5 "exclude_validator_unclear" $false
Assert-True (@($e3ExcludedReviewDecision.e3_gate_failures) -contains "e3_human_review_excluded_pair") "E3 excluded review decision was not recorded"
Assert-True (-not $e3ExcludedReviewDecision.included_in_e3_aggregate) "E3 excluded review decision entered aggregate"
$externalOrigin = [pscustomobject]@{
    type = "external_benchmark"
    source = "terminal-bench"
    source_version = "pinned-revision"
    source_url = "https://example.invalid/terminal-bench"
    license = "external-license"
    data_policy = "pointer_only_no_solution_or_hidden_tests"
    sample_id = "sample-001"
    original_prompt_sha256 = "abc123"
    original_validator_sha256 = "def456"
}
$validExternalFidelity = [pscustomobject]@{
    official_runner_or_equivalent = $true
    docker_runtime = $true
    container_workdir = "/app"
    validator_runtime = "official_or_equivalent_docker"
    agent_cannot_read_validator_source = $true
    e3_eligible = $true
    downgrade_reason = ""
}
$externalBenchmark = [pscustomobject]@{ name = "terminal-bench"; adapter_version = "whale-taskspace-e3-adapter-v1"; validator_fidelity = $validExternalFidelity }
$validExternalProof = [pscustomobject]@{ validator_fidelity = $validExternalFidelity; runtime_proof_path = "runtime.json"; isolation_proof_path = "isolation.json"; combined_proof_path = "combined.json" }
$completeSideOutcomes = [pscustomobject]@{ standard_success = $true; taskspace_success = $true }
$e3ExternalReady = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $externalOrigin $externalBenchmark $e3Config $true $true 5 "include_no_clear_delta" $false $validExternalProof $completeSideOutcomes
Assert-True ($e3ExternalReady.reported_evidence_level -eq "E3") "complete external E3 evidence did not promote to E3"
$e3ExternalMissingOutcomes = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $externalOrigin $externalBenchmark $e3Config $true $true 5 "include_no_clear_delta" $false $validExternalProof
Assert-True (@($e3ExternalMissingOutcomes.e3_gate_failures) -contains "e3_side_outcomes_missing") "external E3 without side outcomes was not fail-closed"
$e3ExternalTimeout = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $externalOrigin $externalBenchmark $e3Config $true $true 5 "include_taskspace_better" $false $validExternalProof ([pscustomobject]@{ standard_success = $false; taskspace_success = $true; exec_timeouts = @("left/taskspace") })
Assert-True (@($e3ExternalTimeout.e3_gate_failures) -contains "e3_exec_timeout") "external E3 timeout side was promoted to E3"
$invalidExternalBenchmark = [pscustomobject]@{
    name = "terminal-bench"
    adapter_version = "whale-taskspace-e3-adapter-v1"
    validator_fidelity = [pscustomobject]@{
        official_runner_or_equivalent = $false
        docker_runtime = $false
        container_workdir = ""
        validator_runtime = "windows_git_bash_non_docker"
        agent_cannot_read_validator_source = $false
        e3_eligible = $false
        downgrade_reason = "engineering smoke only"
    }
}
$e3ExternalUnfaithful = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $externalOrigin $invalidExternalBenchmark $e3Config $true $true 5 "include_no_clear_delta" $false $null $completeSideOutcomes
Assert-True ($e3ExternalUnfaithful.reported_evidence_level -ne "E3") "unfaithful external validator was promoted to E3"
Assert-True (@($e3ExternalUnfaithful.e3_gate_failures) -contains "e3_external_validator_fidelity_unproven") "external validator fidelity gate failure was not recorded"
Assert-True (@($e3ExternalUnfaithful.e3_gate_failures) -contains "e3_external_validator_source_not_isolated") "external validator source isolation gate failure was not recorded"
$externalOriginMissingRevision = $externalOrigin.PSObject.Copy()
$externalOriginMissingRevision.source_version = ""
$e3ExternalMissingRevision = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $externalOriginMissingRevision $externalBenchmark $e3Config $true $true 5 "include_no_clear_delta" $false $validExternalProof $completeSideOutcomes
Assert-True (@($e3ExternalMissingRevision.e3_gate_failures) -contains "e3_external_source_version_missing") "external E3 source revision gate failure was not recorded"
$e3Ready = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $e3Origin $null $e3Config $true $true 5 "include_taskspace_better" $false $null ([pscustomobject]@{ standard_success = $false; taskspace_success = $true })
Assert-True ($e3Ready.reported_evidence_level -eq "E3") "complete E3 evidence did not promote to E3"
Assert-True ($e3Ready.included_in_e3_aggregate) "complete E3 evidence was not included in E3 aggregate"

$auditPairDir = Join-Path $runDir "audit-pair"
New-Item -ItemType Directory -Path $auditPairDir | Out-Null
$requiredAuditArtifacts = @(
        "manifest.resolved.json",
        "left/artifacts/metrics.json",
        "right/artifacts/metrics.json",
        "left/artifacts/whale-exec.jsonl",
        "right/artifacts/whale-exec.jsonl",
        "left/artifacts/validation.stdout.log",
        "right/artifacts/validation.stdout.log",
        "left/artifacts/git-diff.patch",
        "right/artifacts/git-diff.patch",
        "right/artifacts/observability/action-map-observability.json"
    )
foreach ($relative in $requiredAuditArtifacts) {
    $path = Join-Path $auditPairDir $relative
    New-Item -ItemType Directory -Path (Split-Path -Parent $path) -Force | Out-Null
    "artifact" | Set-Content -LiteralPath $path -Encoding UTF8
}
$auditHashes = [ordered]@{}
foreach ($relative in $requiredAuditArtifacts) {
    $auditHashes[$relative] = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $auditPairDir $relative)).Hash.ToLowerInvariant()
}
$auditJsonPath = Join-Path $auditPairDir "audit-review.json"
@{
    reviewer = "codex"
    date = "2026-06-02"
    artifact_basis = $requiredAuditArtifacts
    artifact_hashes = $auditHashes
    decision = "include_no_clear_delta"
    claim_scope = "self-test audit"
    disagreement = $false
} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $auditJsonPath -Encoding UTF8
$audit = Get-TaskspaceAuditReview $auditPairDir "" 0 "self-test audit"
Assert-True ($audit.completed) "complete audit review sidecar was not accepted"
Assert-True ($audit.decision -eq "include_no_clear_delta") "audit decision was not parsed"
"changed artifact" | Set-Content -LiteralPath (Join-Path $auditPairDir "left/artifacts/metrics.json") -Encoding UTF8
$staleHashAudit = Get-TaskspaceAuditReview $auditPairDir "" 0 "self-test audit"
Assert-True (-not $staleHashAudit.completed) "stale audit with outdated artifact hash was accepted"
Assert-True (@($staleHashAudit.failures | Where-Object { $_ -eq "audit_artifact_hash_mismatch:left/artifacts/metrics.json" }).Count -eq 1) "stale audit artifact hash mismatch was not reported"
"artifact" | Set-Content -LiteralPath (Join-Path $auditPairDir "left/artifacts/metrics.json") -Encoding UTF8
$badAuditJsonPath = Join-Path $auditPairDir "audit-review.json"
@{
    reviewer = "codex"
    date = "2026-06-02"
    artifact_basis = @("left/artifacts/metrics.json")
    decision = "include_taskspace_better"
    claim_scope = "self-test audit"
} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $badAuditJsonPath -Encoding UTF8
$badAudit = Get-TaskspaceAuditReview $auditPairDir "" 0 "self-test audit"
Assert-True (-not $badAudit.completed) "hollow audit with only pair-report was accepted"
Assert-True (@($badAudit.failures | Where-Object { $_ -like "audit_required_artifact_missing:*" }).Count -gt 0) "audit required artifact failures were not reported"
@{
    reviewer = "codex"
    date = "2026-06-02"
    artifact_basis = @($auditJsonPath)
    decision = "include_taskspace_better"
    claim_scope = "self-test audit"
} | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $badAuditJsonPath -Encoding UTF8
$staleAudit = Get-TaskspaceAuditReview $auditPairDir "" 0 "self-test audit"
Assert-True (-not $staleAudit.completed) "audit with absolute artifact path was accepted"
Assert-True (@($staleAudit.failures | Where-Object { $_ -like "audit_artifact_path_not_pair_relative:*" }).Count -eq 1) "audit absolute artifact path was not rejected"
$auditRoot = Join-Path $runDir "generic-audit-root"
New-Item -ItemType Directory -Path $auditRoot | Out-Null
Copy-Item -LiteralPath $badAuditJsonPath -Destination (Join-Path $auditRoot "audit-review.json") -Force
$auditNoLocalPairDir = Join-Path $runDir "audit-no-local-pair"
New-Item -ItemType Directory -Path $auditNoLocalPairDir | Out-Null
$ignoredGenericAudit = Get-TaskspaceAuditReview $auditNoLocalPairDir $auditRoot 1 "self-test audit"
Assert-True (-not $ignoredGenericAudit.completed) "generic AuditReviewRoot\\audit-review.json was accepted"
Assert-True (@($ignoredGenericAudit.failures) -contains "audit_review_missing") "generic AuditReviewRoot audit fallback was not ignored"
$leakyFixture = Join-Path $runDir "leaky-fixture"
New-Item -ItemType Directory -Path $leakyFixture | Out-Null
"secret" | Set-Content -LiteralPath (Join-Path $leakyFixture "solution.py") -Encoding UTF8
"secret" | Set-Content -LiteralPath (Join-Path $leakyFixture "answer.txt") -Encoding UTF8
"secret" | Set-Content -LiteralPath (Join-Path $leakyFixture "gold.patch") -Encoding UTF8
New-Item -ItemType Directory -Path (Join-Path $leakyFixture "nested\private-tests") -Force | Out-Null
"secret" | Set-Content -LiteralPath (Join-Path $leakyFixture "nested\private-tests\case.txt") -Encoding UTF8
$hiddenFixture = Join-Path $runDir "hidden-fixture"
New-Item -ItemType Directory -Path $hiddenFixture | Out-Null
"secret" | Set-Content -LiteralPath (Join-Path $hiddenFixture "hidden-test") -Encoding UTF8
New-Item -ItemType Directory -Path (Join-Path $hiddenFixture "private") | Out-Null
New-Item -ItemType Directory -Path (Join-Path $hiddenFixture "hidden") | Out-Null
$cleanDest = Join-Path $runDir "clean-fixture"
Assert-Throws { Copy-TaskspaceExternalFixture $leakyFixture $cleanDest | Out-Null } "external fixture materialization accepted solution/private files"
Assert-True (-not (Test-Path -LiteralPath (Join-Path $cleanDest "solution.py"))) "external fixture copied files before failing leak scan"
Assert-Throws { Copy-TaskspaceExternalFixture $hiddenFixture (Join-Path $runDir "hidden-clean-fixture") | Out-Null } "external fixture materialization accepted hidden files"
$terminalBenchNoEnv = Join-Path $runDir "terminal-bench-no-env"
New-Item -ItemType Directory -Path $terminalBenchNoEnv | Out-Null
@'
instruction: |-
  Create a file called hello.txt.
category: file-operations
'@ | Set-Content -LiteralPath (Join-Path $terminalBenchNoEnv "task.yaml") -Encoding UTF8
"FROM scratch" | Set-Content -LiteralPath (Join-Path $terminalBenchNoEnv "Dockerfile") -Encoding UTF8
"do not leak" | Set-Content -LiteralPath (Join-Path $terminalBenchNoEnv "solution.sh") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $terminalBenchNoEnv "run-tests.sh") -Encoding UTF8
New-Item -ItemType Directory -Path (Join-Path $terminalBenchNoEnv "task-deps\nested") -Force | Out-Null
"public" | Set-Content -LiteralPath (Join-Path $terminalBenchNoEnv "task-deps\input.csv") -Encoding UTF8
"private test" | Set-Content -LiteralPath (Join-Path $terminalBenchNoEnv "task-deps\nested\run-tests.sh") -Encoding UTF8
New-Item -ItemType Directory -Path (Join-Path $terminalBenchNoEnv "task-deps\tests") -Force | Out-Null
"private" | Set-Content -LiteralPath (Join-Path $terminalBenchNoEnv "task-deps\tests\case.py") -Encoding UTF8
$adapterOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $terminalBenchNoEnv -OutputRoot (Join-Path $runDir "external-out") -SampleId "no-env" -SourceVersion "pinned"
$adapterScenarioDir = [string]($adapterOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
Assert-True (Test-Path -LiteralPath (Join-Path $adapterScenarioDir "prompt.txt")) "terminal-bench adapter did not extract task.yaml instruction"
Assert-True (-not (Test-Path -LiteralPath (Join-Path $adapterScenarioDir "fixture\solution.sh"))) "terminal-bench adapter leaked solution.sh from official task root"
Assert-True (Test-Path -LiteralPath (Join-Path $adapterScenarioDir "fixture\task-deps\input.csv")) "terminal-bench adapter dropped public task-deps fixture"
Assert-True (-not (Test-Path -LiteralPath (Join-Path $adapterScenarioDir "fixture\task-deps\nested\run-tests.sh"))) "terminal-bench adapter leaked nested validator script"
Assert-True (-not (Test-Path -LiteralPath (Join-Path $adapterScenarioDir "fixture\task-deps\tests\case.py"))) "terminal-bench adapter leaked nested tests directory"
$adapterScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $adapterScenarioDir "scenario.json") | ConvertFrom-Json
Assert-True (-not [bool]$adapterScenario.external_benchmark.validator_fidelity.e3_eligible) "terminal-bench post-hoc Docker validator was over-promoted to E3 eligible"
Assert-True ([string]$adapterScenario.external_benchmark.validator_fidelity.validator_runtime -eq "terminal_bench_equivalent_docker_app") "terminal-bench validator runtime was not equivalent Docker /app"
Assert-True ([bool]$adapterScenario.external_benchmark.validator_fidelity.agent_cannot_read_validator_source) "terminal-bench validator source guard declaration was not recorded"
Assert-True ([bool]$adapterScenario.external_benchmark.validator_fidelity.docker_runtime) "terminal-bench Docker runtime capability was not recorded"
Assert-True ([string]$adapterScenario.external_benchmark.adapter_metadata.instruction_extraction_mode -eq "literal") "terminal-bench literal instruction mode was not recorded"
Assert-True (@($adapterScenario.prompt_guard.source_spans).Count -eq 2) "terminal-bench prompt guard source spans were not recorded"
Assert-True (@($adapterScenario.external_benchmark.adapter_metadata.generated_fixture_allowlist) -contains "task-deps/input.csv") "terminal-bench recursive fixture allowlist missed public file"
$adapterValidatorText = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $adapterScenarioDir "external-validator.ps1")
Assert-True ($adapterValidatorText -match "proxy_env_skipped_loopback") "terminal-bench validator did not guard WSL loopback proxy injection"
Assert-True ($adapterValidatorText -match "Invoke-DockerBackendProbe") "terminal-bench validator did not time-bound Docker backend probing"
Assert-True ($adapterValidatorText -match "Requested native Docker backend is unavailable") "terminal-bench validator did not validate native Docker wrapper availability"
Assert-True ($adapterValidatorText -match "Test-DockerCommandIsWslWrapper") "terminal-bench validator did not detect WSL docker command wrappers"
Assert-True ($adapterValidatorText -match "getpwnam\\\(root\\\) failed") "terminal-bench validator did not classify WSL root lookup backend failures"
$suiteRunnerText = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $PSScriptRoot "run-taskspace-e3-suite.ps1")
$suiteStatusText = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $PSScriptRoot "lib\suite-status.ps1")
Assert-True (($suiteRunnerText + $suiteStatusText) -match "child_process_failed") "E3 suite runner did not classify child process failures"
$terminalBenchEnv = Join-Path $runDir "terminal-bench-env"
New-Item -ItemType Directory -Path (Join-Path $terminalBenchEnv "environment") -Force | Out-Null
@'
instruction: "Create hello.txt."
category: file-operations
'@ | Set-Content -LiteralPath (Join-Path $terminalBenchEnv "task.yaml") -Encoding UTF8
"FROM scratch" | Set-Content -LiteralPath (Join-Path $terminalBenchEnv "environment\Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $terminalBenchEnv "run-tests.sh") -Encoding UTF8
$envOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $terminalBenchEnv -OutputRoot (Join-Path $runDir "external-env-out") -SampleId "env" -SourceVersion "pinned"
$envScenarioDir = [string]($envOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$envScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $envScenarioDir "scenario.json") | ConvertFrom-Json
Assert-True (@($envScenario.external_benchmark.adapter_metadata.generated_fixture_allowlist) -contains "environment") "terminal-bench environment fixture metadata was not stable"
$aliasRoot = Join-Path $runDir "terminal-bench-alias-root"
New-Item -ItemType Directory -Path (Join-Path $aliasRoot "app") -Force | Out-Null
$aliasSide = [pscustomobject]@{ RepoDir = (Join-Path $aliasRoot "app"); ExecutionAliasRoot = $aliasRoot }
$aliasMount = Mount-TaskspaceExecutionAlias $aliasSide
try {
    powershell -NoProfile -Command "Set-Location '$($aliasMount.execution_repo_dir)'; New-Item -ItemType File -Force -Path /app/subst-smoke.txt | Out-Null"
} finally {
    Dismount-TaskspaceExecutionAlias $aliasMount
}
Assert-True (Test-Path -LiteralPath (Join-Path $aliasRoot "app\subst-smoke.txt")) "Terminal-Bench /app execution alias did not map to repo root"
$metrics = [pscustomobject]@{
    mode = "left"; logical_mode = "standard"; business_success = $true; exec_exit_code = 0
    exec_timed_out = $false
    public_validation_exit_code = 0; hidden_oracle_exit_code = 0; oracle_isolation_level = "hard_sandbox"
    wall_time_ms = 1; tool_call_count = 1; changed_paths = @("src/tax_calc.py")
    changed_file_inventory = @([pscustomobject]@{ path = "src/tax_calc.py"; status = "M "; source = "git_status"; sha256 = "abc123"; size_bytes = 12 })
    validator_environment_mismatch = $false
    maps = 0; nodes = 0; edges = 0; edge_order_violations = 0; spawn_agent_calls = 0
    subagent_results = 0; open_leaf_nodes = 0; ordinary_before_binding = $false
    graph_health_path = ""; graph_health_warnings = @(); decision_count = 0; decision_density = 0.0
    accepted_results = 0; unreviewed_results = 0; questioned_or_invalid_results = 0
    result_adoption_rate = 0.0; subagent_decision_yield = 0.0
}
$rightMetrics = $metrics.PSObject.Copy()
$rightMetrics.mode = "right"; $rightMetrics.logical_mode = "taskspace"
$rightGraphPath = Join-Path $pairOne.Right.ArtifactDir "graph-health.json"
Write-TaskspaceGraphHealthReport $graphReport $rightGraphPath
$rightMetrics.graph_health_path = $rightGraphPath
$rightMetrics.graph_health_warnings = @($graphReport.warnings)
$rightMetrics.nodes = [int]$graphReport.node_count
$rightMetrics.edges = [int]$graphReport.edge_count
$rightMetrics.decision_count = [int]$graphReport.decision_count
$rightMetrics.decision_density = [double]$graphReport.decision_density
$rightMetrics.unreviewed_results = [int]$graphReport.unreviewed_result_count
$taxonomy = @(Get-TaskspaceFailureTaxonomy $metrics $rightMetrics)
Assert-True (@($taxonomy) -contains "subagent_noise_or_unused") "failure taxonomy did not use graph health subagent signal"
Assert-True ((Get-TaskspaceUtilityDirection $metrics $rightMetrics $taxonomy) -eq "both_success") "utility direction did not classify both-success pair"
$timeoutMetrics = $metrics.PSObject.Copy()
$timeoutMetrics.business_success = $false
$timeoutMetrics.public_validation_exit_code = 124
$timeoutMetrics.changed_paths = @()
$timeoutMetrics | Add-Member -NotePropertyName validator_environment_failures -NotePropertyValue @("public_validation_timeout") -Force
$timeoutTaxonomy = @(Get-TaskspaceFailureTaxonomy $timeoutMetrics $rightMetrics)
Assert-True (@($timeoutTaxonomy) -contains "validator_slow_or_flaky") "timeout taxonomy did not classify validator slowness"
Assert-True (-not (@($timeoutTaxonomy) -contains "agent_no_patch")) "timeout taxonomy incorrectly added agent_no_patch"
$execTimeoutMetrics = $metrics.PSObject.Copy()
$execTimeoutMetrics.business_success = $false
$execTimeoutMetrics | Add-Member -NotePropertyName exec_timed_out -NotePropertyValue $true -Force
$execTimeoutMetrics.changed_paths = @()
$execTimeoutTaxonomy = @(Get-TaskspaceFailureTaxonomy $execTimeoutMetrics $rightMetrics)
Assert-True (-not (@($execTimeoutTaxonomy) -contains "agent_no_patch")) "exec timeout taxonomy incorrectly added agent_no_patch"
$environmentOnlyMetrics = $metrics.PSObject.Copy()
$environmentOnlyMetrics.business_success = $false
$environmentOnlyMetrics.public_validation_exit_code = 1
$environmentOnlyMetrics.changed_paths = @("src/example.py")
$environmentOnlyMetrics | Add-Member -NotePropertyName validator_environment_failures -NotePropertyValue @("docker_cleanup_container_failure") -Force
$environmentOnlyTaxonomy = @(Get-TaskspaceFailureTaxonomy $environmentOnlyMetrics $rightMetrics)
Assert-True (@($environmentOnlyTaxonomy) -contains "environment_noise") "environment-only taxonomy did not classify environment noise"
Assert-True (-not (@($environmentOnlyTaxonomy) -contains "agent_patch_wrong")) "environment-only taxonomy incorrectly added agent_patch_wrong"
$mismatchedOutcomeLeft = $metrics.PSObject.Copy()
$mismatchedOutcomeRight = $rightMetrics.PSObject.Copy()
$mismatchedOutcomeRight.hidden_oracle_exit_code = 1
$mismatchedOutcomeControl = Compare-TaskspacePairVariables ([pscustomobject]@{
    prompt_sha256_left = "same"; prompt_sha256_right = "same"
    fixture_sha256_left = "same"; fixture_sha256_right = "same"
    whale_sha256_left = "same"; whale_sha256_right = "same"
    model_left = "same"; model_right = "same"
    timeout_seconds_left = 1; timeout_seconds_right = 1
}) $mismatchedOutcomeLeft $mismatchedOutcomeRight
Assert-True (-not $mismatchedOutcomeControl.invalid_pair) "different validator outcomes were incorrectly treated as variable-control failures"
$reportPath = Join-Path $runDir "manual-review-report.md"
$varControl = [pscustomobject]@{ invalid_pair = $false; failures = @() }
$manualEvidence = Get-TaskspaceEvidenceGate 3 $manualGuard "hard_sandbox" "known"
$manualEvidence | Add-Member -NotePropertyName failure_taxonomy -NotePropertyValue @("subagent_noise_or_unused") -Force
$manualEvidence | Add-Member -NotePropertyName utility_direction -NotePropertyValue "both_success" -Force
$manifestResolvedForAudit = [pscustomobject]@{
    repeat = 1
    scenario = "manual-review"
    human_review_required = $false
}
$auditResult = Write-TaskspaceAuditManifest $pairOne.PairDir $manifestResolvedForAudit $metrics $rightMetrics $manualEvidence $varControl $null
Assert-True (Test-Path -LiteralPath $auditResult.json_path) "audit manifest json was not written"
Assert-True (Test-Path -LiteralPath $auditResult.yaml_path) "audit manifest yaml was not written"
$auditJson = Get-Content -Raw -Encoding UTF8 -LiteralPath $auditResult.json_path | ConvertFrom-Json
Assert-True ([string]$auditJson.audit_version -eq "taskspace-e3-audit-v1") "audit manifest schema version missing"
Assert-True ([string]$auditJson.classification.utility_direction -eq "both_success") "audit manifest utility direction missing"
$manualEvidence | Add-Member -NotePropertyName audit_manifest_path -NotePropertyValue $auditResult.json_path -Force
Write-TaskspacePairReport $reportPath $manifest $manualGuard $varControl $manualEvidence $metrics $rightMetrics $pairOne
$reportText = Get-Content -Raw -Encoding UTF8 -LiteralPath $reportPath
Assert-True ($reportText -match "manual_review_required: True") "manual review requirement was not persisted in pair report"
Assert-True ($reportText -match "audit_manifest_path:") "pair report did not include audit manifest path"
Assert-True ($reportText -match "failure_taxonomy: subagent_noise_or_unused") "pair report did not include failure taxonomy"
$summaryPath = Join-Path $runDir "summary.md"
Write-TaskspaceRunSummary -Path $summaryPath -Reports @([pscustomobject]@{ pair_dir = $pairOne.PairDir; pair_report = $reportPath; evidence = [pscustomobject]@{ reported_evidence_level = "E2"; included_in_utility_aggregate = $true } })
$summaryText = Get-Content -Raw -Encoding UTF8 -LiteralPath $summaryPath
Assert-True ($summaryText -match "included_in_utility_aggregate: True") "run summary did not reflect evidence gate aggregate inclusion"
Assert-True ($summaryText -notmatch "included_in_e3_aggregate") "E2 run summary emitted E3 aggregate noise"
$e3CandidateSummaryPath = Join-Path $runDir "summary-e3-candidate.md"
Write-TaskspaceRunSummary -Path $e3CandidateSummaryPath -Reports @([pscustomobject]@{ pair_dir = $pairOne.PairDir; pair_report = $reportPath; evidence_target = "E3"; evidence = $e3ExternalUnfaithful })
$e3CandidateSummaryText = Get-Content -Raw -Encoding UTF8 -LiteralPath $e3CandidateSummaryPath
Assert-True ($e3CandidateSummaryText -match "included_in_e3_aggregate: False") "E3 candidate summary did not show E3 aggregate exclusion"
$e3FailedSummaryPath = Join-Path $runDir "summary-e3-failed.md"
Write-TaskspaceRunSummary -Path $e3FailedSummaryPath -Reports @([pscustomobject]@{ pair_dir = $pairOne.PairDir; pair_report = $reportPath; evidence_target = "E3"; evidence = [pscustomobject]@{ reported_evidence_level = "E1"; included_in_utility_aggregate = $false; included_in_e3_aggregate = $false } })
$e3FailedSummaryText = Get-Content -Raw -Encoding UTF8 -LiteralPath $e3FailedSummaryPath
Assert-True ($e3FailedSummaryText -match "included_in_e3_aggregate: False") "E3 failed summary did not show E3 aggregate exclusion"
Assert-True (-not (Test-TaskspaceEvidenceSatisfiesTarget "E3" "E3-candidate")) "E3-candidate satisfied E3 target"
Assert-True (Test-TaskspaceEvidenceSatisfiesTarget "E3" "E3") "E3 did not satisfy E3 target"
Assert-True (-not (Test-TaskspaceEvidenceSatisfiesTarget "E2" "E2-candidate")) "E2-candidate satisfied E2 target"
$singleFailedReportList = New-Object System.Collections.Generic.List[object]
$singleFailedReportList.Add([pscustomobject]@{ evidence = [pscustomobject]@{ reported_evidence_level = "E1" } })
$singleFailedReports = @(Get-TaskspaceFailedReports $singleFailedReportList "E3")
Assert-True ($singleFailedReports.Count -eq 1) "single failed E3 report did not remain countable after array normalization"
$aggregateFixtureDirPath = Join-Path $RunRoot "aggregate-fixture"
New-Item -ItemType Directory -Force -Path $aggregateFixtureDirPath | Out-Null
$aggregateFixtureDir = (Resolve-Path -LiteralPath $aggregateFixtureDirPath).Path
$aggregateReportPath = [System.IO.Path]::Combine([string]$aggregateFixtureDir, "aggregate.md")
if ([string]::IsNullOrWhiteSpace($aggregateReportPath)) {
    throw "aggregate fixture path was empty; RunRoot=[$RunRoot] fixture=[$aggregateFixtureDir]"
}
Write-TaskspaceAggregateReport -Path $aggregateReportPath -Reports @(
    [pscustomobject]@{ repeat = 1; pair_report = "one.md"; evidence = [pscustomobject]@{ reported_evidence_level = "E2"; included_in_utility_aggregate = $true; evidence_gate_failures = @() } },
    [pscustomobject]@{ repeat = 2; pair_report = "two.md"; evidence = [pscustomobject]@{ reported_evidence_level = "E1"; included_in_utility_aggregate = $false; evidence_gate_failures = @("oracle_isolation_failed") } }
)
$aggregateText = Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregateReportPath
Assert-True ($aggregateText -match "valid_utility_pairs: 1") "aggregate did not count only E2 utility pairs"
Assert-True ($aggregateText -match "excluded_pairs: 1") "aggregate did not exclude non-E2 pair"
Assert-True ($aggregateText -match "valid_e3_pairs: 0") "aggregate did not emit explicit E3 zero count"
$aggregateJson = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $aggregateFixtureDir "aggregate.json") | ConvertFrom-Json
Assert-True ([int]$aggregateJson.valid_utility_pairs -eq 1) "aggregate json did not count valid utility pairs"
$pairIndexJson = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $aggregateFixtureDir "pair-index.json") | ConvertFrom-Json
Assert-True (@($pairIndexJson).Count -eq 2) "pair index json did not include all pairs"
$e3AggregateReportPath = [System.IO.Path]::Combine([string]$aggregateFixtureDir, "aggregate-e3.md")
Write-TaskspaceAggregateReport -Path $e3AggregateReportPath -Reports @(
    [pscustomobject]@{ repeat = 1; pair_report = "e3-one.md"; evidence = $e3Ready },
    [pscustomobject]@{ repeat = 2; pair_report = "e3-two.md"; evidence = $e3Candidate }
)
$e3AggregateText = Get-Content -Raw -Encoding UTF8 -LiteralPath $e3AggregateReportPath
Assert-True ($e3AggregateText -match "valid_e3_pairs: 1") "E3 aggregate did not count complete E3 pairs"
Assert-True ($e3AggregateText -match "e3_human_review_not_completed") "E3 aggregate did not preserve E3 gate failures"
Assert-True ($e3AggregateText -match "e3_human_review_completed_pairs: 1") "E3 aggregate did not count human review completion"
Assert-True ($e3AggregateText -match "include_taskspace_better=1") "E3 aggregate did not summarize human review decisions"
Assert-True ($e3AggregateText -match "e3_taskspace_better_pairs: 1") "E3 aggregate did not count directional TaskSpace benefit"
Assert-True ($e3AggregateText -match "e3_standard_better_pairs: 0") "E3 aggregate did not separate standard-better pairs"
Assert-True ($e3AggregateText -match "only include_taskspace_better counts") "E3 aggregate did not include directional benefit warning"
$resumeClassifyPair = New-Dir (Join-Path $runDir "resume-reclassifies-existing\pair-001")
$resumeResolved = [pscustomobject]@{
    scenario = "resume-reclassifies-existing"
    repeat = 1
    prompt_sha256_left = "same"; prompt_sha256_right = "same"
    fixture_sha256_left = "same"; fixture_sha256_right = "same"
    whale_sha256_left = "same"; whale_sha256_right = "same"
    model_left = "same"; model_right = "same"
    timeout_seconds_left = 1; timeout_seconds_right = 1
    provider_param_status = "known"
    oracle_isolation_policy = "deferred_materialization_allowed"
    sample_origin = $null
    external_benchmark = $null
    e3 = $null
    human_review_required = $false
}
Write-TaskspaceJson $resumeResolved (Join-Path $resumeClassifyPair "manifest.resolved.json")
foreach ($side in @("left", "right")) {
    New-Dir (Join-Path $resumeClassifyPair "$side\repo") | Out-Null
    New-Dir (Join-Path $resumeClassifyPair "$side\artifacts") | Out-Null
    Write-Text (Join-Path $resumeClassifyPair "$side\artifacts\validation.stdout.log") ""
    Write-Text (Join-Path $resumeClassifyPair "$side\artifacts\validation.stderr.log") ""
    Write-Text (Join-Path $resumeClassifyPair "$side\artifacts\git-diff.patch") ""
}
$resumeLeftMetrics = $metrics.PSObject.Copy()
$resumeLeftMetrics.mode = "left"; $resumeLeftMetrics.logical_mode = "standard"; $resumeLeftMetrics.business_success = $true
$resumeLeftMetrics.public_validation_exit_code = 0; $resumeLeftMetrics.hidden_oracle_exit_code = 0
$resumeLeftMetrics | Add-Member -NotePropertyName validator_environment_failures -NotePropertyValue @() -Force
$resumeLeftMetrics | Add-Member -NotePropertyName metrics_taints -NotePropertyValue @() -Force
$resumeRightMetrics = $rightMetrics.PSObject.Copy()
$resumeRightMetrics.mode = "right"; $resumeRightMetrics.logical_mode = "taskspace"; $resumeRightMetrics.business_success = $true
$resumeRightMetrics.public_validation_exit_code = 0; $resumeRightMetrics.hidden_oracle_exit_code = 0
$resumeRightMetrics | Add-Member -NotePropertyName validator_environment_failures -NotePropertyValue @() -Force
$resumeRightMetrics | Add-Member -NotePropertyName metrics_taints -NotePropertyValue @() -Force
Write-TaskspaceJson $resumeLeftMetrics (Join-Path $resumeClassifyPair "left\artifacts\metrics.json")
Write-TaskspaceJson $resumeRightMetrics (Join-Path $resumeClassifyPair "right\artifacts\metrics.json")
$resumeClassified = Get-TaskspacePairEvidenceFromArtifacts $resumeClassifyPair 3 $promptGuardOk $true "" "E2"
Assert-True ([bool]$resumeClassified.evidence.included_in_utility_aggregate) "resume classifier did not preserve valid utility evidence"
Assert-True (-not (@($resumeClassified.evidence.evidence_gate_failures) -contains "resumed_existing_pair_not_reclassified")) "resume classifier kept placeholder failure"
$resumeAggregatePath = Join-Path (Split-Path -Parent $resumeClassifyPair) "aggregate-report.md"
Write-TaskspaceAggregateReport -Path $resumeAggregatePath -Reports @([pscustomobject]@{ repeat = 1; pair_dir = $resumeClassifyPair; pair_report = (Join-Path $resumeClassifyPair "pair-report.md"); evidence = $resumeClassified.evidence })
$resumeAggregate = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path (Split-Path -Parent $resumeClassifyPair) "aggregate.json") | ConvertFrom-Json
Assert-True ([int]$resumeAggregate.valid_utility_pairs -eq 1) "resume aggregate did not include reclassified pair"

$outputRefContractPair = New-Dir (Join-Path $runDir "output-ref-contract\pair-001")
$outputRefManifest = $resumeResolved.PSObject.Copy()
$outputRefManifest.scenario = "output-ref-contract"
$outputRefManifest | Add-Member -NotePropertyName expected -NotePropertyValue ([pscustomobject]@{ min_taskspace_runtime_output_ref_created_count = 1 }) -Force
Write-TaskspaceJson $outputRefManifest (Join-Path $outputRefContractPair "manifest.resolved.json")
foreach ($side in @("left", "right")) {
    New-Dir (Join-Path $outputRefContractPair "$side\repo") | Out-Null
    New-Dir (Join-Path $outputRefContractPair "$side\artifacts") | Out-Null
    Write-Text (Join-Path $outputRefContractPair "$side\artifacts\validation.stdout.log") ""
    Write-Text (Join-Path $outputRefContractPair "$side\artifacts\validation.stderr.log") ""
    Write-Text (Join-Path $outputRefContractPair "$side\artifacts\git-diff.patch") ""
}
$outputRefLeftMetrics = $resumeLeftMetrics.PSObject.Copy()
$outputRefRightMetrics = $resumeRightMetrics.PSObject.Copy()
$outputRefRightMetrics | Add-Member -NotePropertyName runtime_output_ref_created_count -NotePropertyValue 0 -Force
$outputRefRightMetrics.metrics_taints = @()
Write-TaskspaceJson $outputRefLeftMetrics (Join-Path $outputRefContractPair "left\artifacts\metrics.json")
Write-TaskspaceJson $outputRefRightMetrics (Join-Path $outputRefContractPair "right\artifacts\metrics.json")
$outputRefClassified = Get-TaskspacePairEvidenceFromArtifacts $outputRefContractPair 3 $promptGuardOk $true "" "E2"
Assert-True (@($outputRefClassified.right_metrics.metrics_taints) -contains "scenario_expected_runtime_output_ref_created_count_not_met:0<1") "output-ref scenario contract did not taint taskspace metrics"
Assert-True ([bool]$outputRefClassified.evidence.engineering_unclean) "output-ref scenario contract miss did not mark evidence engineering_unclean"
$outputRefPairReport = Join-Path $outputRefContractPair "pair-report.md"
$outputRefReportManifest = [pscustomobject]@{
    Expected = $outputRefManifest.expected
    EvidenceTarget = "E2"
}
Write-TaskspacePairReport $outputRefPairReport $outputRefReportManifest $promptGuardOk $outputRefClassified.variable_control $outputRefClassified.evidence $outputRefClassified.left_metrics $outputRefClassified.right_metrics $outputRefClassified.pair
$outputRefPairReportText = Get-Content -Raw -Encoding UTF8 -LiteralPath $outputRefPairReport
Assert-True ($outputRefPairReportText -match "taskspace_runtime_output_ref_created_count_below_expected: 0 < 1") "output-ref scenario warning missing from pair report"

$e3ReportManifest = $manifest.PSObject.Copy()
$e3ReportManifest.EvidenceTarget = "E3"
$e3ReportManifest.SampleOrigin = $e3Origin
$e3ReportManifest.HumanReviewRequired = $true
$e3ReportManifest.E3 = [pscustomobject]@{ claim_scope = "scope"; minimum_repeats = 3 }
$e3ReportPath = Join-Path $runDir "e3-report.md"
Write-TaskspacePairReport $e3ReportPath $e3ReportManifest $promptGuardOk $varControl $e3LowRepeat $metrics $rightMetrics $pairOne
$e3ReportText = Get-Content -Raw -Encoding UTF8 -LiteralPath $e3ReportPath
Assert-True ($e3ReportText -match "e3_minimum_repeats: 5") "E3 pair report did not show effective clamped minimum repeats"
$timeoutE3 = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $e3Manifest.SampleOrigin $e3Manifest.ExternalBenchmark $e3Manifest.E3 $true $true 5 "include_taskspace_better" $false $externalProof ([pscustomobject]@{ standard_success = $false; taskspace_success = $true; exec_timeouts = @() }) @() @("public_validation_timeout")
Assert-True (-not [bool]$timeoutE3.included_in_e3_aggregate) "public validation timeout was allowed into E3 aggregate"
Assert-True (@($timeoutE3.e3_gate_failures) -contains "public_validation_timeout") "public validation timeout did not reach E3 gate failures"

$finalizeRun = New-Dir (Join-Path $runDir "finalize-preserves-env-failures")
Write-TaskspaceJson $promptGuardOk (Join-Path $finalizeRun "prompt-guard.json")
$finalizePair = New-Dir (Join-Path $finalizeRun "pair-001")
$finalizeManifest = [pscustomobject]@{
    scenario = "finalize-preserves-env-failures"
    repeat = 1
    prompt_sha256_left = "same"; prompt_sha256_right = "same"
    fixture_sha256_left = "same"; fixture_sha256_right = "same"
    whale_sha256_left = "same"; whale_sha256_right = "same"
    model_left = "same"; model_right = "same"
    timeout_seconds_left = 1; timeout_seconds_right = 1
    provider_param_status = "known"
    oracle_isolation_policy = "deferred_materialization_allowed"
    sample_origin = $e3Origin
    external_benchmark = $null
    e3 = $e3Config
    human_review_required = $true
}
Write-TaskspaceJson $finalizeManifest (Join-Path $finalizePair "manifest.resolved.json")
foreach ($side in @("left", "right")) {
    New-Dir (Join-Path $finalizePair "$side\repo") | Out-Null
    New-Dir (Join-Path $finalizePair "$side\artifacts") | Out-Null
    Write-Text (Join-Path $finalizePair "$side\artifacts\validation.stdout.log") ""
    Write-Text (Join-Path $finalizePair "$side\artifacts\validation.stderr.log") ""
    Write-Text (Join-Path $finalizePair "$side\artifacts\git-diff.patch") ""
}
$finalizeLeftMetrics = [pscustomobject]@{
    mode = "left"; logical_mode = "standard"; business_success = $false
    exec_exit_code = 0; exec_timed_out = $false; public_validation_exit_code = 124; hidden_oracle_exit_code = 0
    oracle_isolation_level = "hard_sandbox"; wall_time_ms = 1; tool_call_count = 1
    changed_paths = @(); changed_file_inventory = @(); metrics_warnings = @(); metrics_taints = @("metrics_critical_artifact_unhashed:tests/x.py")
    validator_environment_failures = @("public_validation_timeout", "docker_cleanup_container_failure")
    docker_build_result_path = ""; validator_environment_mismatch = $false
    maps = 0; nodes = 0; edges = 0; edge_order_violations = 0; spawn_agent_calls = 0; subagent_results = 0; open_leaf_nodes = 0; ordinary_before_binding = $false
}
$finalizeRightMetrics = $finalizeLeftMetrics.PSObject.Copy()
$finalizeRightMetrics.mode = "right"; $finalizeRightMetrics.logical_mode = "taskspace"; $finalizeRightMetrics.business_success = $true; $finalizeRightMetrics.public_validation_exit_code = 0
$finalizeRightMetrics.validator_environment_failures = @()
$finalizeRightMetrics.metrics_taints = @()
Write-TaskspaceJson $finalizeLeftMetrics (Join-Path $finalizePair "left\artifacts\metrics.json")
Write-TaskspaceJson $finalizeRightMetrics (Join-Path $finalizePair "right\artifacts\metrics.json")
$finalizeValidationStdout = Join-Path $finalizePair "left\artifacts\validation.stdout.log"
$finalizeValidationBefore = (Get-Item -LiteralPath $finalizeValidationStdout).LastWriteTimeUtc
$finalizeOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "finalize-taskspace-e3-run.ps1") -RunDir $finalizeRun 2>&1
Assert-True ($LASTEXITCODE -ne 0) "finalize with public validation timeout unexpectedly passed E3"
$finalizedReport = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $finalizePair "pair-report.md")
Assert-True ($finalizedReport -match "public_validation_timeout") "finalize dropped public validation timeout failure"
Assert-True ($finalizedReport -match "docker_cleanup_container_failure") "finalize dropped cleanup environment failure"
Assert-True ($finalizedReport -match "metrics_critical_artifact_unhashed:tests/x.py") "finalize dropped metrics taint failure"
Assert-True ($finalizedReport -match "included_in_e3_aggregate: False") "finalize allowed timeout pair into E3 aggregate"
$finalizeHealth = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $finalizeRun "finalize-health.json") | ConvertFrom-Json
Assert-True ([string]$finalizeHealth.rerender_mode -eq "artifact_only") "finalize did not record artifact-only rerender mode"
Assert-True (-not [bool]$finalizeHealth.validation_rerun_allowed -and -not [bool]$finalizeHealth.hidden_oracle_rerun_allowed) "finalize did not forbid validator/oracle rerun"
Assert-True (Test-Path -LiteralPath (Join-Path $finalizeRun "sample-timing.json")) "finalize did not rebuild sample timing before report outputs"
Assert-True ((Get-Item -LiteralPath $finalizeValidationStdout).LastWriteTimeUtc -eq $finalizeValidationBefore) "finalize modified validation stdout, suggesting a validator rerun"

$matrixData = Get-TaskspaceMatrixReportData @(
    [pscustomobject]@{
        scenario = "synthetic"; level = "L1"; exit_code = 0; valid_pairs = 3
        excluded_pairs = 0; non_e2_reports = 0; warning_pairs = 1; utility_warning_pairs = 0
    }
) @("L1") 3
Assert-True ($matrixData.e2_evidence_readiness) "matrix evidence readiness rejected a valid synthetic E2 row"
Assert-True (-not $matrixData.e2_clean_readiness) "matrix clean readiness ignored warning pairs"
Assert-True (-not $matrixData.e2_utility_clean_readiness) "matrix utility clean readiness ignored mechanism warning pairs"
$matrixUtility = Get-TaskspaceMatrixReportData @(
    [pscustomobject]@{
        scenario = "synthetic"; level = "L1"; exit_code = 0; valid_pairs = 3
        excluded_pairs = 0; non_e2_reports = 0; warning_pairs = 0; utility_warning_pairs = 1
    }
) @("L1") 3
Assert-True ($matrixUtility.e2_clean_readiness) "matrix mechanism clean readiness should ignore utility-only cost warnings"
Assert-True (-not $matrixUtility.e2_utility_clean_readiness) "matrix utility clean readiness ignored utility warning pairs"
Assert-True (@($matrixUtility.utility_cost_gaps).Count -eq 1) "matrix utility cost gaps did not record utility warnings"
$matrixClean = Get-TaskspaceMatrixReportData @(
    [pscustomobject]@{
        scenario = "synthetic"; level = "L1"; exit_code = 0; valid_pairs = 3
        excluded_pairs = 0; non_e2_reports = 0; warning_pairs = 0; utility_warning_pairs = 0
    }
) @("L1", "L2") 3
Assert-True (-not $matrixClean.e2_evidence_readiness) "matrix evidence readiness ignored missing required levels"
if ($failures.Count -gt 0) {
    Write-Host "TaskSpace benchmark harness self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "TaskSpace benchmark harness self-test: PASS"
Write-Host "RunRoot: $runDir"
