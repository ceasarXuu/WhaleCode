param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
. (Join-Path $repoRoot "scripts\action-map-real-user-e2e-lib.ps1")
. (Join-Path $PSScriptRoot "lib\scenario-manifest.ps1")
. (Join-Path $PSScriptRoot "lib\audit-report.ps1")
. (Join-Path $PSScriptRoot "lib\e3-proof.ps1")
. (Join-Path $PSScriptRoot "lib\pair-report.ps1")
. (Join-Path $PSScriptRoot "lib\source-guard.ps1")

if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\e3-proof-selftest" }
$runDir = New-Dir (Join-Path $RunRoot (Get-Date -Format "yyyyMMdd-HHmmss-fff"))
$failures = New-Object System.Collections.Generic.List[string]

function Assert-True([bool]$Condition, [string]$Message) {
    if (-not $Condition) { $script:failures.Add($Message) }
}

function New-TestFile([string]$Path, [string]$Text = "artifact") {
    New-Item -ItemType Directory -Path (Split-Path -Parent $Path) -Force | Out-Null
    Set-Content -LiteralPath $Path -Encoding UTF8 -Value $Text
}

function New-ProofLog([string]$Path, [bool]$WithWrapper = $true, [bool]$WithInspect = $true) {
    $artifactDir = Split-Path -Parent $Path
    $runtimeDir = New-Dir (Join-Path $artifactDir "external-validator-runtime")
    $wrapperPath = Join-Path $runtimeDir "validator.ps1"
    $entryPath = Join-Path $runtimeDir "entry.sh"
    New-TestFile $wrapperPath "wrapper"
    New-TestFile $entryPath "entry"
    $wrapperSha = (Get-FileHash -Algorithm SHA256 -LiteralPath $wrapperPath).Hash.ToLowerInvariant()
    $entrySha = (Get-FileHash -Algorithm SHA256 -LiteralPath $entryPath).Hash.ToLowerInvariant()
    $script:TestWrapperSha = $wrapperSha
    $runtimeManifestPath = Join-Path $runtimeDir "terminal-bench-runtime-manifest.json"
    $inspectPath = Join-Path $runtimeDir "terminal-bench-docker-inspect.json"
    $cleanupResultPath = Join-Path $runtimeDir "validation-cleanup-result.json"
    @{
        proof_nonce = "0123456789abcdef0123456789abcdef"
        wrapper_path = $wrapperPath
        wrapper_sha256 = $wrapperSha
        entry_script_path = $entryPath
        entry_sha256 = $entrySha
        validator_command = "bash /tests/run-tests.sh"
        uv_cache_mount = "/tmp/uv-cache"
        uv_installer_sha256 = ("a" * 64)
        uv_archive_sha256 = ("b" * 64)
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $runtimeManifestPath -Encoding UTF8
    if ($WithInspect) {
        $uvCacheSource = if ($script:ForceBadUvCacheSource) { "/tmp/other-cache" } else { "/tmp/uv-cache" }
        @(
            @{
                Mounts = @(
                    @{ Destination = "/tests"; RW = $false },
                    @{ Destination = "/app"; RW = $true },
                    @{ Destination = "/tbench-entry.sh"; RW = $false },
                    @{ Destination = "/tbench-uv-cache"; RW = $false; Source = $uvCacheSource }
                )
                Config = @{ WorkingDir = "/app" }
            }
        ) | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $inspectPath -Encoding UTF8
    }
    @{ classification = "ok"; identity_matched = $true } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $cleanupResultPath -Encoding UTF8
    $lines = @(
        "validator_proof_nonce=0123456789abcdef0123456789abcdef",
        "validator_wrapper_sha256=$wrapperSha",
        "validator_entry_sha256=$entrySha",
        "validator_runtime_manifest_path=$runtimeManifestPath",
        "docker_inspect_path=$inspectPath",
        "validation_cleanup_result_path=$cleanupResultPath",
        "validator_runtime=terminal_bench_equivalent_docker_app",
        "container_workdir=/app",
        "docker_inspect_available=True",
        "test_dir=/tests",
        "validator_mount=/tests",
        "validator_command=bash /tests/run-tests.sh",
        "validator_mount_readonly=true"
    )
    if ($WithWrapper) { $lines = @("validator_runtime_probe=terminal_bench_equivalent_wrapper") + $lines }
    New-TestFile $Path ($lines -join "`n")
}

function New-Metrics([string]$ArtifactDir, [bool]$WithWrapper = $true, [bool]$BusinessSuccess = $true) {
    $validationOut = Join-Path $ArtifactDir "validation.stdout.log"
    New-ProofLog $validationOut $WithWrapper
    New-TestFile (Join-Path $ArtifactDir "validation.stderr.log") ""
    New-TestFile (Join-Path $ArtifactDir "whale-exec.jsonl") ""
    New-TestFile (Join-Path $ArtifactDir "whale-exec.stderr.log") ""
    New-TestFile (Join-Path $ArtifactDir "last-message.md") ""
    [pscustomobject]@{
        logical_mode = "standard"
        public_validation_exit_code = $(if ($BusinessSuccess) { 0 } else { 1 })
        validation_stdout_path = $validationOut
        validation_stderr_path = Join-Path $ArtifactDir "validation.stderr.log"
        jsonl_path = Join-Path $ArtifactDir "whale-exec.jsonl"
        stderr_path = Join-Path $ArtifactDir "whale-exec.stderr.log"
        last_message_path = Join-Path $ArtifactDir "last-message.md"
        business_success = $BusinessSuccess
    }
}

function New-SkippedTimeoutMetrics([string]$ArtifactDir) {
    New-TestFile (Join-Path $ArtifactDir "validation.stdout.log") "public_validation_skipped=true`npublic_validation_skip_reason=agent_exec_timeout`n"
    New-TestFile (Join-Path $ArtifactDir "validation.stderr.log") ""
    New-TestFile (Join-Path $ArtifactDir "whale-exec.jsonl") ""
    New-TestFile (Join-Path $ArtifactDir "whale-exec.stderr.log") "Process timed out"
    New-TestFile (Join-Path $ArtifactDir "last-message.md") ""
    [pscustomobject]@{
        logical_mode = "taskspace"
        exec_timed_out = $true
        public_validation_skipped = $true
        public_validation_skip_reason = "agent_exec_timeout"
        pre_agent_validator_probe_status = "passed"
        pre_agent_validator_probe_hash = ("c" * 64)
        public_validation_exit_code = 0
        validation_stdout_path = Join-Path $ArtifactDir "validation.stdout.log"
        validation_stderr_path = Join-Path $ArtifactDir "validation.stderr.log"
        jsonl_path = Join-Path $ArtifactDir "whale-exec.jsonl"
        stderr_path = Join-Path $ArtifactDir "whale-exec.stderr.log"
        last_message_path = Join-Path $ArtifactDir "last-message.md"
        business_success = $false
    }
}

$scenarioRoot = New-Dir (Join-Path $runDir "scenario")
$validatorSource = New-Dir (Join-Path $scenarioRoot "external-validator-source")
$validatorFile = Join-Path $validatorSource "run-tests.sh"
New-TestFile $validatorFile "echo ok"
$validatorSha = Get-TaskspaceDirectorySha256 $validatorSource
$officialRoot = New-Dir (Join-Path $runDir "official-source")
$officialSource = Join-Path $officialRoot "harness.py"
New-TestFile $officialSource "official protocol source"
Push-Location $officialRoot
try {
    git init | Out-Null
    git config user.email "taskspace-test@example.local" | Out-Null
    git config user.name "TaskSpace Test" | Out-Null
    git add . | Out-Null
    git commit -m "official source" | Out-Null
    $officialRevision = (git rev-parse HEAD).Trim()
    $officialBlob = (git rev-parse "$officialRevision`:harness.py").Trim()
    $currentBlob = (git hash-object $officialSource).Trim()
} finally {
    Pop-Location
}
$officialHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $officialSource).Hash.ToLowerInvariant()
$sourceGuardPath = Join-Path $runDir "pair\external-source-guard-proof.json"
$sourceGuard = [pscustomobject]@{
    active = $true
    proof_path = $sourceGuardPath
    identity = "test"
    process_id = $PID
    protected_file_count = 1
    all_reads_denied_after_protect = $true
    all_denies_removed_after_release = $true
    all_reads_restored_after_release = $true
    required_probe_kinds = @("current_powershell", "powershell_child", "cmd_child")
    files = @([pscustomobject]@{
        path = $validatorFile
        file_sha256_before_protect = (Get-FileHash -Algorithm SHA256 -LiteralPath $validatorFile).Hash.ToLowerInvariant()
        deny_exit_code = 0
        probes_after_protect = @(
            [pscustomobject]@{ kind = "current_powershell"; available = $true; read_denied = $true },
            [pscustomobject]@{ kind = "powershell_child"; available = $true; read_denied = $true },
            [pscustomobject]@{ kind = "cmd_child"; available = $true; read_denied = $true }
        )
    })
    release_files = @([pscustomobject]@{
        path = $validatorFile
        file_sha256_after_release = (Get-FileHash -Algorithm SHA256 -LiteralPath $validatorFile).Hash.ToLowerInvariant()
        remove_exit_code = 0
    })
}
$manifest = [pscustomobject]@{
    ScenarioRoot = $scenarioRoot
    SampleOrigin = [pscustomobject]@{ original_validator_sha256 = $validatorSha; generated_wrapper_sha256 = "" }
    ExternalBenchmark = [pscustomobject]@{
        name = "terminal-bench"
        adapter_version = "test"
        validator_source_dir = "external-validator-source"
        adapter_metadata = [pscustomobject]@{
            official_equivalence = [pscustomobject]@{
                protocol = "terminal_bench_post_agent_tests_v1"
                source_root = $officialRoot
                source_revision = $officialRevision
                source_revision_pinned = $true
                source_files_match_pinned_revision = $true
                task_worktree_dirty = $false
                source_files = @([pscustomobject]@{
                    path = $officialSource
                    relative_path = "harness.py"
                    current_sha256 = $officialHash
                    pinned_blob_id = $officialBlob
                    current_blob_id = $currentBlob
                    matches_pinned_revision = $true
                })
            }
        }
        validator_fidelity = [pscustomobject]@{
            official_runner_or_equivalent = $true
            agent_cannot_read_validator_source = $false
            e3_eligible = $true
        }
    }
}
$pairDir = New-Dir (Join-Path $runDir "pair")
New-TestFile $sourceGuardPath ($sourceGuard | ConvertTo-Json -Depth 5)
$pair = [pscustomobject]@{
    PairDir = $pairDir
    left = [pscustomobject]@{ RepoDir = New-Dir (Join-Path $pairDir "left\repo"); ArtifactDir = New-Dir (Join-Path $pairDir "left\artifacts") }
    right = [pscustomobject]@{ RepoDir = New-Dir (Join-Path $pairDir "right\repo"); ArtifactDir = New-Dir (Join-Path $pairDir "right\artifacts") }
}
$metricsBySide = @{
    left = (New-Metrics $pair.left.ArtifactDir $true)
    right = (New-Metrics $pair.right.ArtifactDir $true)
}
$manifest.SampleOrigin.generated_wrapper_sha256 = $script:TestWrapperSha
$proof = New-TaskspaceExternalEvidenceProof $pair $manifest $metricsBySide $sourceGuard
Assert-True ($proof.validator_fidelity.runtime_proven) "runtime proof did not accept complete structured markers"
$runtimeProof = Get-Content -Raw -Encoding UTF8 -LiteralPath $proof.runtime_proof_path | ConvertFrom-Json
Assert-True (@($runtimeProof.sides | Where-Object { -not $_.uv_cache_proven }).Count -eq 0) "runtime proof did not prove uv cache mount and hashes"
Assert-True (-not $proof.validator_fidelity.agent_cannot_read_validator_source) "declared false source isolation was promoted from placement proof alone"

$skippedPairDir = New-Dir (Join-Path $runDir "pair-skipped-after-timeout")
$skippedPair = [pscustomobject]@{
    PairDir = $skippedPairDir
    left = [pscustomobject]@{ RepoDir = New-Dir (Join-Path $skippedPairDir "left\repo"); ArtifactDir = New-Dir (Join-Path $skippedPairDir "left\artifacts") }
    right = [pscustomobject]@{ RepoDir = New-Dir (Join-Path $skippedPairDir "right\repo"); ArtifactDir = New-Dir (Join-Path $skippedPairDir "right\artifacts") }
}
$skippedMetricsBySide = @{
    left = (New-SkippedTimeoutMetrics $skippedPair.left.ArtifactDir)
    right = (New-SkippedTimeoutMetrics $skippedPair.right.ArtifactDir)
}
$skippedProof = New-TaskspaceExternalEvidenceProof $skippedPair $manifest $skippedMetricsBySide $sourceGuard
Assert-True ($skippedProof.validator_fidelity.runtime_proven) "runtime proof did not accept validation skip backed by passed pre-agent probe"
Assert-True ($skippedProof.validator_fidelity.validator_mount_proven) "mount proof did not accept validation skip backed by passed pre-agent probe"

$script:ForceBadUvCacheSource = $true
$badUvPairDir = New-Dir (Join-Path $runDir "pair-bad-uv-cache")
$badUvPair = [pscustomobject]@{
    PairDir = $badUvPairDir
    left = [pscustomobject]@{ RepoDir = New-Dir (Join-Path $badUvPairDir "left\repo"); ArtifactDir = New-Dir (Join-Path $badUvPairDir "left\artifacts") }
    right = [pscustomobject]@{ RepoDir = New-Dir (Join-Path $badUvPairDir "right\repo"); ArtifactDir = New-Dir (Join-Path $badUvPairDir "right\artifacts") }
}
$badUvMetrics = @{ left = (New-Metrics $badUvPair.left.ArtifactDir $true); right = (New-Metrics $badUvPair.right.ArtifactDir $true) }
$badUvProof = New-TaskspaceExternalEvidenceProof $badUvPair $manifest $badUvMetrics $sourceGuard
Assert-True (-not $badUvProof.validator_fidelity.runtime_proven) "uv cache inspect source mismatch was accepted"
$script:ForceBadUvCacheSource = $false

$manifestIsolated = $manifest.PSObject.Copy()
$manifestIsolated.ExternalBenchmark = $manifest.ExternalBenchmark.PSObject.Copy()
$manifestIsolated.ExternalBenchmark.validator_fidelity = [pscustomobject]@{
    official_runner_or_equivalent = $true
    agent_cannot_read_validator_source = $true
    e3_eligible = $true
}
$isolatedProof = New-TaskspaceExternalEvidenceProof $pair $manifestIsolated $metricsBySide $sourceGuard
Assert-True ($isolatedProof.validator_fidelity.official_runner_or_equivalent) "official equivalent proof did not accept source-hashed /tests protocol"
Assert-True ($isolatedProof.validator_fidelity.agent_cannot_read_validator_source) "source guard proof did not prove validator source isolation"

$runtimeVenvLeak = Join-Path $pair.left.RepoDir ".tbench-testing\lib\python3.11\site-packages\external-validator-source\probe.txt"
New-TestFile $runtimeVenvLeak "validator-looking dependency artifact"
$runtimeVenvProof = New-TaskspaceExternalEvidenceProof $pair $manifestIsolated $metricsBySide $sourceGuard
Assert-True ($runtimeVenvProof.validator_fidelity.agent_cannot_read_validator_source) "runtime .tbench-testing dependency tree was scanned as validator source leak"

$realRepoLeak = Join-Path $pair.right.RepoDir "external-validator-source\probe.txt"
New-TestFile $realRepoLeak "real validator leak"
$realLeakProof = New-TaskspaceExternalEvidenceProof $pair $manifestIsolated $metricsBySide $sourceGuard
Assert-True (-not $realLeakProof.validator_fidelity.agent_cannot_read_validator_source) "real repo validator source leak was hidden by bounded scan"
Remove-Item -LiteralPath $realRepoLeak -Force
Remove-Item -LiteralPath (Split-Path -Parent $realRepoLeak) -Force

$missingOfficialSourceManifest = $manifestIsolated.PSObject.Copy()
$missingOfficialSourceManifest.ExternalBenchmark = $manifestIsolated.ExternalBenchmark.PSObject.Copy()
$missingOfficialSourceManifest.ExternalBenchmark.adapter_metadata = $manifestIsolated.ExternalBenchmark.adapter_metadata.PSObject.Copy()
$missingOfficialSourceManifest.ExternalBenchmark.adapter_metadata.official_equivalence = $manifestIsolated.ExternalBenchmark.adapter_metadata.official_equivalence.PSObject.Copy()
$missingOfficialSourceManifest.ExternalBenchmark.adapter_metadata.official_equivalence.source_files = @([pscustomobject]@{
        path = Join-Path $officialRoot "missing-harness.py"
        relative_path = "missing-harness.py"
        current_sha256 = $officialHash
        pinned_blob_id = $officialBlob
        current_blob_id = $currentBlob
        matches_pinned_revision = $true
    })
$missingOfficialSourceProof = New-TaskspaceExternalEvidenceProof $pair $missingOfficialSourceManifest $metricsBySide $sourceGuard
Assert-True (-not $missingOfficialSourceProof.validator_fidelity.official_runner_or_equivalent) "missing official source file should downgrade proof, not throw"

$manifestMissingSource = $manifest.PSObject.Copy()
$manifestMissingSource.ExternalBenchmark = $manifest.ExternalBenchmark.PSObject.Copy()
$manifestMissingSource.ExternalBenchmark.validator_source_dir = "missing-source"
$missingProof = New-TaskspaceExternalEvidenceProof $pair $manifestMissingSource $metricsBySide $sourceGuard
Assert-True (-not $missingProof.validator_fidelity.agent_cannot_read_validator_source) "missing validator source proved isolation"

$manifestHashMismatch = $manifest.PSObject.Copy()
$manifestHashMismatch.SampleOrigin = [pscustomobject]@{ original_validator_sha256 = ("0" * 64) }
$hashMismatchProof = New-TaskspaceExternalEvidenceProof $pair $manifestHashMismatch $metricsBySide $sourceGuard
Assert-True (-not $hashMismatchProof.validator_fidelity.agent_cannot_read_validator_source) "validator source hash mismatch proved isolation"

$wrapperMismatchManifest = $manifestIsolated.PSObject.Copy()
$wrapperMismatchManifest.SampleOrigin = [pscustomobject]@{ original_validator_sha256 = $validatorSha; generated_wrapper_sha256 = ("0" * 64) }
$wrapperMismatchProof = New-TaskspaceExternalEvidenceProof $pair $wrapperMismatchManifest $metricsBySide $sourceGuard
Assert-True (-not $wrapperMismatchProof.validator_fidelity.runtime_proven) "wrapper hash mismatch was accepted"

$badGuardPath = Join-Path $pairDir "bad-source-guard-proof.json"
$badGuard = $sourceGuard.PSObject.Copy()
$badGuard.proof_path = $badGuardPath
$badGuard.files = @([pscustomobject]@{
    path = $validatorFile
    file_sha256_before_protect = (Get-FileHash -Algorithm SHA256 -LiteralPath $validatorFile).Hash.ToLowerInvariant()
    deny_exit_code = 0
    probes_after_protect = @(
        [pscustomobject]@{ kind = "current_powershell"; available = $true; read_denied = $true },
        [pscustomobject]@{ kind = "powershell_child"; available = $true; read_denied = $false },
        [pscustomobject]@{ kind = "cmd_child"; available = $true; read_denied = $true }
    )
})
New-TestFile $badGuardPath ($badGuard | ConvertTo-Json -Depth 8)
$badGuardProof = New-TaskspaceExternalEvidenceProof $pair $manifestIsolated $metricsBySide $badGuard
Assert-True (-not $badGuardProof.validator_fidelity.agent_cannot_read_validator_source) "forged source guard object/proof was accepted"

$noReadonlyArtifact = New-Dir (Join-Path $pairDir "left-no-readonly\artifacts")
New-ProofLog (Join-Path $noReadonlyArtifact "validation.stdout.log") $true
(Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $noReadonlyArtifact "validation.stdout.log")).Replace("validator_mount_readonly=true", "validator_mount_readonly=false") |
    Set-Content -LiteralPath (Join-Path $noReadonlyArtifact "validation.stdout.log") -Encoding UTF8
New-TestFile (Join-Path $noReadonlyArtifact "validation.stderr.log") ""
New-TestFile (Join-Path $noReadonlyArtifact "whale-exec.jsonl") ""
New-TestFile (Join-Path $noReadonlyArtifact "whale-exec.stderr.log") ""
New-TestFile (Join-Path $noReadonlyArtifact "last-message.md") ""
$metricsBySide.left = [pscustomobject]@{
    logical_mode = "standard"; public_validation_exit_code = 0
    validation_stdout_path = Join-Path $noReadonlyArtifact "validation.stdout.log"
    validation_stderr_path = Join-Path $noReadonlyArtifact "validation.stderr.log"
    jsonl_path = Join-Path $noReadonlyArtifact "whale-exec.jsonl"
    stderr_path = Join-Path $noReadonlyArtifact "whale-exec.stderr.log"
    last_message_path = Join-Path $noReadonlyArtifact "last-message.md"
    business_success = $true
}
$noReadonlyProof = New-TaskspaceExternalEvidenceProof $pair $manifest $metricsBySide $sourceGuard
Assert-True (-not $noReadonlyProof.validator_fidelity.official_runner_or_equivalent) "missing readonly marker kept official runner equivalence"

$missingInspectArtifact = New-Dir (Join-Path $pairDir "left-missing-inspect\artifacts")
New-ProofLog (Join-Path $missingInspectArtifact "validation.stdout.log") $true $false
New-TestFile (Join-Path $missingInspectArtifact "validation.stderr.log") ""
New-TestFile (Join-Path $missingInspectArtifact "whale-exec.jsonl") ""
New-TestFile (Join-Path $missingInspectArtifact "whale-exec.stderr.log") ""
New-TestFile (Join-Path $missingInspectArtifact "last-message.md") ""
$metricsBySide.left = [pscustomobject]@{
    logical_mode = "standard"; public_validation_exit_code = 0
    validation_stdout_path = Join-Path $missingInspectArtifact "validation.stdout.log"
    validation_stderr_path = Join-Path $missingInspectArtifact "validation.stderr.log"
    jsonl_path = Join-Path $missingInspectArtifact "whale-exec.jsonl"
    stderr_path = Join-Path $missingInspectArtifact "whale-exec.stderr.log"
    last_message_path = Join-Path $missingInspectArtifact "last-message.md"
    business_success = $true
}
$missingInspectProof = New-TaskspaceExternalEvidenceProof $pair $manifest $metricsBySide $sourceGuard
Assert-True (-not $missingInspectProof.validator_fidelity.runtime_proven) "runtime proof accepted missing Docker inspect artifact"

$metricsBySide.left = New-Metrics (New-Dir (Join-Path $pairDir "left2\artifacts")) $false
$spoofProof = New-TaskspaceExternalEvidenceProof $pair $manifest $metricsBySide $sourceGuard
Assert-True (-not $spoofProof.validator_fidelity.runtime_proven) "runtime proof accepted logs without wrapper marker"

$promptGuardOk = [pscustomobject]@{ invalid_prompt = $false; manual_review_required = $false }
$e3Config = [pscustomobject]@{ claim_scope = "scope"; minimum_repeats = 5 }
$origin = [pscustomobject]@{
    type = "external_benchmark"; source = "terminal-bench"; source_version = "rev"
    source_url = "https://example.invalid"; license = "license"; data_policy = "policy"
    sample_id = "sample"; original_prompt_sha256 = "abc"; original_validator_sha256 = $validatorSha
}
$external = [pscustomobject]@{ name = "terminal-bench"; adapter_version = "test"; validator_fidelity = [pscustomobject]@{} }
$sideOutcomes = [pscustomobject]@{ standard_success = $false; taskspace_success = $true }
$badDecision = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $origin $manifestIsolated.ExternalBenchmark $e3Config $true $true 5 "include_no_clear_delta" $false $isolatedProof $sideOutcomes
Assert-True (@($badDecision.e3_gate_failures) -contains "e3_human_review_decision_inconsistent_with_outcome") "one-sided outcome matched include_no_clear_delta"
$bothSuccessWrongDirection = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $origin $manifestIsolated.ExternalBenchmark $e3Config $true $true 5 "include_taskspace_better" $false $isolatedProof ([pscustomobject]@{ standard_success = $true; taskspace_success = $true })
Assert-True (@($bothSuccessWrongDirection.e3_gate_failures) -contains "e3_human_review_decision_inconsistent_with_outcome") "both-success pair counted as directional TaskSpace benefit"
$disagreementEvidence = Get-TaskspaceEvidenceGate 5 $promptGuardOk "hard_sandbox" "known" $false $true $false $true "deferred_materialization_allowed" "E3" $origin $manifestIsolated.ExternalBenchmark $e3Config $true $true 5 "include_no_clear_delta" $true $isolatedProof ([pscustomobject]@{ standard_success = $true; taskspace_success = $true })
Assert-True (@($disagreementEvidence.e3_gate_failures) -contains "e3_human_review_disagreement") "human review disagreement did not block E3"

$auditPair = New-Dir (Join-Path $runDir "audit-pair")
foreach ($relative in @(Get-TaskspaceRequiredAuditArtifacts $pairDir)) {
    $source = Join-Path $pairDir $relative
    if (Test-Path -LiteralPath $source) {
        $dest = Join-Path $auditPair $relative
        New-Item -ItemType Directory -Path (Split-Path -Parent $dest) -Force | Out-Null
        Copy-Item -LiteralPath $source -Destination $dest -Force
    } else {
        New-TestFile (Join-Path $auditPair $relative)
    }
}
New-TestFile (Join-Path $auditPair "manifest.resolved.json") (@{ external_benchmark = @{ name = "terminal-bench" } } | ConvertTo-Json -Depth 5)
New-TestFile (Join-Path $auditPair "right\artifacts\observability\action-map-observability.json") "{}"
foreach ($side in @("left", "right")) {
    New-TestFile (Join-Path $auditPair "$side\artifacts\external-validator-runtime\terminal-bench-runtime-manifest.json") (@{ proof_nonce = "0123456789abcdef0123456789abcdef" } | ConvertTo-Json -Depth 5)
    New-TestFile (Join-Path $auditPair "$side\artifacts\external-validator-runtime\validation-cleanup-result.json") (@{ classification = "ok"; identity_matched = $true } | ConvertTo-Json -Depth 5)
}
$basis = @(Get-TaskspaceRequiredAuditArtifacts $auditPair)
Assert-True (-not ($basis -contains "pair-report.md")) "audit required artifact basis includes generated pair-report.md"
$hashes = [ordered]@{}
foreach ($relative in $basis) { $hashes[$relative] = (Get-FileHash -Algorithm SHA256 -LiteralPath (Join-Path $auditPair $relative)).Hash.ToLowerInvariant() }
@{
    reviewer = "codex"; date = "2026-06-02"; artifact_basis = $basis; artifact_hashes = $hashes
    decision = "include_no_clear_delta"; claim_scope = "scope"; disagreement = $false
} | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath (Join-Path $auditPair "audit-review.json") -Encoding UTF8
$weakAudit = Get-TaskspaceAuditReview $auditPair "" 0 "scope"
Assert-True (@($weakAudit.failures | Where-Object { $_ -like "audit_attestation_missing_or_false:*" }).Count -gt 0) "external audit without attestations was accepted"

if ($failures.Count -gt 0) {
    Write-Host "TaskSpace E3 proof harness self-test: FAIL"
    foreach ($failure in $failures) { Write-Host "- $failure" }
    exit 1
}
Write-Host "TaskSpace E3 proof harness self-test: PASS"
Write-Host "RunRoot: $runDir"
