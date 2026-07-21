param(
    [Parameter(Mandatory = $true)]
    [string]$RepoRoot,
    [Parameter(Mandatory = $true)]
    [string]$TargetCommit,
    [Parameter(Mandatory = $true)]
    [string]$ToolchainAddCommit,
    [Parameter(Mandatory = $true)]
    [string]$RequiredCheckRunId,
    [string]$RequiredCheckName = "r7-continuous-action-completion",
    [string]$ExportRoot = "",
    [string]$AttestationPath = ""
)

$ErrorActionPreference = "Stop"
$repo = (Resolve-Path -LiteralPath $RepoRoot).Path
$anchorPath = "benchmarks/taskspace/r7/continuous-action-v2-toolchain-anchor-v1.json"
$requiredRoles = @(
    "artifact_schema", "candidate_generator", "candidate_manifest_schema", "candidate_verifier",
    "closure_generator_entry", "closure_generator_main", "closure_generator_sources",
    "completion_launcher", "completion_verifier",
    "evaluation_contract", "phase_ownership", "required_check_workflow", "strict_parser",
    "toolchain_core", "toolchain_test", "transition_command", "tools_cargo_lock", "tools_cargo_manifest"
)

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
if (($roles -join "`n") -cne (($requiredRoles | Sort-Object) -join "`n")) { throw "R7_BOOTSTRAP_ROLE_SET" }

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
    $exported = Join-Path $ExportRoot ([System.IO.Path]::GetFileName($path))
    if (Test-Path -LiteralPath $exported) { throw "R7_BOOTSTRAP_EXPORT_NAME_COLLISION path=$path" }
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
$env:R7_STRICT_PARSER_PATH = $byRole.strict_parser
$env:R7_CANDIDATE_SCHEMA_PATH = $byRole.candidate_manifest_schema
$env:R7_ARTIFACT_SCHEMA_PATH = $byRole.artifact_schema
& pwsh -NoLogo -NoProfile -File $byRole.completion_verifier -TargetCommit $TargetCommit -ToolchainAddCommit $ToolchainAddCommit -RequiredCheckRunId $RequiredCheckRunId -RequiredCheckName $RequiredCheckName -ExportManifestPath $manifestPath -AttestationPath $AttestationPath
if ($LASTEXITCODE -ne 0) { throw "R7_BOOTSTRAP_COMPLETION_VERIFIER_FAILED" }
