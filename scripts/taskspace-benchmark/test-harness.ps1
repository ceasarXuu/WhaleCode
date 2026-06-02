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
. (Join-Path $PSScriptRoot "lib\audit-report.ps1")
. (Join-Path $PSScriptRoot "lib\pair-report.ps1")
. (Join-Path $PSScriptRoot "lib\matrix-report.ps1")
. (Join-Path $PSScriptRoot "adapters\external-benchmark-common.ps1")

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
$manifestByPath = Read-TaskspaceScenarioManifest $repoRoot "" $manifest.ScenarioRoot
Assert-True ($manifestByPath.Id -eq $manifest.Id) "ScenarioPath manifest read did not preserve id"
Assert-True ($manifestByPath.ScenarioRoot -eq $manifest.ScenarioRoot) "ScenarioPath manifest read resolved a different root"
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
Assert-True (-not (Test-Path -LiteralPath $pairOne.ReviewerOracleDir)) "reviewer-only oracle directory was materialized before agent execution"
Assert-True (Test-TaskspaceNeutralCwd $pairOne.Left.RepoDir) "left cwd contains treatment label"
Assert-True (Test-TaskspaceNeutralCwd $pairOne.Right.RepoDir) "right cwd contains treatment label"
Assert-True (-not (Test-TaskspaceNeutralCwd "D:\work\taskspace-benchmark\pair-001\left\repo")) "taskspace-benchmark path was treated as neutral"
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
$emptyDiffPath = Join-Path $pairOne.Left.ArtifactDir "empty-diff.patch"
Get-TaskspaceDiffText $pairOne.Left.RepoDir $emptyDiffPath | Out-Null
Assert-True (Test-Path -LiteralPath $emptyDiffPath) "empty git diff artifact was not written"

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
$e3ExternalReady = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $externalOrigin $externalBenchmark $e3Config $true $true 5 "include_no_clear_delta" $false
Assert-True ($e3ExternalReady.reported_evidence_level -eq "E3") "complete external E3 evidence did not promote to E3"
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
$e3ExternalUnfaithful = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $externalOrigin $invalidExternalBenchmark $e3Config $true $true 5 "include_no_clear_delta" $false
Assert-True ($e3ExternalUnfaithful.reported_evidence_level -ne "E3") "unfaithful external validator was promoted to E3"
Assert-True (@($e3ExternalUnfaithful.e3_gate_failures) -contains "e3_external_validator_fidelity_unproven") "external validator fidelity gate failure was not recorded"
Assert-True (@($e3ExternalUnfaithful.e3_gate_failures) -contains "e3_external_validator_source_not_isolated") "external validator source isolation gate failure was not recorded"
$externalOriginMissingRevision = $externalOrigin.PSObject.Copy()
$externalOriginMissingRevision.source_version = ""
$e3ExternalMissingRevision = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $externalOriginMissingRevision $externalBenchmark $e3Config $true $true 5 "include_no_clear_delta" $false
Assert-True (@($e3ExternalMissingRevision.e3_gate_failures) -contains "e3_external_source_version_missing") "external E3 source revision gate failure was not recorded"
$e3Ready = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $e3Origin $null $e3Config $true $true 5 "include_taskspace_better" $false
Assert-True ($e3Ready.reported_evidence_level -eq "E3") "complete E3 evidence did not promote to E3"
Assert-True ($e3Ready.included_in_e3_aggregate) "complete E3 evidence was not included in E3 aggregate"

$auditPairDir = Join-Path $runDir "audit-pair"
New-Item -ItemType Directory -Path $auditPairDir | Out-Null
$requiredAuditArtifacts = @(
        "manifest.resolved.json",
        "pair-report.md",
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
"changed artifact" | Set-Content -LiteralPath (Join-Path $auditPairDir "pair-report.md") -Encoding UTF8
$staleHashAudit = Get-TaskspaceAuditReview $auditPairDir "" 0 "self-test audit"
Assert-True (-not $staleHashAudit.completed) "stale audit with outdated artifact hash was accepted"
Assert-True (@($staleHashAudit.failures | Where-Object { $_ -eq "audit_artifact_hash_mismatch:pair-report.md" }).Count -eq 1) "stale audit artifact hash mismatch was not reported"
"artifact" | Set-Content -LiteralPath (Join-Path $auditPairDir "pair-report.md") -Encoding UTF8
$badAuditJsonPath = Join-Path $auditPairDir "audit-review.json"
@{
    reviewer = "codex"
    date = "2026-06-02"
    artifact_basis = @("pair-report.md")
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
$adapterOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $terminalBenchNoEnv -OutputRoot (Join-Path $runDir "external-out") -SampleId "no-env" -SourceVersion "pinned"
$adapterScenarioDir = [string]($adapterOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
Assert-True (Test-Path -LiteralPath (Join-Path $adapterScenarioDir "prompt.txt")) "terminal-bench adapter did not extract task.yaml instruction"
Assert-True (-not (Test-Path -LiteralPath (Join-Path $adapterScenarioDir "fixture\solution.sh"))) "terminal-bench adapter leaked solution.sh from official task root"
$adapterScenario = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $adapterScenarioDir "scenario.json") | ConvertFrom-Json
Assert-True (-not [bool]$adapterScenario.external_benchmark.validator_fidelity.e3_eligible) "terminal-bench post-hoc Docker validator was over-promoted to E3 eligible"
Assert-True ([string]$adapterScenario.external_benchmark.validator_fidelity.validator_runtime -eq "terminal_bench_docker_app") "terminal-bench validator runtime was not Docker /app"
Assert-True (-not [bool]$adapterScenario.external_benchmark.validator_fidelity.agent_cannot_read_validator_source) "terminal-bench validator source isolation was over-claimed"
Assert-True ([bool]$adapterScenario.external_benchmark.validator_fidelity.docker_runtime) "terminal-bench Docker runtime capability was not recorded"
Assert-True ([string]$adapterScenario.external_benchmark.adapter_metadata.instruction_extraction_mode -eq "literal") "terminal-bench literal instruction mode was not recorded"
$terminalBenchInline = Join-Path $runDir "terminal-bench-inline"
New-Item -ItemType Directory -Path $terminalBenchInline | Out-Null
@'
instruction: "Fix the inline instruction case."
category: software-engineering
'@ | Set-Content -LiteralPath (Join-Path $terminalBenchInline "task.yaml") -Encoding UTF8
"FROM scratch" | Set-Content -LiteralPath (Join-Path $terminalBenchInline "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $terminalBenchInline "run-tests.sh") -Encoding UTF8
$inlineOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $terminalBenchInline -OutputRoot (Join-Path $runDir "external-inline-out") -SampleId "inline" -SourceVersion "pinned"
$inlineScenarioDir = [string]($inlineOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$inlinePrompt = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $inlineScenarioDir "prompt.txt")
Assert-True ($inlinePrompt -match "Fix the inline instruction case") "terminal-bench adapter did not extract inline task.yaml instruction"
$terminalBenchFolded = Join-Path $runDir "terminal-bench-folded"
New-Item -ItemType Directory -Path $terminalBenchFolded | Out-Null
@'
instruction: >
  Read one line
  then write the file.
category: file-operations
'@ | Set-Content -LiteralPath (Join-Path $terminalBenchFolded "task.yaml") -Encoding UTF8
"FROM scratch" | Set-Content -LiteralPath (Join-Path $terminalBenchFolded "Dockerfile") -Encoding UTF8
"echo ok" | Set-Content -LiteralPath (Join-Path $terminalBenchFolded "run-tests.sh") -Encoding UTF8
$foldedOutput = & (Join-Path $PSScriptRoot "adapters\terminal-bench-adapter.ps1") -TaskDir $terminalBenchFolded -OutputRoot (Join-Path $runDir "external-folded-out") -SampleId "folded" -SourceVersion "pinned"
$foldedScenarioDir = [string]($foldedOutput | Select-Object -Last 1 | ForEach-Object { $_.scenario_dir })
$foldedPrompt = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $foldedScenarioDir "prompt.txt")
Assert-True ($foldedPrompt -match "Read one line then write the file") "terminal-bench adapter did not fold task.yaml instruction"
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
$externalWrapperStub = Join-Path $runDir "external-wrapper-stub.ps1"
@'
param(
    [string]$ScenarioPath,
    [int]$Repeats,
    [string]$WhaleBin,
    [string]$Model,
    [string]$RunRoot,
    [switch]$AllowNonE2Result,
    [switch]$PlanOnly
)
if ($PlanOnly) { exit 0 }
if ($AllowNonE2Result) { Write-Host "stub diagnostic allowed"; exit 0 }
Write-Host "stub target unsatisfied"
exit 1
'@ | Set-Content -LiteralPath $externalWrapperStub -Encoding UTF8
$externalWrapperRunRoot = Join-Path $runDir "external-wrapper-runs"
$wrapperDefaultOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1") -Benchmark terminal-bench -TaskDir $terminalBenchNoEnv -SourceVersion "pinned" -RunRoot $externalWrapperRunRoot -RunnerPath $externalWrapperStub 2>&1
$wrapperDefaultExit = $LASTEXITCODE
Assert-True ($wrapperDefaultExit -ne 0) "external benchmark wrapper hid unsatisfied E3 target by default"
$wrapperDiagnosticOutput = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "run-taskspace-external-benchmark.ps1") -Benchmark terminal-bench -TaskDir $terminalBenchNoEnv -SourceVersion "pinned" -RunRoot $externalWrapperRunRoot -RunnerPath $externalWrapperStub -AllowDiagnosticNonTargetResult 2>&1
$wrapperDiagnosticExit = $LASTEXITCODE
Assert-True ($wrapperDiagnosticExit -eq 0) "external benchmark wrapper did not allow explicit diagnostic non-target result"
Assert-True (($wrapperDiagnosticOutput -join "`n") -match "DiagnosticNonTargetResultAllowed: True") "external benchmark wrapper did not print diagnostic opt-in marker"

$metrics = [pscustomobject]@{
    mode = "left"; logical_mode = "standard"; business_success = $true; exec_exit_code = 0
    public_validation_exit_code = 0; hidden_oracle_exit_code = 0; oracle_isolation_level = "hard_sandbox"
    wall_time_ms = 1; tool_call_count = 1; changed_paths = @("src/tax_calc.py")
    changed_file_inventory = @([pscustomobject]@{ path = "src/tax_calc.py"; status = "M "; source = "git_status"; sha256 = "abc123"; size_bytes = 12 })
    validator_environment_mismatch = $false
    maps = 0; nodes = 0; edges = 0; edge_order_violations = 0; spawn_agent_calls = 0
    subagent_results = 0; open_leaf_nodes = 0; ordinary_before_binding = $false
}
$rightMetrics = $metrics.PSObject.Copy()
$rightMetrics.mode = "right"; $rightMetrics.logical_mode = "taskspace"
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
Write-TaskspacePairReport $reportPath $manifest $manualGuard $varControl $manualEvidence $metrics $rightMetrics $pairOne
$reportText = Get-Content -Raw -Encoding UTF8 -LiteralPath $reportPath
Assert-True ($reportText -match "manual_review_required: True") "manual review requirement was not persisted in pair report"
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
$aggregatePath = Join-Path $runDir "aggregate.md"
Write-TaskspaceAggregateReport -Path $aggregatePath -Reports @(
    [pscustomobject]@{ repeat = 1; pair_report = "one.md"; evidence = [pscustomobject]@{ reported_evidence_level = "E2"; included_in_utility_aggregate = $true; evidence_gate_failures = @() } },
    [pscustomobject]@{ repeat = 2; pair_report = "two.md"; evidence = [pscustomobject]@{ reported_evidence_level = "E1"; included_in_utility_aggregate = $false; evidence_gate_failures = @("oracle_isolation_failed") } }
)
$aggregateText = Get-Content -Raw -Encoding UTF8 -LiteralPath $aggregatePath
Assert-True ($aggregateText -match "valid_utility_pairs: 1") "aggregate did not count only E2 utility pairs"
Assert-True ($aggregateText -match "excluded_pairs: 1") "aggregate did not exclude non-E2 pair"
Assert-True ($aggregateText -notmatch "valid_e3_pairs") "E2 aggregate emitted E3 aggregate noise"
$e3AggregatePath = Join-Path $runDir "aggregate-e3.md"
Write-TaskspaceAggregateReport -Path $e3AggregatePath -Reports @(
    [pscustomobject]@{ repeat = 1; pair_report = "e3-one.md"; evidence = $e3Ready },
    [pscustomobject]@{ repeat = 2; pair_report = "e3-two.md"; evidence = $e3Candidate }
)
$e3AggregateText = Get-Content -Raw -Encoding UTF8 -LiteralPath $e3AggregatePath
Assert-True ($e3AggregateText -match "valid_e3_pairs: 1") "E3 aggregate did not count complete E3 pairs"
Assert-True ($e3AggregateText -match "e3_human_review_not_completed") "E3 aggregate did not preserve E3 gate failures"
Assert-True ($e3AggregateText -match "e3_human_review_completed_pairs: 1") "E3 aggregate did not count human review completion"
Assert-True ($e3AggregateText -match "include_taskspace_better=1") "E3 aggregate did not summarize human review decisions"
Assert-True ($e3AggregateText -match "e3_taskspace_better_pairs: 1") "E3 aggregate did not count directional TaskSpace benefit"
Assert-True ($e3AggregateText -match "e3_standard_better_pairs: 0") "E3 aggregate did not separate standard-better pairs"
Assert-True ($e3AggregateText -match "only include_taskspace_better counts") "E3 aggregate did not include directional benefit warning"
$e3ReportManifest = $manifest.PSObject.Copy()
$e3ReportManifest.EvidenceTarget = "E3"
$e3ReportManifest.SampleOrigin = $e3Origin
$e3ReportManifest.HumanReviewRequired = $true
$e3ReportManifest.E3 = [pscustomobject]@{ claim_scope = "scope"; minimum_repeats = 3 }
$e3ReportPath = Join-Path $runDir "e3-report.md"
Write-TaskspacePairReport $e3ReportPath $e3ReportManifest $promptGuardOk $varControl $e3LowRepeat $metrics $rightMetrics $pairOne
$e3ReportText = Get-Content -Raw -Encoding UTF8 -LiteralPath $e3ReportPath
Assert-True ($e3ReportText -match "e3_minimum_repeats: 5") "E3 pair report did not show effective clamped minimum repeats"
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
