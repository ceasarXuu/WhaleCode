function Get-R7ExecutionPlanRepositoryFile {
    param([string]$RepoRoot, [string]$RelativePath, [string]$Label)
    Assert-R7ExecutionPlan (-not [IO.Path]::IsPathRooted($RelativePath)) `
        "$Label path must be repository-relative"
    Assert-R7ExecutionPlan ($RelativePath -notmatch '(^|[\\/])\.\.([\\/]|$)') `
        "$Label path escapes the repository"
    Assert-R7ExecutionPlan ($RelativePath -notmatch '^[a-zA-Z][a-zA-Z0-9+.-]*:') `
        "$Label path must not be a URI"
    $root = [IO.Path]::GetFullPath($RepoRoot).TrimEnd(
        [IO.Path]::DirectorySeparatorChar,
        [IO.Path]::AltDirectorySeparatorChar
    )
    $fullPath = [IO.Path]::GetFullPath((Join-Path $root $RelativePath))
    $prefix = $root + [IO.Path]::DirectorySeparatorChar
    Assert-R7ExecutionPlan (
        $fullPath.StartsWith($prefix, [StringComparison]::Ordinal)
    ) "$Label path resolves outside the repository"
    Assert-R7ExecutionPlan (Test-Path -LiteralPath $fullPath -PathType Leaf) `
        "$Label file is missing: $RelativePath"

    $cursor = $root
    foreach ($segment in $RelativePath -split '[\\/]') {
        $cursor = Join-Path $cursor $segment
        if (Test-Path -LiteralPath $cursor) {
            $item = Get-Item -Force -LiteralPath $cursor
            $isLink = ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0
            Assert-R7ExecutionPlan (-not $isLink) `
                "$Label path contains a symlink or reparse point: $RelativePath"
        }
    }
    $fullPath
}

function Get-R7ExecutionPlanEvidence {
    param(
        $Reference,
        [string]$RepoRoot,
        [string]$Label,
        [string]$ExpectedSchemaPath,
        [string]$ExpectedSchemaVersion
    )
    Assert-R7ExecutionPlan (
        [string]$Reference.schema_path -eq $ExpectedSchemaPath
    ) "$Label uses an unapproved evidence schema path"
    Assert-R7ExecutionPlan (
        [string]$Reference.schema_version -eq $ExpectedSchemaVersion
    ) "$Label uses an unapproved evidence schema version"
    $fullPath = Get-R7ExecutionPlanRepositoryFile `
        $RepoRoot ([string]$Reference.path) "$Label evidence"
    $schemaPath = Get-R7ExecutionPlanRepositoryFile `
        $RepoRoot $ExpectedSchemaPath "$Label schema"
    $raw = Get-Content -Raw -Encoding UTF8 -LiteralPath $fullPath
    Assert-R7ExecutionPlan (
        $raw | Test-Json -SchemaFile $schemaPath -ErrorAction Stop
    ) "$Label evidence does not match its authoritative schema"
    $actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $fullPath).Hash.ToLowerInvariant()
    Assert-R7ExecutionPlan ($actualHash -eq [string]$Reference.sha256) `
        "$Label evidence SHA-256 does not match: $($Reference.path)"
    $artifact = $raw | ConvertFrom-Json -Depth 100 -NoEnumerate
    Assert-R7ExecutionPlan (
        [string]$artifact.schema_version -eq [string]$Reference.schema_version
    ) "$Label evidence schema_version does not match its reference"
    Assert-R7ExecutionPlan (
        [string]$artifact.artifact_type -eq [string]$Reference.artifact_type
    ) "$Label evidence artifact_type does not match its reference"
    $artifact
}

function Assert-R7ExecutionPlanPhaseEvidence {
    param($Phase, [string]$RepoRoot)
    $reference = $Phase.evidence_artifact
    Assert-R7ExecutionPlan (
        [string]$reference.artifact_type -eq [string]$Phase.acceptance_evidence_type
    ) "$($Phase.id) evidence artifact_type does not match its acceptance contract"
    $artifact = Get-R7ExecutionPlanEvidence $reference $RepoRoot ([string]$Phase.id) `
        "benchmarks/taskspace/r7/r7-phase-evidence-v1.schema.json" `
        "r71-phase-evidence-v1"
    foreach ($record in @($artifact.records)) {
        foreach ($field in @($Phase.observability.required_fields)) {
            $property = $record.PSObject.Properties[[string]$field]
            Assert-R7ExecutionPlan ($null -ne $property -and $null -ne $property.Value) `
                "$($Phase.id) evidence record is missing required field: $field"
        }
    }
}

function Get-R7ExecutionPlanHeldOutSamples {
    param($Reference, [string]$RepoRoot, [string]$Label)
    Assert-R7ExecutionPlan (
        [string]$Reference.artifact_type -eq "held_out_sample_manifest"
    ) "$Label uses the wrong artifact_type"
    $artifact = Get-R7ExecutionPlanEvidence $Reference $RepoRoot $Label `
        "benchmarks/taskspace/r7/r7-held-out-set-v1.schema.json" `
        "r71-held-out-set-v1"
    @($artifact.sample_ids)
}
