param(
    [Parameter(Mandatory = $true)][string]$TaskDir,
    [Parameter(Mandatory = $true)][string]$OutputRoot,
    [string]$SampleId = "",
    [string]$SourceVersion = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "external-benchmark-common.ps1")

$taskRoot = (Resolve-Path -LiteralPath $TaskDir).Path
if ([string]::IsNullOrWhiteSpace($SampleId)) { $SampleId = Split-Path -Leaf $taskRoot }
$instructionCandidates = @("instruction.md", "prompt.md", "task.md", "README.md") |
    ForEach-Object { Join-Path $taskRoot $_ } |
    Where-Object { Test-Path -LiteralPath $_ }
if (@($instructionCandidates).Count -eq 0) { throw "Terminal-Bench instruction file not found in: $taskRoot" }
$fixtureSource = Join-Path $taskRoot "environment"
if (-not (Test-Path -LiteralPath $fixtureSource)) { throw "Terminal-Bench environment directory not found: $fixtureSource" }
if ([string]::IsNullOrWhiteSpace($SourceVersion)) { throw "Terminal-Bench SourceVersion must pin the external source revision." }
$validatorCandidates = @("verify.sh", "test.sh", "run-tests.sh") |
    ForEach-Object { Join-Path $taskRoot $_ } |
    Where-Object { Test-Path -LiteralPath $_ }
if (@($validatorCandidates).Count -eq 0) { throw "Terminal-Bench validator script not found in: $taskRoot" }
$validatorSource = @($validatorCandidates)[0]
$generatedDir = New-TaskspaceExternalDir (Join-Path $OutputRoot "_adapter-generated")
$validatorSourceDir = New-TaskspaceExternalDir (Join-Path $generatedDir "terminal-bench-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-validator-source")
Copy-Item -LiteralPath $validatorSource -Destination (Join-Path $validatorSourceDir "verify.sh") -Force
$originalValidatorSha = Get-TaskspaceExternalTreeSha256 $validatorSourceDir
$validator = Join-Path $generatedDir "terminal-bench-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-validator.ps1"
$validatorLines = @(
    '$ErrorActionPreference = "Stop"',
    '$script = Join-Path $PSScriptRoot "external-validator-source\verify.sh"',
    'if (-not (Get-Command bash -ErrorAction SilentlyContinue)) { throw "bash is required for Terminal-Bench validation" }',
    '& bash $script',
    'exit $LASTEXITCODE'
)
$validatorLines | Set-Content -LiteralPath $validator -Encoding UTF8
$scenarioId = "terminal_bench__$($SampleId -replace '[^A-Za-z0-9_.-]', '_')"
$scenarioDir = Join-Path $OutputRoot $scenarioId
New-TaskspaceExternalScenario $scenarioDir $scenarioId "terminal-bench" "whale-taskspace-e3-terminal-bench-v1" @($instructionCandidates)[0] $fixtureSource $validator $validatorSourceDir $originalValidatorSha $SampleId $SourceVersion "https://github.com/laude-institute/terminal-bench" "external-benchmark-license-see-source" "pointer_only_no_solution_or_hidden_tests" "Terminal-Bench coding/file/debug/data-processing subset"
