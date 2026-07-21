param([string]$SourceCommit = "HEAD")

$ErrorActionPreference = "Stop"
$sourceRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$sourceCommitId = (& git -C $sourceRoot rev-parse $SourceCommit).Trim()
$runId = [DateTime]::UtcNow.ToString("yyyyMMddHHmmssfff")
$clone = Join-Path $sourceRoot "target/r7-toolchain/integration-$runId"

function Invoke-Git {
    param([string]$Repo, [string[]]$Arguments)
    $output = & git -C $Repo @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) { throw "R7_INTEGRATION_GIT_FAILED args=$($Arguments -join ' ') detail=$($output -join [Environment]::NewLine)" }
    @($output)
}

function Get-GitLine {
    param([string]$Repo, [string[]]$Arguments)
    $lines = @(Invoke-Git $Repo $Arguments)
    if ($lines.Count -ne 1) { throw "R7_INTEGRATION_GIT_LINE_COUNT count=$($lines.Count)" }
    ([string]$lines[0]).Trim()
}

function Get-Hash {
    param([string]$Path)
    [System.BitConverter]::ToString([System.Security.Cryptography.SHA256]::Create().ComputeHash([System.IO.File]::ReadAllBytes($Path))).Replace("-", "").ToLowerInvariant()
}

function Write-Json {
    param([string]$Path, $Value)
    [System.IO.Directory]::CreateDirectory((Split-Path $Path -Parent)) | Out-Null
    $json = $Value | ConvertTo-Json -Depth 100
    [System.IO.File]::WriteAllText($Path, $json + [Environment]::NewLine, [System.Text.UTF8Encoding]::new($false))
}

function Invoke-Script {
    param([string]$Repo, [string]$Name, [string[]]$Arguments = @())
    $output = & pwsh -NoLogo -NoProfile -File (Join-Path $Repo "scripts/taskspace-benchmark/$Name") @Arguments
    if ($LASTEXITCODE -ne 0) { throw "R7_INTEGRATION_SCRIPT_FAILED script=$Name" }
    @($output)
}

function Invoke-PinnedCompletion {
    param([string]$Repo, [string]$Launcher, [string]$Target, [string]$AnchorCommit, [string]$Attempt, [string]$Attestation)
    & pwsh -NoLogo -NoProfile -File $Launcher -RepoRoot $Repo -TargetCommit $Target -ToolchainAddCommit $AnchorCommit -RequiredCheckRunId "70001" -RequiredCheckRunAttempt $Attempt -RequiredCheckName "r7-continuous-action-completion" -Repository "ceasarXuu/WhaleCode" -WorkflowRef "ceasarXuu/WhaleCode/.github/workflows/r7-continuous-action-completion.yml@refs/heads/whalecode-alpha" -WorkflowSha $Target -EventName "push" -GitSha $Target -GitRef "refs/heads/whalecode-alpha" -ExecutionImage $env:R7_EXECUTION_IMAGE -PowerShellArchivePath $env:R7_POWERSHELL_ARCHIVE_PATH -AttestationPath $Attestation
    if ($LASTEXITCODE -ne 0) { throw "R7_INTEGRATION_COMPLETION_FAILED attempt=$Attempt" }
}

& git clone --quiet --no-hardlinks $sourceRoot $clone
if ($LASTEXITCODE -ne 0) { throw "R7_INTEGRATION_CLONE_FAILED" }
[void](Invoke-Git $clone @("checkout", "--quiet", "-b", "r7-integration-$runId", $sourceCommitId))
[void](Invoke-Git $clone @("config", "user.name", "R7 Integration"))
[void](Invoke-Git $clone @("config", "user.email", "r7-integration@invalid.local"))
[void](Invoke-Script $clone "test-r7-continuous-action-toolchain.ps1" @("-Mode", "PreAnchor"))

$roles = [ordered]@{
    anchor_schema = "benchmarks/taskspace/r7/immutable-anchor-v1.schema.json"
    artifact_fixtures = "scripts/taskspace-benchmark/r7-v2-artifact-fixtures.ps1"
    artifact_schema = "benchmarks/taskspace/r7/candidate-artifact-content-v2.schema.json"
    candidate_generator = "scripts/taskspace-benchmark/new-r7-continuous-action-candidate.ps1"
    candidate_manifest_schema = "benchmarks/taskspace/r7/taskspace-candidate-manifest-v2.schema.json"
    candidate_set_verifier = "scripts/taskspace-benchmark/test-r7-continuous-action-candidate-set.ps1"
    candidate_verifier = "scripts/taskspace-benchmark/test-r7-continuous-action-candidate.ps1"
    carrier_validation_fixture = "benchmarks/taskspace/r7/fixtures/carrier-validation-dev-v1/scenario.json"
    closure_generator_entry = "third_party/codex-cli/codex-rs/tools/src/bin/r7_carrier_entry_closure/entry.rs"
    closure_generator_main = "third_party/codex-cli/codex-rs/tools/src/bin/r7_carrier_entry_closure.rs"
    closure_generator_profiles = "third_party/codex-cli/codex-rs/tools/src/bin/r7_carrier_entry_closure/profiles.rs"
    closure_generator_provider = "third_party/codex-cli/codex-rs/tools/src/bin/r7_carrier_entry_closure/provider.rs"
    closure_generator_sources = "third_party/codex-cli/codex-rs/tools/src/bin/r7_carrier_entry_closure/sources.rs"
    closure_generator_sources_test = "third_party/codex-cli/codex-rs/tools/src/bin/r7_carrier_entry_closure/sources_tests.rs"
    completion_evidence_schema = "benchmarks/taskspace/r7/continuous-action-completion-evidence-v1.schema.json"
    completion_launcher = "scripts/taskspace-benchmark/invoke-r7-continuous-action-completion.ps1"
    completion_verifier = "scripts/taskspace-benchmark/verify-r7-continuous-action-completion.ps1"
    evaluation_contract = "benchmarks/taskspace/r7/continuous-action-evaluation-v1.json"
    evaluation_launcher = "scripts/taskspace-benchmark/evaluate-r7-continuous-action-runset.ps1"
    evaluation_library = "scripts/taskspace-benchmark/lib/r7-continuous-action-evaluator.ps1"
    evaluation_result_schema = "benchmarks/taskspace/r7/continuous-action-evaluation-result-v1.schema.json"
    evaluation_schema = "benchmarks/taskspace/r7/continuous-action-evaluation-v1.schema.json"
    evaluation_test = "scripts/taskspace-benchmark/test-r7-continuous-action-evaluator.ps1"
    execution_environment = "benchmarks/taskspace/r7/ca0-execution-environment-v1.json"
    execution_environment_schema = "benchmarks/taskspace/r7/ca0-execution-environment-v1.schema.json"
    integration_test = "scripts/taskspace-benchmark/test-r7-continuous-action-integration.ps1"
    phase_ownership = "benchmarks/taskspace/r7/r7-phase-ownership-v1.json"
    projection_ownership_inventory = "benchmarks/taskspace/r7/phase-a-ownership-inventory.json"
    raw_run_set_schema = "benchmarks/taskspace/r7/continuous-action-raw-run-set-v1.schema.json"
    required_check_workflow = ".github/workflows/r7-continuous-action-completion.yml"
    strict_json_library = "scripts/taskspace-benchmark/lib/r7-strict-json.ps1"
    strict_parser = "scripts/taskspace-benchmark/invoke-r7-strict-json.ps1"
    toolchain_core = "scripts/taskspace-benchmark/r7-v2-toolchain-core.ps1"
    toolchain_history = "scripts/taskspace-benchmark/r7-v2-history.ps1"
    toolchain_promotion = "scripts/taskspace-benchmark/r7-v2-promotion.ps1"
    toolchain_test = "scripts/taskspace-benchmark/test-r7-continuous-action-toolchain.ps1"
    toolchain_transaction = "scripts/taskspace-benchmark/r7-v2-git-transaction.ps1"
    toolchain_transaction_test = "scripts/taskspace-benchmark/test-r7-git-transaction.ps1"
    transition_command = "scripts/taskspace-benchmark/set-r7-continuous-action-candidate-status.ps1"
    tools_cargo_lock = "third_party/codex-cli/codex-rs/Cargo.lock"
    tools_cargo_manifest = "third_party/codex-cli/codex-rs/tools/Cargo.toml"
}
$anchorParent = Get-GitLine $clone @("rev-parse", "HEAD")
$artifacts = foreach ($entry in $roles.GetEnumerator()) {
    [pscustomobject][ordered]@{role = $entry.Key; path = $entry.Value; sha256 = Get-Hash (Join-Path $clone $entry.Value); git_mode = "100644"}
}
$anchorPath = Join-Path $clone "benchmarks/taskspace/r7/continuous-action-v2-toolchain-anchor-v1.json"
Write-Json $anchorPath ([pscustomobject][ordered]@{schema_version = 1; anchor_kind = "continuous_action_v2_toolchain"; anchored_parent_commit = $anchorParent; artifacts = @($artifacts)})
[void](Invoke-Git $clone @("add", "--", "benchmarks/taskspace/r7/continuous-action-v2-toolchain-anchor-v1.json"))
[void](Invoke-Git $clone @("commit", "--quiet", "-m", "test(r7): add synthetic toolchain anchor"))
$anchorCommit = Get-GitLine $clone @("rev-parse", "HEAD")
[void](Invoke-Script $clone "test-r7-continuous-action-toolchain.ps1" @("-Mode", "Anchored"))

$candidateOutput = @(Invoke-Script $clone "new-r7-continuous-action-candidate.ps1" @("-ArtifactSourceDirectory", (Join-Path $clone "target/r7-toolchain/self-test")))
$candidateJson = ([string]$candidateOutput[-1]) | ConvertFrom-Json
$candidateId = [string]$candidateJson.candidate_id
$candidateManifestPath = "benchmarks/taskspace/r7/candidates/$candidateId/manifest.json"
$candidate = ((Invoke-Git $clone @("show", "HEAD:$candidateManifestPath")) -join [Environment]::NewLine) | ConvertFrom-Json -Depth 100
[void](Invoke-Script $clone "test-r7-continuous-action-evaluator.ps1")
$evalDir = Get-ChildItem -LiteralPath (Join-Path $clone "target/r7-continuous-action-evaluator-test") -Directory | Sort-Object Name | Select-Object -Last 1
$runSet = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $evalDir.FullName "runset.json") | ConvertFrom-Json -Depth 100
$baseline = ((Invoke-Git $clone @("show", "HEAD:benchmarks/taskspace/r7/continuous-action-ca0-baseline-v3.json")) -join [Environment]::NewLine) | ConvertFrom-Json
$runSet.identity.standard_commit = [string]$candidate.candidate_commit
$runSet.identity.candidate_commit = [string]$candidate.candidate_commit
$runSet.identity.sibling_baseline_commit = [string]$baseline.anchored_parent_commit
$runSet.evaluation_contract.path = [string]$candidate.artifact_hashes.continuous_action_evaluation.path
$runSet.evaluation_contract.sha256 = [string]$candidate.artifact_hashes.continuous_action_evaluation.sha256

$evidenceRelative = "benchmarks/taskspace/r7/evidence/$candidateId"
$evidenceRoot = Join-Path $clone $evidenceRelative
[System.IO.Directory]::CreateDirectory((Join-Path $evidenceRoot "artifacts")) | Out-Null
foreach ($run in @($runSet.runs)) {
    foreach ($property in @($run.artifacts.psobject.Properties)) {
        $fileName = "$($run.run_id)-$($property.Name).json"
        $destinationRelative = "$evidenceRelative/artifacts/$fileName"
        [System.IO.File]::WriteAllBytes((Join-Path $clone $destinationRelative), [System.IO.File]::ReadAllBytes((Join-Path $evalDir.FullName ([string]$property.Value.path))))
        $property.Value.path = $destinationRelative
        $property.Value.sha256 = Get-Hash (Join-Path $clone $destinationRelative)
    }
}
$runSetPath = Join-Path $evidenceRoot "raw-run-set.json"
Write-Json $runSetPath $runSet
$resultPath = Join-Path $evidenceRoot "evaluation-result.json"
[void](Invoke-Script $clone "evaluate-r7-continuous-action-runset.ps1" @("-RunSetPath", $runSetPath, "-EvaluationContractPath", (Join-Path $clone ([string]$runSet.evaluation_contract.path)), "-RunArtifactRoot", $clone, "-OutputPath", $resultPath))
$completionPath = Join-Path $evidenceRoot "completion-evidence.json"
$completion = [pscustomobject][ordered]@{
    schema_version = 2; artifact_role = "continuous_action_completion_evidence"; candidate_id = $candidateId
    candidate_commit = [string]$candidate.candidate_commit
    evaluation_contract = [pscustomobject]@{path = $runSet.evaluation_contract.path; sha256 = $runSet.evaluation_contract.sha256; git_mode = "100644"}
    raw_run_set = [pscustomobject]@{path = "$evidenceRelative/raw-run-set.json"; sha256 = Get-Hash $runSetPath; git_mode = "100644"}
    evaluation_result = [pscustomobject]@{path = "$evidenceRelative/evaluation-result.json"; sha256 = Get-Hash $resultPath; git_mode = "100644"}
    completed_at_utc = [DateTime]::UtcNow.ToString("o")
}
Write-Json $completionPath $completion
[void](Invoke-Git $clone @("add", "--", $evidenceRelative))
[void](Invoke-Git $clone @("commit", "--quiet", "-m", "test(r7): add integration completion evidence"))
$evidenceCommit = Get-GitLine $clone @("rev-parse", "HEAD")
[void](Invoke-Script $clone "set-r7-continuous-action-candidate-status.ps1" @("-CandidateId", $candidateId, "-ToStatus", "promotion_pending", "-EvidencePath", "$evidenceRelative/completion-evidence.json", "-ExpectedHead", $evidenceCommit))
$pendingCommit = Get-GitLine $clone @("rev-parse", "HEAD")
[void](Invoke-Script $clone "set-r7-continuous-action-candidate-status.ps1" @("-CandidateId", $candidateId, "-ToStatus", "promoted", "-EvidencePath", "$evidenceRelative/completion-evidence.json", "-ExpectedHead", $pendingCommit))
$promotedCommit = Get-GitLine $clone @("rev-parse", "HEAD")

$launcher = Join-Path $clone "target/r7-toolchain/pinned-launcher.ps1"
$launcherText = ((Invoke-Git $clone @("show", "$($anchorParent):$($roles.completion_launcher)")) -join [Environment]::NewLine) + [Environment]::NewLine
[System.IO.File]::WriteAllText($launcher, $launcherText, [System.Text.UTF8Encoding]::new($false))
$promotionAttestation = Join-Path $clone "target/r7-toolchain/promotion-attestation.json"
Invoke-PinnedCompletion $clone $launcher $promotedCommit $anchorCommit "1" $promotionAttestation
[void](Invoke-Script $clone "set-r7-continuous-action-candidate-status.ps1" @("-CandidateId", $candidateId, "-ToStatus", "reverted", "-EvidencePath", "$evidenceRelative/completion-evidence.json", "-ExpectedHead", $promotedCommit))
$revertedCommit = Get-GitLine $clone @("rev-parse", "HEAD")
$revertAttestation = Join-Path $clone "target/r7-toolchain/revert-attestation.json"
Invoke-PinnedCompletion $clone $launcher $revertedCommit $anchorCommit "2" $revertAttestation

$successorSource = Join-Path $clone "target/r7-toolchain/successor-source"
[System.IO.Directory]::CreateDirectory($successorSource) | Out-Null
Copy-Item -Path (Join-Path $clone "target/r7-toolchain/self-test/*") -Destination $successorSource -Recurse
$successorTransitionPath = Join-Path $successorSource "transition-schema.json"
$successorTransition = Get-Content -Raw -Encoding UTF8 -LiteralPath $successorTransitionPath | ConvertFrom-Json -Depth 100
$successorTransition.positive_fixtures[0].id = "$($successorTransition.positive_fixtures[0].id)-successor"
Write-Json $successorTransitionPath $successorTransition
$successorOutput = @(Invoke-Script $clone "new-r7-continuous-action-candidate.ps1" @("-ArtifactSourceDirectory", $successorSource))
$successorJson = ([string]$successorOutput[-1]) | ConvertFrom-Json
$successorId = [string]$successorJson.candidate_id
if ($successorId -ceq $candidateId) { throw "R7_INTEGRATION_SUCCESSOR_ID_REUSED" }
$successorEvidenceRelative = "benchmarks/taskspace/r7/evidence/$successorId/lifecycle-evidence.json"
$successorEvidencePath = Join-Path $clone $successorEvidenceRelative
Write-Json $successorEvidencePath ([pscustomobject][ordered]@{schema_version = 1; candidate_id = $successorId; test_evidence = "successor_lifecycle"})
[void](Invoke-Git $clone @("add", "--", $successorEvidenceRelative))
[void](Invoke-Git $clone @("commit", "--quiet", "-m", "test(r7): add successor lifecycle evidence"))
$successorEvidenceCommit = Get-GitLine $clone @("rev-parse", "HEAD")
[void](Invoke-Script $clone "set-r7-continuous-action-candidate-status.ps1" @("-CandidateId", $successorId, "-ToStatus", "promotion_pending", "-EvidencePath", $successorEvidenceRelative, "-ExpectedHead", $successorEvidenceCommit))
$successorPendingCommit = Get-GitLine $clone @("rev-parse", "HEAD")
$predecessor = ((Invoke-Git $clone @("show", "$successorPendingCommit`:$candidateManifestPath")) -join [Environment]::NewLine) | ConvertFrom-Json -Depth 100
if ([string]$predecessor.superseded_by.candidate_id -cne $successorId) { throw "R7_INTEGRATION_SUCCESSOR_BINDING_MISSING" }
[void](Invoke-Script $clone "set-r7-continuous-action-candidate-status.ps1" @("-CandidateId", $successorId, "-ToStatus", "rejected", "-EvidencePath", $successorEvidenceRelative, "-ExpectedHead", $successorPendingCommit))
$successorRejectedCommit = Get-GitLine $clone @("rev-parse", "HEAD")
[void](Invoke-Script $clone "test-r7-continuous-action-candidate-set.ps1" @("-TargetCommit", $successorRejectedCommit))

$result = [pscustomobject][ordered]@{
    schema_version = 1; test = "r7_continuous_action_integration"; passed = $true; clone = $clone
    source_commit = $sourceCommitId; anchor_commit = $anchorCommit; candidate_id = $candidateId
    promoted_commit = $promotedCommit; reverted_commit = $revertedCommit
    successor_candidate_id = $successorId; successor_pending_commit = $successorPendingCommit; successor_rejected_commit = $successorRejectedCommit
    promotion_attestation_sha256 = Get-Hash $promotionAttestation; revert_attestation_sha256 = Get-Hash $revertAttestation
}
Write-Json (Join-Path $clone "target/r7-toolchain/integration-result.json") $result
Write-Output ($result | ConvertTo-Json -Compress)
