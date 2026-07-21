param(
    [ValidateSet("PreAnchor", "Anchored")]
    [string]$Mode = "PreAnchor"
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "r7-v2-toolchain-core.ps1")
. (Join-Path $PSScriptRoot "r7-v2-artifact-fixtures.ps1")

function Assert-True {
    param([bool]$Condition, [string]$Code)
    if (-not $Condition) { throw $Code }
}

function Assert-Throws {
    param([scriptblock]$Action, [string]$Code)
    $threw = $false
    try { & $Action } catch { $threw = $true }
    if (-not $threw) { throw $Code }
}

function New-Hash {
    param([object]$Seed)
    Get-R7Sha256Text ([string]$Seed)
}

function Test-CandidateSchema {
    param([string]$SchemaPath)
    $id = New-Hash 'a'
    $artifactHashes = [pscustomobject][ordered]@{}
    $roles = @($script:R7ArtifactNames.Keys)
    for ($index = 0; $index -lt $roles.Count; $index++) {
        $role = $roles[$index]
        $artifactHashes | Add-Member -NotePropertyName $role -NotePropertyValue ([pscustomobject]@{artifact_role = $role; path = "$script:R7CandidateRoot/$id/$($script:R7ArtifactNames[$role])"; sha256 = New-Hash ([char](65 + $index)); git_mode = "100644"})
    }
    $patch = [pscustomobject]@{op = "replace"; path = "/candidate_status"; old_value_sha256 = (New-Hash 'b'); value = "promoted"; new_value_sha256 = (New-Hash 'c')}
    $authorityPatches = @(0..8 | ForEach-Object {
        [pscustomobject]@{op = "replace"; path = "/authority/$_"; old_value_sha256 = (New-Hash 'b'); value = $_; new_value_sha256 = (New-Hash 'c')}
    })
    $productionPatches = @(0..11 | ForEach-Object {
        [pscustomobject]@{op = "replace"; path = "/production/$_"; old_value_sha256 = (New-Hash 'b'); value = $_; new_value_sha256 = (New-Hash 'c')}
    })
    $candidate = [pscustomobject][ordered]@{
        schema_version = 2; contract_id = "r7-continuous-action-candidate-$id"; contract_status = "candidate_record"; candidate_id = $id; candidate_commit = "1" * 40; candidate_status = "evaluation_candidate"
        baseline_anchor = [pscustomobject]@{path = $script:R7BaselineAnchorPath; first_add_commit = "1" * 40; anchored_parent_commit = "2" * 40; sha256 = (New-Hash 'd')}
        toolchain_anchor = [pscustomobject]@{path = $script:R7ToolchainAnchorPath; first_add_commit = "3" * 40; anchored_parent_commit = "4" * 40; sha256 = (New-Hash 'e')}
        active_authority = [pscustomobject]@{contract_id = "authority"; path = $script:R7AuthorityPath; git_commit = "2" * 40; sha256 = (New-Hash 'f'); git_mode = "100644"}
        active_production_manifest = [pscustomobject]@{contract_id = "production"; path = $script:R7ProductionPath; git_commit = "2" * 40; sha256 = (New-Hash '1'); git_mode = "100644"}
        activation_targets = [pscustomobject][ordered]@{
            activation_through = "FLA-3.5"; authority_contract_status = "production_active_through_fla3_5_with_carrier_repair"; production_manifest_version = "1.0.5"
            blocking_repair = [pscustomobject]@{id = "FLA-3.5-continuous-action-regression-repair"; implementation_status = "active_verified"}
            production_runtime_status = [pscustomobject]@{L4 = "carrier_repair_active"; L5 = "carrier_result_repair_active_projection_baseline"}
            L4 = @([pscustomobject]@{artifact_role = "l4_schema"; authority_layer = "L4"; implementation_status = "active_repair_verified"; path = $artifactHashes.l4_schema.path; sha256 = $artifactHashes.l4_schema.sha256; activation_phase = "FLA-3.5"})
            L5 = @(
                [pscustomobject]@{artifact_role = "typed_outcome"; authority_layer = "L5-result"; implementation_status = "active_repair_verified"; path = $artifactHashes.typed_outcome.path; sha256 = $artifactHashes.typed_outcome.sha256; activation_phase = "FLA-3.5"},
                [pscustomobject]@{artifact_role = "projection_baseline"; authority_layer = "L5-projection"; implementation_status = "selected_baseline"; path = "projection"; sha256 = (New-Hash '2'); activation_phase = "FLA-7"},
                [pscustomobject]@{artifact_role = "lifecycle_baseline"; authority_layer = "L5-lifecycle"; implementation_status = "selected_not_implemented"; path = "lifecycle"; sha256 = (New-Hash '3'); activation_phase = "FLA-7"}
            )
        }
        artifact_hashes = $artifactHashes
        promotion = [pscustomobject]@{changed_paths = @($script:R7AuthorityPath, $script:R7ProductionPath, "$script:R7CandidateRoot/$id/manifest.json"); authority_patch = $authorityPatches; production_patch = $productionPatches; candidate_patch = @($patch)}
        status_evidence = [pscustomobject]@{event_kind = "candidate_created"; evidence_path = "evidence.json"; evidence_sha256 = (New-Hash '4')}
    }
    $json = $candidate | ConvertTo-Json -Depth 100
    Assert-True ($json | Test-Json -SchemaFile $SchemaPath -ErrorAction Stop) "R7_TEST_VALID_CANDIDATE_REJECTED"
    $candidate.promotion.authority_patch = @($authorityPatches | Select-Object -First 8)
    Assert-True (-not (($candidate | ConvertTo-Json -Depth 100) | Test-Json -SchemaFile $SchemaPath -ErrorAction SilentlyContinue)) "R7_TEST_INCOMPLETE_PATCH_ACCEPTED"
    $candidate.promotion.authority_patch = $authorityPatches
    $candidate.schema_version = 1
    Assert-True (-not (($candidate | ConvertTo-Json -Depth 100) | Test-Json -SchemaFile $SchemaPath -ErrorAction SilentlyContinue)) "R7_TEST_V1_CANDIDATE_ACCEPTED"
}

$scratch = Join-Path $script:R7RepoRoot "target/r7-toolchain/self-test"
[System.IO.Directory]::CreateDirectory($scratch) | Out-Null
Assert-True ((Get-R7GitLine @("rev-parse", "HEAD")) -match '^[0-9a-f]{40}$') "R7_TEST_GIT_SCALAR_LINE"
$strictParser = Join-Path $PSScriptRoot "invoke-r7-strict-json.ps1"
$validPath = Join-Path $scratch "valid.json"
[System.IO.File]::WriteAllText($validPath, "{`"a`":1}`n", [System.Text.UTF8Encoding]::new($false))
& pwsh -NoLogo -NoProfile -File $strictParser -Path $validPath | Out-Null
Assert-True ($LASTEXITCODE -eq 0) "R7_TEST_STRICT_VALID_REJECTED"
foreach ($invalid in @(
    [pscustomobject]@{name = "duplicate"; bytes = [System.Text.Encoding]::UTF8.GetBytes("{`"a`":1,`"a`":2}")},
    [pscustomobject]@{name = "trailing"; bytes = [System.Text.Encoding]::UTF8.GetBytes("{`"a`":1,}")},
    [pscustomobject]@{name = "invalid-utf8"; bytes = [byte[]]@(0xff, 0xfe, 0xfd)},
    [pscustomobject]@{name = "unsafe-integer"; bytes = [System.Text.Encoding]::UTF8.GetBytes("{`"n`":9007199254740992}")},
    [pscustomobject]@{name = "isolated-surrogate"; bytes = [System.Text.Encoding]::UTF8.GetBytes("{`"s`":`"\uD800`"}")},
    [pscustomobject]@{name = "overflow-float"; bytes = [System.Text.Encoding]::UTF8.GetBytes("{`"n`":1e400}")}
)) {
    $path = Join-Path $scratch "$($invalid.name).json"
    [System.IO.File]::WriteAllBytes($path, $invalid.bytes)
    Assert-Throws { & pwsh -NoLogo -NoProfile -File $strictParser -Path $path 2>$null | Out-Null; if ($LASTEXITCODE -ne 0) { throw "rejected" } } "R7_TEST_STRICT_NEGATIVE_ACCEPTED name=$($invalid.name)"
}
$canonicalPath = Join-Path $scratch "canonical-array.json"
[System.IO.File]::WriteAllText($canonicalPath, "{`"z`":1,`"a`":[1]}", [System.Text.UTF8Encoding]::new($false))
$canonical = & pwsh -NoLogo -NoProfile -File $strictParser -Path $canonicalPath -EmitCanonical
Assert-True (($canonical -join "") -ceq '{"a":[1],"z":1}') "R7_TEST_CANONICAL_ARRAY_OR_ORDER"
$coreCanonical = ConvertTo-R7CanonicalJson ([pscustomobject][ordered]@{z = 1; a = @(1)})
Assert-True ($coreCanonical -ceq '{"a":[1],"z":1}') "R7_TEST_CORE_CANONICAL_ARRAY_OR_ORDER"

$artifactSchema = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/candidate-artifact-content-v2.schema.json"
$manifestSchema = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/taskspace-candidate-manifest-v2.schema.json"
$evaluationSchema = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/continuous-action-evaluation-v1.schema.json"
$completionEvidenceSchema = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/continuous-action-completion-evidence-v1.schema.json"
[void](Read-R7StrictJson $artifactSchema)
[void](Read-R7StrictJson $manifestSchema)
[void](Read-R7StrictJson $evaluationSchema)
[void](Read-R7StrictJson $completionEvidenceSchema)
$closurePath = Join-Path $scratch "entry-closure.json"
& cargo run --locked -q -p codex-tools --bin r7_carrier_entry_closure --manifest-path (Join-Path $script:R7RepoRoot "third_party/codex-cli/codex-rs/Cargo.toml") -- --repo-root $script:R7RepoRoot --output $closurePath
if ($LASTEXITCODE -ne 0) { throw "R7_TEST_CLOSURE_GENERATION_FAILED" }
$closure = Read-R7StrictJson $closurePath $artifactSchema
& cargo run --locked -q -p codex-tools --bin r7_carrier_entry_closure --manifest-path (Join-Path $script:R7RepoRoot "third_party/codex-cli/codex-rs/Cargo.toml") -- --repo-root $script:R7RepoRoot --output $closurePath --check
if ($LASTEXITCODE -ne 0) { throw "R7_TEST_CLOSURE_EXACT_CHECK_FAILED" }
$fixtures = New-R7ExecutableArtifactFixtures $closure
$fixtures.continuous_action_evaluation = Read-R7StrictJson (Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/continuous-action-evaluation-v1.json") $artifactSchema
[void](Read-R7StrictJson (Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/continuous-action-evaluation-v1.json") $evaluationSchema)
Assert-R7GeneratedArtifactSemantics $fixtures (Join-Path $scratch "executable-semantics")
foreach ($role in $script:R7ArtifactNames.Keys) {
    $path = Join-Path $scratch $script:R7ArtifactNames[$role]
    Write-R7JsonFile $path $fixtures[$role]
    $body = Read-R7StrictJson $path $artifactSchema
    Assert-True ([string]$body.artifact_role -ceq $role) "R7_TEST_ARTIFACT_ROLE_MISMATCH role=$role"
}
Test-CandidateSchema $manifestSchema

$evaluation = $fixtures.continuous_action_evaluation
$digestObject = $evaluation | ConvertTo-Json -Depth 100 | ConvertFrom-Json -Depth 100
$digestObject.psobject.Properties.Remove("contract_digest")
Assert-True ((Get-R7Sha256Text ((ConvertTo-R7CanonicalJson $digestObject) + "`n")) -ceq [string]$evaluation.contract_digest) "R7_TEST_EVALUATION_DIGEST"
$ownership = Read-R7StrictJson (Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/r7-phase-ownership-v1.json")
$domains = @($ownership.domains)
Assert-True (($domains | ForEach-Object domain | Sort-Object -Unique).Count -eq $domains.Count) "R7_TEST_PHASE_OWNER_DUPLICATE"
foreach ($alias in @($ownership.forbidden_parallel_owners)) { Assert-True (@($domains | Where-Object owner_phase -eq $alias).Count -eq 0) "R7_TEST_PHASE_ALIAS_OWNER" }

$scripts = @(
    "invoke-r7-strict-json.ps1", "r7-v2-toolchain-core.ps1", "r7-v2-history.ps1", "r7-v2-promotion.ps1",
    "r7-v2-git-transaction.ps1", "r7-v2-artifact-fixtures.ps1", "test-r7-continuous-action-candidate-set.ps1",
    "evaluate-r7-continuous-action-runset.ps1", "test-r7-continuous-action-evaluator.ps1", "test-r7-continuous-action-integration.ps1",
    "new-r7-continuous-action-candidate.ps1",
    "test-r7-continuous-action-candidate.ps1", "set-r7-continuous-action-candidate-status.ps1",
    "invoke-r7-continuous-action-completion.ps1", "verify-r7-continuous-action-completion.ps1",
    "test-r7-continuous-action-toolchain.ps1"
)
foreach ($scriptName in $scripts) {
    $errors = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot $scriptName), [ref]$null, [ref]$errors)
    Assert-True ($errors.Count -eq 0) "R7_TEST_SCRIPT_SYNTAX name=$scriptName"
}
$libraryScripts = @("lib/r7-strict-json.ps1", "lib/r7-continuous-action-evaluator.ps1")
foreach ($relative in $libraryScripts) {
    $errors = $null
    [void][System.Management.Automation.Language.Parser]::ParseFile((Join-Path $PSScriptRoot $relative), [ref]$null, [ref]$errors)
    Assert-True ($errors.Count -eq 0) "R7_TEST_SCRIPT_SYNTAX name=$relative"
}
& pwsh -NoLogo -NoProfile -File (Join-Path $PSScriptRoot "test-r7-continuous-action-evaluator.ps1") | Out-Null
if ($LASTEXITCODE -ne 0) { throw "R7_TEST_EVALUATOR_SELFTEST_FAILED" }
$workflow = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $script:R7RepoRoot ".github/workflows/r7-continuous-action-completion.yml")
foreach ($marker in @("actions/checkout@d23441a48e516b6c34aea4fa41551a30e30af803", "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a", "git show", "completion_launcher", "RequiredCheckRunAttempt", "GITHUB_WORKFLOW_SHA")) { Assert-True $workflow.Contains($marker, [System.StringComparison]::Ordinal) "R7_TEST_WORKFLOW_MARKER marker=$marker" }
if ($Mode -eq "PreAnchor") {
    Assert-True (-not (Test-Path -LiteralPath (Join-Path $script:R7RepoRoot $script:R7ToolchainAnchorPath))) "R7_TEST_PREANCHOR_UNEXPECTED_ANCHOR"
} else {
    [void](Assert-R7ToolchainWorktree)
}
$result = [pscustomobject][ordered]@{schema_version = 1; test = "r7_continuous_action_v2_toolchain"; mode = $Mode; passed = $true; artifact_roles = $script:R7ArtifactNames.Count; closure_entries = @($closure.entries).Count; strict_negative_cases = 6; evaluator_negative_cases = 5; git_scalar_cases = 1; scripts_parsed = $scripts.Count + $libraryScripts.Count}
$resultPath = Join-Path $scratch "toolchain-test-result.json"
Write-R7JsonFile $resultPath $result
Write-Output ($result | ConvertTo-Json -Compress)
