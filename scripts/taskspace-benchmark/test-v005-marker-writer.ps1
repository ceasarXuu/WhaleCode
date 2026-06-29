param([string]$RunRoot = "")

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
if (-not $RunRoot) { $RunRoot = Join-Path $repoRoot "target\v005-marker-writer-selftest" }
$runDir = Join-Path ([System.IO.Path]::GetFullPath($RunRoot)) (Get-Date -Format "yyyyMMdd-HHmmss-fff")
New-Item -ItemType Directory -Force -Path $runDir | Out-Null
$failures = New-Object System.Collections.Generic.List[string]
function Assert-True([bool]$Condition, [string]$Message) { if (-not $Condition) { [void]$script:failures.Add($Message) } }

$testOutputPath = Join-Path $runDir "non-agent-gates.out"
"fixture non-agent gates passed" | Set-Content -LiteralPath $testOutputPath -Encoding UTF8
$taskListHash = "task-list-marker-selftest"
$profileHash = "profile-marker-selftest"
$sourceVersion = "terminal-bench@marker-selftest"
$sampleSetId = "terminal-bench_E3-P0_3_5"

$output = & powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $PSScriptRoot "write-v005-markers.ps1") `
    -OutputDir $runDir `
    -TaskListHash $taskListHash `
    -SourceVersion $sourceVersion `
    -ProfileHash $profileHash `
    -SampleSetId $sampleSetId `
    -TestOutputPath $testOutputPath `
    -ApproveFullE3 `
    -ApprovalSource "fixture approval" 2>&1
Assert-True ($LASTEXITCODE -eq 0) "marker writer did not exit 0: $($output -join ' | ')"

$codePath = Join-Path $runDir "v005-code-complete.json"
$approvalPath = Join-Path $runDir "v005-user-approval.json"
Assert-True (Test-Path -LiteralPath $codePath -PathType Leaf) "code-complete marker missing"
Assert-True (Test-Path -LiteralPath $approvalPath -PathType Leaf) "user-approval marker missing"

$head = (& git -C $repoRoot rev-parse HEAD).Trim()
$code = Get-Content -Raw -Encoding UTF8 -LiteralPath $codePath | ConvertFrom-Json
$approval = Get-Content -Raw -Encoding UTF8 -LiteralPath $approvalPath | ConvertFrom-Json
Assert-True ([string]$code.status -eq "pass" -and [bool]$code.code_complete) "code-complete marker status invalid"
Assert-True ([string]$code.git_commit -eq $head) "code-complete marker did not bind current HEAD"
Assert-True ([string]$code.task_list_hash -eq $taskListHash -and [string]$code.profile_hash -eq $profileHash -and [string]$code.source_version -eq $sourceVersion) "code-complete marker identity mismatch"
Assert-True ([string]$code.sample_set_id -eq $sampleSetId) "code-complete marker sample set mismatch"
Assert-True (@($code.test_outputs).Count -eq 1 -and -not [string]::IsNullOrWhiteSpace([string]$code.test_outputs[0].sha256)) "code-complete test output sha missing"
Assert-True (@($code.unfinished_p0_items).Count -eq 0) "code-complete marker should not list unfinished P0 items"

Assert-True ([string]$approval.status -eq "pass") "approval marker status invalid"
Assert-True ([string]$approval.approved_command_category -eq "full_e3") "approval marker command category invalid"
Assert-True ([string]$approval.approved_sample_set_id -eq $sampleSetId) "approval marker sample set mismatch"
Assert-True (-not [string]::IsNullOrWhiteSpace([string]$approval.approval_timestamp)) "approval marker timestamp missing"

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Error $_ }
    exit 1
}
Write-Host "v005 marker writer selftest passed"
Write-Host "RunRoot: $runDir"
