param(
    [Parameter(Mandatory = $true)][string]$OutputDir,
    [Parameter(Mandatory = $true)][string]$TaskListHash,
    [Parameter(Mandatory = $true)][string]$SourceVersion,
    [Parameter(Mandatory = $true)][string]$ProfileHash,
    [Parameter(Mandatory = $true)][string]$SampleSetId,
    [string[]]$TestOutputPath = @(),
    [string]$ApprovalSource = "",
    [switch]$ApproveFullE3
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$outputRoot = [System.IO.Path]::GetFullPath($OutputDir)
New-Item -ItemType Directory -Force -Path $outputRoot | Out-Null

function Get-FileSha256OrEmpty {
    param([string]$Path)
    if ([string]::IsNullOrWhiteSpace($Path) -or -not (Test-Path -LiteralPath $Path -PathType Leaf)) { return "" }
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

$head = ((& git -C $repoRoot rev-parse HEAD 2>$null) | Select-Object -First 1).Trim()
if ([string]::IsNullOrWhiteSpace($head)) { throw "Cannot resolve current git HEAD." }

$testOutputs = @($TestOutputPath | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) } | ForEach-Object {
        $full = [System.IO.Path]::GetFullPath([string]$_)
        if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { throw "TestOutputPath not found: $full" }
        [pscustomobject]@{
            path = $full
            sha256 = Get-FileSha256OrEmpty $full
        }
    })

if ($testOutputs.Count -eq 0) {
    throw "At least one TestOutputPath is required for a code-complete marker."
}

$codeCompletePath = Join-Path $outputRoot "v005-code-complete.json"
[pscustomobject]@{
    schema_version = 1
    status = "pass"
    producer = "write-v005-markers.ps1"
    code_complete = $true
    git_commit = $head
    task_list_hash = $TaskListHash
    source_version = $SourceVersion
    profile_hash = $ProfileHash
    sample_set_id = $SampleSetId
    unfinished_p0_items = @()
    test_outputs = @($testOutputs)
    generated_at = (Get-Date).ToString("o")
} | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $codeCompletePath -Encoding UTF8

Write-Host "V005CodeComplete: $codeCompletePath"

if ($ApproveFullE3) {
    if ([string]::IsNullOrWhiteSpace($ApprovalSource)) {
        throw "ApprovalSource is required when ApproveFullE3 is set."
    }
    $approvalPath = Join-Path $outputRoot "v005-user-approval.json"
    [pscustomobject]@{
        schema_version = 1
        status = "pass"
        producer = "write-v005-markers.ps1"
        approved_command_category = "full_e3"
        approved_sample_set_id = $SampleSetId
        approval_source = $ApprovalSource
        approval_timestamp = (Get-Date).ToString("o")
        task_list_hash = $TaskListHash
        source_version = $SourceVersion
        profile_hash = $ProfileHash
        generated_at = (Get-Date).ToString("o")
    } | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $approvalPath -Encoding UTF8
    Write-Host "V005UserApproval: $approvalPath"
}
