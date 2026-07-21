param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRoot,
    [Parameter(Mandatory = $true)]
    [string]$TargetCommit,
    [Parameter(Mandatory = $true)]
    [string]$ToolchainAddCommit,
    [Parameter(Mandatory = $true)]
    [string]$RequiredCheckRunId,
    [Parameter(Mandatory = $true)]
    [string]$RequiredCheckRunAttempt,
    [Parameter(Mandatory = $true)]
    [string]$Repository,
    [Parameter(Mandatory = $true)]
    [string]$WorkflowRef,
    [Parameter(Mandatory = $true)]
    [string]$WorkflowSha,
    [Parameter(Mandatory = $true)]
    [string]$EventName,
    [Parameter(Mandatory = $true)]
    [string]$GitSha,
    [Parameter(Mandatory = $true)]
    [string]$GitRef,
    [string]$RequiredCheckName = "r7-continuous-action-completion",
    [string]$ExportRoot = "",
    [string]$AttestationPath = ""
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path -LiteralPath $RepoRoot).Path
$anchorPath = "benchmarks/taskspace/r7/continuous-action-v2-toolchain-anchor-v1.json"
$requiredArtifacts = [ordered]@{
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
    closure_generator_sources = "third_party/codex-cli/codex-rs/tools/src/bin/r7_carrier_entry_closure/sources.rs"
    completion_evidence_schema = "benchmarks/taskspace/r7/continuous-action-completion-evidence-v1.schema.json"
    completion_launcher = "scripts/taskspace-benchmark/invoke-r7-continuous-action-completion.ps1"
    completion_verifier = "scripts/taskspace-benchmark/verify-r7-continuous-action-completion.ps1"
    evaluation_contract = "benchmarks/taskspace/r7/continuous-action-evaluation-v1.json"
    evaluation_launcher = "scripts/taskspace-benchmark/evaluate-r7-continuous-action-runset.ps1"
    evaluation_library = "scripts/taskspace-benchmark/lib/r7-continuous-action-evaluator.ps1"
    evaluation_result_schema = "benchmarks/taskspace/r7/continuous-action-evaluation-result-v1.schema.json"
    evaluation_schema = "benchmarks/taskspace/r7/continuous-action-evaluation-v1.schema.json"
    evaluation_test = "scripts/taskspace-benchmark/test-r7-continuous-action-evaluator.ps1"
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

function Invoke-Git {
    param([string[]]$Arguments)
    $output = & git -C $repo @Arguments 2>&1
    if ($LASTEXITCODE -ne 0) { throw "R7_BOOTSTRAP_GIT_FAILED args=$($Arguments -join ' ') detail=$($output -join "`n")" }
    @($output)
}

function Get-GitLine {
    param([string[]]$Arguments)
    $lines = @(Invoke-Git $Arguments)
    if ($lines.Count -ne 1) { throw "R7_BOOTSTRAP_EXPECTED_ONE_LINE args=$($Arguments -join ' ') count=$($lines.Count)" }
    ([string]$lines[0]).Trim()
}

function Get-GitBytes {
    param([string]$Commit, [string]$Path)
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = "git"
    foreach ($argument in @("-C", $repo, "show", "${Commit}:$Path")) { $startInfo.ArgumentList.Add($argument) }
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void]$process.Start()
    $errorTask = $process.StandardError.ReadToEndAsync()
    $stream = [System.IO.MemoryStream]::new()
    $process.StandardOutput.BaseStream.CopyTo($stream)
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw "R7_BOOTSTRAP_BLOB_MISSING commit=$Commit path=$Path detail=$($errorTask.Result)" }
    $stream.ToArray()
}

function Get-Sha256 {
    param([byte[]]$Bytes)
    [System.BitConverter]::ToString(
        [System.Security.Cryptography.SHA256]::Create().ComputeHash($Bytes)
    ).Replace("-", "").ToLowerInvariant()
}

function Read-StrictAnchor {
    param([byte[]]$Bytes)
    $text = [System.Text.UTF8Encoding]::new($false, $true).GetString($Bytes)
    $options = [System.Text.Json.JsonDocumentOptions]::new()
    $options.CommentHandling = [System.Text.Json.JsonCommentHandling]::Disallow
    $options.AllowTrailingCommas = $false
    $document = [System.Text.Json.JsonDocument]::Parse($text, $options)
    $document.Dispose()
    $settings = [Newtonsoft.Json.Linq.JsonLoadSettings]::new()
    $settings.DuplicatePropertyNameHandling = [Newtonsoft.Json.Linq.DuplicatePropertyNameHandling]::Error
    [void][Newtonsoft.Json.Linq.JToken]::Parse($text, $settings)
    $text | ConvertFrom-Json -Depth 50
}

if ($TargetCommit -notmatch '^[0-9a-f]{40}$') { throw "R7_BOOTSTRAP_TARGET_INVALID" }
if ($ToolchainAddCommit -notmatch '^[0-9a-f]{40}$') { throw "R7_BOOTSTRAP_TOOLCHAIN_COMMIT_INVALID" }
$head = Get-GitLine @("rev-parse", "HEAD")
if ($head -cne $TargetCommit) { throw "R7_BOOTSTRAP_CHECKOUT_DRIFT expected=$TargetCommit actual=$head" }
$history = @(Invoke-Git @("log", "--first-parent", "--reverse", "--format=%H", $TargetCommit, "--", $anchorPath))
if ($history.Count -ne 1 -or [string]$history[0] -cne $ToolchainAddCommit) { throw "R7_BOOTSTRAP_ANCHOR_HISTORY_INVALID" }
$parent = Get-GitLine @("rev-parse", "$ToolchainAddCommit^1")
$anchorBytes = Get-GitBytes $ToolchainAddCommit $anchorPath
$anchor = Read-StrictAnchor $anchorBytes
if ([string]$anchor.anchor_kind -cne "continuous_action_v2_toolchain") { throw "R7_BOOTSTRAP_ANCHOR_KIND" }
if ([string]$anchor.anchored_parent_commit -cne $parent) { throw "R7_BOOTSTRAP_ANCHOR_PARENT" }
$roles = @($anchor.artifacts | ForEach-Object { [string]$_.role } | Sort-Object)
if (($roles -join "`n") -cne ((@($requiredArtifacts.Keys) | Sort-Object) -join "`n")) { throw "R7_BOOTSTRAP_ROLE_SET" }
foreach ($entry in $requiredArtifacts.GetEnumerator()) {
    $matches = @($anchor.artifacts | Where-Object { [string]$_.role -ceq [string]$entry.Key })
    if ($matches.Count -ne 1 -or [string]$matches[0].path -cne [string]$entry.Value) {
        throw "R7_BOOTSTRAP_ROLE_PATH role=$($entry.Key) expected=$($entry.Value)"
    }
}

if ([string]::IsNullOrWhiteSpace($ExportRoot)) { $ExportRoot = Join-Path $repo "target/r7-toolchain/pinned-export-$TargetCommit-$RequiredCheckRunId" }
if ([string]::IsNullOrWhiteSpace($AttestationPath)) { $AttestationPath = Join-Path $repo "target/r7-toolchain/completion-attestation-$TargetCommit.json" }
[System.IO.Directory]::CreateDirectory($ExportRoot) | Out-Null
$exports = [System.Collections.Generic.List[object]]::new()
foreach ($artifact in @($anchor.artifacts | Sort-Object role)) {
    $path = [string]$artifact.path
    $treeEntry = (Invoke-Git @("ls-tree", $parent, "--", $path)) -join "`n"
    if (-not $treeEntry.StartsWith("100644 blob ", [System.StringComparison]::Ordinal)) { throw "R7_BOOTSTRAP_MODE role=$($artifact.role)" }
    $bytes = Get-GitBytes $parent $path
    $hash = Get-Sha256 $bytes
    if ($hash -cne [string]$artifact.sha256) { throw "R7_BOOTSTRAP_HASH role=$($artifact.role)" }
    $exported = Join-Path $ExportRoot $path
    [System.IO.Directory]::CreateDirectory((Split-Path $exported -Parent)) | Out-Null
    if (Test-Path -LiteralPath $exported) { throw "R7_BOOTSTRAP_EXPORT_COLLISION path=$path" }
    [System.IO.File]::WriteAllBytes($exported, $bytes)
    $exports.Add([pscustomobject][ordered]@{role = [string]$artifact.role; source_path = $path; sha256 = $hash; git_mode = "100644"; exported_path = $exported})
}
$exportManifest = [pscustomobject][ordered]@{
    schema_version = 1
    target_commit = $TargetCommit
    toolchain_add_commit = $ToolchainAddCommit
    toolchain_parent_commit = $parent
    anchor_sha256 = Get-Sha256 $anchorBytes
    artifacts = $exports.ToArray()
}
$manifestPath = Join-Path $ExportRoot "export-manifest.json"
[System.IO.File]::WriteAllText($manifestPath, "$(ConvertTo-Json $exportManifest -Depth 20)`n", [System.Text.UTF8Encoding]::new($false))
$byRole = @{}
foreach ($entry in $exports) { $byRole[[string]$entry.role] = [string]$entry.exported_path }
$env:R7_REPO_ROOT = $repo
$env:R7_ANCHOR_SCHEMA_PATH = $byRole.anchor_schema
$env:R7_STRICT_PARSER_PATH = $byRole.strict_parser
$env:R7_CANDIDATE_SCHEMA_PATH = $byRole.candidate_manifest_schema
$env:R7_ARTIFACT_SCHEMA_PATH = $byRole.artifact_schema
$env:R7_EVALUATION_SCHEMA_PATH = $byRole.evaluation_schema
$env:R7_RUN_SET_SCHEMA_PATH = $byRole.raw_run_set_schema
$env:R7_EVALUATION_RESULT_SCHEMA_PATH = $byRole.evaluation_result_schema
$env:R7_COMPLETION_EVIDENCE_SCHEMA_PATH = $byRole.completion_evidence_schema
& pwsh -NoLogo -NoProfile -File $byRole.completion_verifier -TargetCommit $TargetCommit -ToolchainAddCommit $ToolchainAddCommit -RequiredCheckRunId $RequiredCheckRunId -RequiredCheckRunAttempt $RequiredCheckRunAttempt -RequiredCheckName $RequiredCheckName -Repository $Repository -WorkflowRef $WorkflowRef -WorkflowSha $WorkflowSha -EventName $EventName -GitSha $GitSha -GitRef $GitRef -ExportManifestPath $manifestPath -AttestationPath $AttestationPath
if ($LASTEXITCODE -ne 0) { throw "R7_BOOTSTRAP_COMPLETION_VERIFIER_FAILED" }
