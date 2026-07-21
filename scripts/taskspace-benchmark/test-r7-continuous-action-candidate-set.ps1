param(
    [string]$TargetCommit = "HEAD",
    [switch]$SkipWorktree
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "r7-v2-toolchain-core.ps1")

function Assert-Equal {
    param($Actual, $Expected, [string]$Code)
    if ($Actual -cne $Expected) { throw "$Code expected=$Expected actual=$Actual" }
}

function Write-R7CommitScratch {
    param([string]$Commit, [string]$Path, [string]$Label)
    $scratchRoot = Join-Path $script:R7RepoRoot "target/r7-toolchain/strict-inputs"
    [System.IO.Directory]::CreateDirectory($scratchRoot) | Out-Null
    $safe = $Label -replace '[^A-Za-z0-9._-]', '-'
    $scratch = Join-Path $scratchRoot "$Commit-$safe.json"
    [System.IO.File]::WriteAllBytes($scratch, (Get-R7GitBlobBytes $Commit $Path))
    $scratch
}

function Read-R7CommitJson {
    param([string]$Commit, [string]$Path, [string]$SchemaPath, [string]$Label)
    Read-R7StrictJson (Write-R7CommitScratch $Commit $Path $Label) $SchemaPath
}

function Get-R7CandidateManifestPaths {
    param([string]$Commit)
    $paths = @(Invoke-R7Git @("ls-tree", "-r", "--name-only", $Commit, "--", $script:R7CandidateRoot))
    @($paths | Where-Object {
        $_ -match "^$([regex]::Escape($script:R7CandidateRoot))/[0-9a-f]{64}/manifest\.json$"
    } | Sort-Object)
}

function Assert-R7CandidateNamespace {
    param([string]$Commit, [string]$CandidateId)
    $prefix = "$script:R7CandidateRoot/$CandidateId/"
    $actual = @(Invoke-R7Git @("ls-tree", "-r", "--name-only", $Commit, "--", "$script:R7CandidateRoot/$CandidateId") |
        Where-Object { [string]$_ -like "$prefix*" } |
        ForEach-Object { ([string]$_).Substring($prefix.Length) } |
        Sort-Object)
    $expected = @("creation-evidence.json", "manifest.json")
    foreach ($name in $script:R7ArtifactNames.Values) { $expected += [string]$name }
    $expected = @($expected | Sort-Object)
    Assert-Equal ($actual -join "`n") ($expected -join "`n") "R7_CANDIDATE_NAMESPACE_DRIFT candidate=$CandidateId"
    foreach ($name in $expected) {
        [void](Assert-R7OrdinaryBlob $Commit "$prefix$name" "R7_CANDIDATE_FILE_MODE")
    }
}

function Assert-R7CandidateScript {
    param([string]$Commit, [string]$CandidateId)
    $scriptPath = Join-Path $PSScriptRoot "test-r7-continuous-action-candidate.ps1"
    $output = & pwsh -NoLogo -NoProfile -File $scriptPath -CandidateId $CandidateId -TargetCommit $Commit 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "R7_CANDIDATE_SET_MEMBER_FAILED candidate=$CandidateId detail=$($output -join "`n")"
    }
}

function Get-R7StatusCount {
    param([object[]]$Items, [string]$Status)
    @($Items | Where-Object { [string]$_.manifest.candidate_status -eq $Status }).Count
}

function Assert-R7ActivationHistory {
    param([string]$BaselineAdd, [string]$Target, [string]$ManifestSchema, [string]$ArtifactSchema)
    $events = @(Invoke-R7Git @(
        "log", "--first-parent", "--reverse", "--format=%H", "$BaselineAdd..$Target", "--",
        $script:R7AuthorityPath, $script:R7ProductionPath
    ))
    foreach ($commit in $events) {
        $parent = Get-R7GitLine @("rev-parse", "$commit^1")
        $changed = @(Invoke-R7Git @("diff-tree", "--no-commit-id", "--name-only", "-r", $commit) | Sort-Object -Unique)
        $manifests = @($changed | Where-Object {
            $_ -match "^$([regex]::Escape($script:R7CandidateRoot))/[0-9a-f]{64}/manifest\.json$"
        })
        if ($manifests.Count -ne 1) {
            throw "R7_ACTIVATION_EVENT_MANIFEST_COUNT commit=$commit count=$($manifests.Count)"
        }
        $path = [string]$manifests[0]
        $before = Read-R7CommitJson $parent $path $ManifestSchema "activation-before-$commit"
        $after = Read-R7CommitJson $commit $path $ManifestSchema "activation-after-$commit"
        $transition = "$([string]$before.candidate_status)->$([string]$after.candidate_status)"
        if ($transition -eq "promotion_pending->promoted") {
            $expected = @($after.promotion.changed_paths | Sort-Object -Unique)
        } elseif ($transition -eq "promoted->reverted") {
            $rollbackPath = [string]$after.artifact_hashes.rollback_manifest.path
            $rollback = Read-R7CommitJson $commit $rollbackPath $ArtifactSchema "activation-rollback-$commit"
            $runtime = @($rollback.changed_path_inventory | Where-Object {
                [string]$_.rollback_action -ne "preserve"
            } | ForEach-Object { [string]$_.path })
            $expected = @($script:R7AuthorityPath, $script:R7ProductionPath, $path) + $runtime |
                Sort-Object -Unique
        } else {
            throw "R7_ACTIVATION_EVENT_TRANSITION_INVALID commit=$commit transition=$transition"
        }
        Assert-Equal ($changed -join "`n") ($expected -join "`n") "R7_ACTIVATION_EVENT_PATHS commit=$commit"
    }
}

$target = Get-R7GitLine @("rev-parse", $TargetCommit)
if (-not $SkipWorktree) { [void](Assert-R7ToolchainWorktree $target) }
$baseline = Get-R7BaselineAnchor $target

$manifestSchema = if ([string]::IsNullOrWhiteSpace($env:R7_CANDIDATE_SCHEMA_PATH)) {
    Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/taskspace-candidate-manifest-v2.schema.json"
} else {
    $env:R7_CANDIDATE_SCHEMA_PATH
}
$artifactSchema = if ([string]::IsNullOrWhiteSpace($env:R7_ARTIFACT_SCHEMA_PATH)) {
    Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/candidate-artifact-content-v2.schema.json"
} else {
    $env:R7_ARTIFACT_SCHEMA_PATH
}

$allCandidatePaths = @(Invoke-R7Git @("ls-tree", "-r", "--name-only", $target, "--", $script:R7CandidateRoot))
$namespaceIds = @($allCandidatePaths | ForEach-Object {
    $match = [regex]::Match([string]$_, "^$([regex]::Escape($script:R7CandidateRoot))/(?<id>[^/]+)/")
    if (-not $match.Success) { throw "R7_CANDIDATE_NAMESPACE_PATH_INVALID path=$_" }
    if ($match.Groups["id"].Value -notmatch '^[0-9a-f]{64}$') {
        throw "R7_CANDIDATE_NAMESPACE_ID_INVALID path=$_"
    }
    $match.Groups["id"].Value
} | Sort-Object -Unique)
$manifestPaths = @(Get-R7CandidateManifestPaths $target)
$candidateDirs = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
$candidateIds = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
$candidates = [System.Collections.Generic.List[object]]::new()

foreach ($path in $manifestPaths) {
    $match = [regex]::Match($path, "^$([regex]::Escape($script:R7CandidateRoot))/(?<id>[0-9a-f]{64})/manifest\.json$")
    if (-not $match.Success) { throw "R7_CANDIDATE_MANIFEST_PATH_INVALID path=$path" }
    $dirId = $match.Groups["id"].Value
    if (-not $candidateDirs.Add($dirId)) { throw "R7_CANDIDATE_DIR_DUPLICATE candidate=$dirId" }
    Assert-R7CandidateNamespace $target $dirId
    Assert-R7CandidateScript $target $dirId
    $manifest = Read-R7CommitJson $target $path $manifestSchema "set-manifest-$dirId"
    $manifestId = [string]$manifest.candidate_id
    if (-not $candidateIds.Add($manifestId)) { throw "R7_CANDIDATE_ID_DUPLICATE candidate=$manifestId" }
    Assert-Equal $manifestId $dirId "R7_CANDIDATE_DIR_ID_DRIFT"
    $candidates.Add([pscustomobject][ordered]@{id = $dirId; path = $path; manifest = $manifest})
}
Assert-Equal $candidateDirs.Count $namespaceIds.Count "R7_CANDIDATE_ORPHAN_NAMESPACE"
foreach ($namespaceId in $namespaceIds) {
    if (-not $candidateDirs.Contains($namespaceId)) {
        throw "R7_CANDIDATE_MANIFEST_MISSING candidate=$namespaceId"
    }
}
Assert-R7ActivationHistory $baseline.add_commit $target $manifestSchema $artifactSchema

$pendingCount = Get-R7StatusCount $candidates.ToArray() "promotion_pending"
$promoted = @($candidates.ToArray() | Where-Object { [string]$_.manifest.candidate_status -eq "promoted" })
if ($pendingCount -gt 1) { throw "R7_PROMOTION_PENDING_NOT_UNIQUE count=$pendingCount" }
if ($promoted.Count -gt 1) { throw "R7_PROMOTED_NOT_UNIQUE count=$($promoted.Count)" }
if ($pendingCount -gt 0 -and $promoted.Count -gt 0) { throw "R7_PROMOTION_STATE_CONFLICT" }

$baselineAuthority = (Get-R7GitBlobText $baseline.parent_commit $script:R7AuthorityPath) | ConvertFrom-Json -Depth 100
$baselineProduction = (Get-R7GitBlobText $baseline.parent_commit $script:R7ProductionPath) | ConvertFrom-Json -Depth 100
$actualAuthority = (Get-R7GitBlobText $target $script:R7AuthorityPath) | ConvertFrom-Json -Depth 100
$actualProduction = (Get-R7GitBlobText $target $script:R7ProductionPath) | ConvertFrom-Json -Depth 100
[void](Assert-R7OrdinaryBlob $target $script:R7AuthorityPath "R7_AUTHORITY_FILE_MODE")
[void](Assert-R7OrdinaryBlob $target $script:R7ProductionPath "R7_PRODUCTION_FILE_MODE")

if ($promoted.Count -eq 1) {
    $winner = $promoted[0]
    Assert-Equal ([string]$actualProduction.promoted_candidate_id) ([string]$winner.id) "R7_PRODUCTION_PROMOTED_POINTER_DRIFT"
    $hashes = [pscustomobject][ordered]@{}
    foreach ($role in $script:R7ArtifactNames.Keys) {
        $hashes | Add-Member -NotePropertyName $role -NotePropertyValue ([string]$winner.manifest.artifact_hashes.$role.sha256)
    }
    $references = New-R7ArtifactReferences ([string]$winner.id) $hashes
    $expected = New-R7ExpectedPromotionContract $baselineAuthority $baselineProduction ([string]$winner.id) $references
    Assert-Equal (ConvertTo-R7CanonicalJson $actualAuthority) (ConvertTo-R7CanonicalJson $expected.expected_authority) "R7_PROMOTED_SET_AUTHORITY_DRIFT"
    Assert-Equal (ConvertTo-R7CanonicalJson $actualProduction) (ConvertTo-R7CanonicalJson $expected.expected_production) "R7_PROMOTED_SET_PRODUCTION_DRIFT"
} else {
    if ($null -ne $actualProduction.psobject.Properties["promoted_candidate_id"]) {
        throw "R7_PRODUCTION_PROMOTED_POINTER_UNEXPECTED"
    }
    Assert-Equal (ConvertTo-R7CanonicalJson $actualAuthority) (ConvertTo-R7CanonicalJson $baselineAuthority) "R7_UNPROMOTED_SET_AUTHORITY_DRIFT"
    Assert-Equal (ConvertTo-R7CanonicalJson $actualProduction) (ConvertTo-R7CanonicalJson $baselineProduction) "R7_UNPROMOTED_SET_PRODUCTION_DRIFT"
}

[pscustomobject][ordered]@{
    valid = $true
    target_commit = $target
    candidate_count = $candidates.Count
    promotion_pending_count = $pendingCount
    promoted_count = $promoted.Count
} | ConvertTo-Json -Compress
