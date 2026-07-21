function Assert-CandidateActivationTargets {
    param([object]$Candidate, [object]$ActiveAuthority)
    $l4Targets = @($Candidate.activation_targets.L4)
    $l5Targets = @($Candidate.activation_targets.L5)
    Assert-Equal $l4Targets.Count 1 "Candidate must declare exactly one L4 activation target"
    Assert-Equal $l5Targets.Count 3 "Candidate must declare exactly three L5 activation targets"
    $l4 = $l4Targets[0]
    Assert-Equal ([string]$l4.artifact_role) "l4_schema" "Candidate L4 activation role drifted"
    Assert-Equal ([string]$l4.path) ([string]$Candidate.artifact_hashes.l4_schema.path) "Candidate L4 activation path is not identity-bound"
    Assert-Equal ([string]$l4.sha256) ([string]$Candidate.artifact_hashes.l4_schema.sha256) "Candidate L4 activation hash is not identity-bound"
    $typed = @($l5Targets | Where-Object { [string]$_.artifact_role -eq "typed_outcome" })
    $projection = @($l5Targets | Where-Object { [string]$_.artifact_role -eq "projection_baseline" })
    $lifecycle = @($l5Targets | Where-Object { [string]$_.artifact_role -eq "lifecycle_baseline" })
    Assert-Equal $typed.Count 1 "Candidate must declare exactly one typed outcome activation target"
    Assert-Equal $projection.Count 1 "Candidate must declare exactly one retained projection target"
    Assert-Equal $lifecycle.Count 1 "Candidate must declare exactly one retained lifecycle target"
    Assert-Equal ([string]$typed[0].path) ([string]$Candidate.artifact_hashes.typed_outcome.path) "Candidate typed outcome activation path is not identity-bound"
    Assert-Equal ([string]$typed[0].sha256) ([string]$Candidate.artifact_hashes.typed_outcome.sha256) "Candidate typed outcome activation hash is not identity-bound"
    $baselineProjection = @($ActiveAuthority.selected_targets | Where-Object { [string]$_.layer -eq "L5-projection" })
    $baselineLifecycle = @($ActiveAuthority.selected_targets | Where-Object { [string]$_.layer -eq "L5-lifecycle" })
    Assert-Equal $baselineProjection.Count 1 "Active authority must contain exactly one projection baseline"
    Assert-Equal $baselineLifecycle.Count 1 "Active authority must contain exactly one lifecycle baseline"
    Assert-Equal ([string]$projection[0].path) ([string]$baselineProjection[0].artifact) "Candidate retained projection path drifted from active authority"
    Assert-Equal ([string]$projection[0].sha256) ([string]$baselineProjection[0].sha256) "Candidate retained projection hash drifted from active authority"
    Assert-Equal ([string]$projection[0].activation_phase) ([string]$baselineProjection[0].activation_phase) "Candidate retained projection phase drifted from active authority"
    Assert-Equal ([string]$lifecycle[0].path) ([string]$baselineLifecycle[0].artifact) "Candidate retained lifecycle path drifted from active authority"
    Assert-Equal ([string]$lifecycle[0].sha256) ([string]$baselineLifecycle[0].sha256) "Candidate retained lifecycle hash drifted from active authority"
    Assert-Equal ([string]$lifecycle[0].activation_phase) ([string]$baselineLifecycle[0].activation_phase) "Candidate retained lifecycle phase drifted from active authority"
}

function Assert-CandidateArtifactSchemaContract {
    param([string]$SchemaPath)
    $sha = "0" * 64
    $metrics = @{transition_carrier_rate = 1; carrier_execution_started_rate = 1; correctness_rate = 1}
    $thresholds = @{transition_carrier_rate_min = 1; correctness_rate_min = 1; request_amplification_max = 1}
    $oracles = 1..4 | ForEach-Object { @{id = "oracle-$_"; given = "state $_"; when = "event $_"; then = "fact $_"} }
    $capabilityEntries = @("Function", "Namespace", "ToolSearch", "LocalShell", "ImageGeneration", "WebSearch", "Freeform") |
        ForEach-Object { @{wire_api = "responses"; tool_spec = $_; invocation_source = "builtin"; disposition = "non_carrier"; reason = "fixture disposition"} }
    $samples = @(
        @{id = "simple"; fixture_sha256 = $sha; repeats = 3},
        @{id = "complex"; fixture_sha256 = $sha; repeats = 3}
    )
    $validBodies = @(
        @{schema_version = 1; artifact_role = "l4_schema"; provider_tool = @{name = "exec_command"; wire_kind = "function"; carrier_field = "taskspace_transition"; business_parameters = @{type = "object"; properties = @{cmd = @{type = "string"}}}; standard_unchanged = $true}},
        @{schema_version = 1; artifact_role = "transition_schema"; transition_schema = @{schema_id = "r7-transition-v1"; actions = @("initialize_map", "bind_node", "complete_then_continue"); required_fields = @("action", "expected_revision"); standalone_nonterminal_allowed = $false}},
        @{schema_version = 1; artifact_role = "typed_outcome"; outcome_type = "TaskSpaceCarrierOutcome"; outcome_variants = @("RejectedBeforeCommit", "CommittedNotExecuted", "Executed"); tool_output_preservation = "opaque"; post_hook_separate = $true},
        @{schema_version = 1; artifact_role = "lifecycle_oracle_v2"; oracle_version = 2; oracles = $oracles},
        @{schema_version = 1; artifact_role = "capability_matrix"; matrix_id = "r7-carrier-matrix-v1"; entry_closure_sha256 = $sha; entries = $capabilityEntries},
        @{schema_version = 1; artifact_role = "rollback_manifest"; baseline_authority_sha256 = $sha; baseline_production_sha256 = $sha; restore_targets = @(@{path = "authority.json"; sha256 = $sha}, @{path = "production.json"; sha256 = $sha}); verification_commands = @("verify")},
        @{schema_version = 1; artifact_role = "continuous_action_evaluation"; evaluation_id = "ca-eval-v1"; sealed = $true; samples = $samples; metrics = $metrics; thresholds = $thresholds},
        @{schema_version = 1; artifact_role = "fla8_evaluation_v2"; evaluation_id = "fla8-v2"; sealed = $true; held_out_identity = @{suite_id = "held-out"; manifest_sha256 = $sha; content_mounted = $false}; metrics = $metrics; thresholds = $thresholds}
    )
    foreach ($body in $validBodies) {
        $json = $body | ConvertTo-Json -Depth 30
        Assert-True ($json | Test-Json -SchemaFile $SchemaPath -ErrorAction Stop) "Role-specific artifact schema rejected $($body.artifact_role)"
    }
    $emptyPayloads = @(
        @{schema_version = 1; artifact_role = "l4_schema"; provider_tool = @{}},
        @{schema_version = 1; artifact_role = "transition_schema"; transition_schema = @{}},
        @{schema_version = 1; artifact_role = "typed_outcome"; outcome_variants = @("Executed")},
        @{schema_version = 1; artifact_role = "lifecycle_oracle_v2"; oracles = @("carrier")},
        @{schema_version = 1; artifact_role = "capability_matrix"; entries = @("exec")},
        @{schema_version = 1; artifact_role = "rollback_manifest"; restore_targets = @("authority")},
        @{schema_version = 1; artifact_role = "continuous_action_evaluation"; samples = @("simple"); metrics = @{}; thresholds = @{}},
        @{schema_version = 1; artifact_role = "fla8_evaluation_v2"; held_out_identity = @{}; metrics = @{}; thresholds = @{}}
    )
    foreach ($body in $emptyPayloads) {
        $json = $body | ConvertTo-Json -Depth 20
        Assert-True (-not ($json | Test-Json -SchemaFile $SchemaPath -ErrorAction SilentlyContinue)) "Role-specific artifact schema accepted an empty $($body.artifact_role) payload"
    }
}

function Assert-CandidateActivationContract {
    param([object]$Candidate)
    $activeAuthorityRaw = Get-GitBlobText ([string]$Candidate.active_authority.git_commit) ([string]$Candidate.active_authority.path)
    $activeAuthority = $activeAuthorityRaw | ConvertFrom-Json -Depth 50
    $activeProductionRaw = Get-GitBlobText ([string]$Candidate.active_production_manifest.git_commit) ([string]$Candidate.active_production_manifest.path)
    $activeProduction = $activeProductionRaw | ConvertFrom-Json -Depth 50
    Assert-CandidateActivationSnapshot $Candidate "evaluation_candidate" $activeAuthorityRaw $activeAuthority $activeProduction $activeProductionRaw ([string]$Candidate.active_authority.sha256) ([string]$Candidate.active_production_manifest.sha256)

    $driftedProduction = $activeProductionRaw | ConvertFrom-Json -Depth 50
    $driftedProduction.layers[3].selected_targets += [pscustomobject]@{artifact = "old-target"; sha256 = ("0" * 64); activation_phase = "old"}
    $driftedProductionRaw = $driftedProduction | ConvertTo-Json -Depth 50
    Assert-Throws {
        Assert-CandidateActivationSnapshot $Candidate "reverted" $activeAuthorityRaw $activeAuthority $driftedProduction $driftedProductionRaw ([string]$Candidate.active_authority.sha256) (Get-TextSha256 $driftedProductionRaw)
    } "Reverted candidate accepted a production manifest that was not byte-exact baseline"

    $promotedAuthority = $activeAuthorityRaw | ConvertFrom-Json -Depth 50
    $retainedTargets = @($promotedAuthority.selected_targets | Where-Object { [string]$_.layer -ne "L4" -and -not ([string]$_.layer).StartsWith("L5-", [System.StringComparison]::Ordinal) })
    $candidateAuthorityTargets = @()
    $candidateL4 = $Candidate.activation_targets.L4[0]
    $candidateAuthorityTargets += [pscustomobject]@{layer = "L4"; implementation_status = "active_repair_verified"; activation_phase = $candidateL4.activation_phase; artifact = $candidateL4.path; sha256 = $candidateL4.sha256}
    foreach ($target in @($Candidate.activation_targets.L5)) {
        $authorityLayer = if ([string]$target.artifact_role -eq "typed_outcome") { "L5-result" } elseif ([string]$target.artifact_role -eq "projection_baseline") { "L5-projection" } else { "L5-lifecycle" }
        $candidateAuthorityTargets += [pscustomobject]@{layer = $authorityLayer; implementation_status = "active_repair_verified"; activation_phase = $target.activation_phase; artifact = $target.path; sha256 = $target.sha256}
    }
    $promotedAuthority.selected_targets = @($retainedTargets) + @($candidateAuthorityTargets)
    $promotedAuthorityRaw = $promotedAuthority | ConvertTo-Json -Depth 50
    $promotedProduction = $activeProductionRaw | ConvertFrom-Json -Depth 50
    $promotedProduction.source_authority.sha256 = Get-TextSha256 $promotedAuthorityRaw
    $promotedProduction | Add-Member -NotePropertyName promoted_candidate_id -NotePropertyValue ([string]$Candidate.candidate_id) -Force
    foreach ($layer in @("L4", "L5")) {
        $productionLayer = @($promotedProduction.layers | Where-Object { [string]$_.id -eq $layer })[0]
        $productionLayer.selected_targets = @($Candidate.activation_targets.$layer | ForEach-Object { [pscustomobject]@{artifact = $_.path; sha256 = $_.sha256; activation_phase = $_.activation_phase} })
    }
    Assert-CandidateActivationSnapshot $Candidate "promoted" $promotedAuthorityRaw $promotedAuthority $promotedProduction
    $promotedProduction.layers[3].selected_targets += [pscustomobject]@{artifact = "old-target"; sha256 = ("0" * 64); activation_phase = "old"}
    Assert-Throws {
        Assert-CandidateActivationSnapshot $Candidate "promoted" $promotedAuthorityRaw $promotedAuthority $promotedProduction
    } "Promoted candidate accepted an undeclared production target"
}

function Assert-CandidateManifestIntegrity {
    param([object]$Candidate, [string]$ManifestPath = "")
    $candidateId = [string]$Candidate.candidate_id
    Assert-Equal ([string]$Candidate.contract_id) "r7-taskspace-five-layer-candidate-$candidateId" "Candidate contract id does not match candidate id"
    Assert-Equal (Get-CandidateContentId $Candidate) $candidateId "Candidate content id does not match active snapshot and artifact hashes"
    $candidateCommit = [string]$Candidate.candidate_commit
    & git -C $repoRoot cat-file -e "$candidateCommit^{commit}" 2>$null
    Assert-True ($LASTEXITCODE -eq 0) "Candidate commit is unavailable: $candidateCommit"
    Assert-Equal ([string]$Candidate.active_authority.contract_id) "r7-five-layer-contract-authority-v1" "Candidate active authority id drifted"
    Assert-Equal ([string]$Candidate.active_authority.path) "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json" "Candidate active authority path drifted"
    $authorityBlob = Get-GitBlobText ([string]$Candidate.active_authority.git_commit) ([string]$Candidate.active_authority.path)
    Assert-Equal (Get-GitBlobSha256 ([string]$Candidate.active_authority.git_commit) ([string]$Candidate.active_authority.path)) ([string]$Candidate.active_authority.sha256) "Candidate active authority snapshot hash drifted"
    $activeAuthorityBody = $authorityBlob | ConvertFrom-Json -Depth 50
    Assert-Equal ([string]$Candidate.source_authority.contract_id) ([string]$Candidate.active_authority.contract_id) "Candidate source and active authority ids differ"
    Assert-Equal ([string]$Candidate.source_authority.path) ([string]$Candidate.active_authority.path) "Candidate source and active authority paths differ"
    Assert-Equal ([string]$Candidate.source_authority.sha256) ([string]$Candidate.active_authority.sha256) "Candidate source and active authority hashes differ"
    $productionSnapshot = $Candidate.active_production_manifest
    $productionBlob = Get-GitBlobText ([string]$productionSnapshot.git_commit) ([string]$productionSnapshot.path)
    Assert-Equal (Get-GitBlobSha256 ([string]$productionSnapshot.git_commit) ([string]$productionSnapshot.path)) ([string]$productionSnapshot.sha256) "Candidate active production snapshot hash drifted"
    $productionSnapshotBody = $productionBlob | ConvertFrom-Json -Depth 50
    Assert-Equal ([string]$productionSnapshotBody.contract_id) ([string]$productionSnapshot.contract_id) "Candidate active production contract id drifted"
    Assert-Equal ([string]$productionSnapshotBody.source_authority.sha256) ([string]$Candidate.active_authority.sha256) "Candidate production snapshot does not use the active authority snapshot"
    Assert-CandidateActivationTargets $Candidate $activeAuthorityBody
    $namespaceRoot = [System.IO.Path]::GetFullPath((Join-Path $repoRoot "benchmarks/taskspace/r7/candidates/$candidateId"))
    $namespacePrefix = $namespaceRoot.TrimEnd([System.IO.Path]::DirectorySeparatorChar) + [System.IO.Path]::DirectorySeparatorChar
    $seenPaths = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    $seenHashes = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    foreach ($artifact in $Candidate.artifact_hashes.psobject.Properties) {
        Assert-Equal ([string]$artifact.Value.artifact_role) ([string]$artifact.Name) "Candidate artifact role marker drifted"
        $relativePath = [string]$artifact.Value.path
        $expectedPrefix = "benchmarks/taskspace/r7/candidates/$candidateId/"
        Assert-True $relativePath.StartsWith($expectedPrefix, [System.StringComparison]::Ordinal) "Candidate artifact escaped its namespace: $relativePath"
        $canonicalPath = [System.IO.Path]::GetFullPath((Join-Path $repoRoot $relativePath))
        Assert-True $canonicalPath.StartsWith($namespacePrefix, [System.StringComparison]::Ordinal) "Candidate artifact escaped its canonical namespace: $relativePath"
        $pathCursor = [System.IO.Path]::GetFullPath($repoRoot)
        foreach ($segment in $relativePath.Split([char[]]@('/', '\'), [System.StringSplitOptions]::RemoveEmptyEntries)) {
            $pathCursor = Join-Path $pathCursor $segment
            if (Test-Path -LiteralPath $pathCursor) {
                $pathItem = Get-Item -LiteralPath $pathCursor -Force
                Assert-True (($pathItem.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0) "Candidate artifact path contains a symlink: $relativePath"
            }
        }
        Assert-True ($seenPaths.Add($canonicalPath)) "Candidate artifact paths must be unique: $relativePath"
        Assert-True ($seenHashes.Add([string]$artifact.Value.sha256)) "Candidate artifact roles must not reuse one blob hash"
        if (-not [string]::IsNullOrWhiteSpace($ManifestPath)) {
            Assert-True (Test-Path -LiteralPath $canonicalPath -PathType Leaf) "Candidate artifact missing: $relativePath"
            $resolvedArtifactPath = (Resolve-Path -LiteralPath $canonicalPath).Path
            Assert-True $resolvedArtifactPath.StartsWith($namespacePrefix, [System.StringComparison]::Ordinal) "Candidate artifact escaped its resolved namespace: $relativePath"
            $item = Get-Item -LiteralPath $canonicalPath -Force
            Assert-True (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -eq 0) "Candidate artifact must not be a symlink: $relativePath"
            Assert-Equal (Get-Sha256 $canonicalPath) ([string]$artifact.Value.sha256) "Candidate artifact hash drifted: $relativePath"
            Assert-Equal (Get-GitBlobSha256 $candidateCommit $relativePath) ([string]$artifact.Value.sha256) "Candidate artifact was not frozen by candidate commit: $relativePath"
            $treeEntry = (& git -C $repoRoot ls-tree $candidateCommit -- $relativePath).Trim()
            Assert-True $treeEntry.StartsWith("100644 blob ", [System.StringComparison]::Ordinal) "Candidate artifact must be a regular non-executable Git blob: $relativePath"
            $artifactBody = Get-Content -Raw -Encoding UTF8 -LiteralPath $canonicalPath | ConvertFrom-Json -Depth 50
            Assert-Equal ([string]$artifactBody.artifact_role) ([string]$artifact.Name) "Candidate artifact content role drifted: $relativePath"
            Assert-True (-not [string]::IsNullOrWhiteSpace([string]$artifactBody.schema_version)) "Candidate artifact schema_version missing: $relativePath"
            $artifactSchemaPath = Join-Path $repoRoot ([string]$authority.candidate_registry.artifact_schema)
            $artifactRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $canonicalPath
            Assert-True ($artifactRaw | Test-Json -SchemaFile $artifactSchemaPath -ErrorAction Stop) "Candidate artifact does not match its role-specific schema: $relativePath"
        }
    }
    if (-not [string]::IsNullOrWhiteSpace($ManifestPath)) {
        $expectedPath = Join-Path $repoRoot "benchmarks/taskspace/r7/candidates/$candidateId/manifest.json"
        Assert-Equal ([System.IO.Path]::GetFullPath($ManifestPath)) ([System.IO.Path]::GetFullPath($expectedPath)) "Candidate manifest path does not match candidate id"
    }
}

function Assert-CandidateHistoryIntegrity {
    param([string]$ManifestPath, [string]$CurrentRaw, [object]$Authority)
    $relativePath = [System.IO.Path]::GetRelativePath($repoRoot, $ManifestPath).Replace("\", "/")
    $historyCommits = @(& git -C $repoRoot log --first-parent --reverse --format=%H -- $relativePath)
    $previousStatus = ""
    $lastRaw = $null
    foreach ($commit in $historyCommits) {
        & git -C $repoRoot cat-file -e "${commit}:$relativePath" 2>$null
        Assert-True ($LASTEXITCODE -eq 0) "Candidate manifest history contains a deletion: $relativePath at $commit"
        $candidateRaw = Get-GitBlobText $commit $relativePath
        Assert-True ($candidateRaw | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction Stop) "Historical candidate manifest does not match schema: $relativePath at $commit"
        $candidate = $candidateRaw | ConvertFrom-Json -Depth 50
        Assert-CandidateManifestIntegrity $candidate $ManifestPath
        Assert-CandidateStateHistory $candidate $previousStatus $Authority
        $authorityRawAtCommit = Get-GitBlobText $commit "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"
        $authorityAtCommit = $authorityRawAtCommit | ConvertFrom-Json -Depth 50
        $productionRawAtCommit = Get-GitBlobText $commit "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"
        $productionAtCommit = $productionRawAtCommit | ConvertFrom-Json -Depth 50
        $authorityHashAtCommit = Get-GitBlobSha256 $commit "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"
        $productionHashAtCommit = Get-GitBlobSha256 $commit "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"
        Assert-CandidateActivationSnapshot $candidate ([string]$candidate.candidate_status) $authorityRawAtCommit $authorityAtCommit $productionAtCommit $productionRawAtCommit $authorityHashAtCommit $productionHashAtCommit
        $previousStatus = [string]$candidate.candidate_status
        $lastRaw = $candidateRaw
    }
    if ($null -eq $lastRaw -or (Get-TextSha256 $lastRaw) -cne (Get-TextSha256 $CurrentRaw)) {
        Assert-True ($CurrentRaw | Test-Json -SchemaFile $manifestSchemaPath -ErrorAction Stop) "Worktree candidate manifest does not match schema: $relativePath"
        $candidate = $CurrentRaw | ConvertFrom-Json -Depth 50
        Assert-CandidateManifestIntegrity $candidate $ManifestPath
        Assert-CandidateStateHistory $candidate $previousStatus $Authority
        $currentAuthorityRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityPath
        $currentProductionRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath
        $currentProduction = $currentProductionRaw | ConvertFrom-Json -Depth 50
        Assert-CandidateActivationSnapshot $candidate ([string]$candidate.candidate_status) $currentAuthorityRaw $Authority $currentProduction $currentProductionRaw (Get-Sha256 $authorityPath) (Get-Sha256 $manifestPath)
    }
    $currentCandidate = $CurrentRaw | ConvertFrom-Json -Depth 50
    $currentAuthorityRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $authorityPath
    $currentProductionRaw = Get-Content -Raw -Encoding UTF8 -LiteralPath $manifestPath
    $currentProduction = $currentProductionRaw | ConvertFrom-Json -Depth 50
    Assert-CandidateActivationSnapshot $currentCandidate ([string]$currentCandidate.candidate_status) $currentAuthorityRaw $Authority $currentProduction $currentProductionRaw (Get-Sha256 $authorityPath) (Get-Sha256 $manifestPath)
}
