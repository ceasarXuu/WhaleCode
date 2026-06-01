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
$instruction = Join-Path $taskRoot "instruction.md"
if (-not (Test-Path -LiteralPath $instruction)) { throw "DeepSWE instruction.md not found: $instruction" }
$fixtureSource = Join-Path $taskRoot "environment"
if (-not (Test-Path -LiteralPath $fixtureSource)) { throw "DeepSWE environment directory not found: $fixtureSource" }
if ([string]::IsNullOrWhiteSpace($SourceVersion)) { throw "DeepSWE SourceVersion must pin the external source revision." }
$testsDir = Join-Path $taskRoot "tests"
$testScript = Join-Path $testsDir "test.sh"
if (-not (Test-Path -LiteralPath $testScript)) { throw "DeepSWE tests/test.sh not found: $testScript" }
$generatedDir = New-TaskspaceExternalDir (Join-Path $OutputRoot "_adapter-generated")
$validatorSource = New-TaskspaceExternalDir (Join-Path $generatedDir "deepswe-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-validator-source")
Copy-Item -LiteralPath $testScript -Destination (Join-Path $validatorSource "test.sh") -Force
$testPatch = Join-Path $testsDir "test.patch"
if (Test-Path -LiteralPath $testPatch) { Copy-Item -LiteralPath $testPatch -Destination (Join-Path $validatorSource "test.patch") -Force }
$originalValidatorSha = Get-TaskspaceExternalTreeSha256 $validatorSource
$validator = Join-Path $generatedDir "deepswe-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-validator.ps1"
$validatorLines = @(
    '$ErrorActionPreference = "Stop"',
    '$validatorRoot = Join-Path $PSScriptRoot "external-validator-source"',
    '$testPatch = Join-Path $validatorRoot "test.patch"',
    'if (Test-Path -LiteralPath $testPatch) { git apply --check $testPatch; git apply $testPatch }',
    '$testScript = Join-Path $validatorRoot "test.sh"',
    'if (-not (Test-Path -LiteralPath $testScript)) { throw "DeepSWE test.sh not found: $testScript" }',
    'if (-not (Get-Command bash -ErrorAction SilentlyContinue)) { throw "bash is required for DeepSWE validation" }',
    '& bash $testScript',
    'exit $LASTEXITCODE'
)
$validatorLines | Set-Content -LiteralPath $validator -Encoding UTF8
$scenarioId = "deepswe__$($SampleId -replace '[^A-Za-z0-9_.-]', '_')"
$scenarioDir = Join-Path $OutputRoot $scenarioId
New-TaskspaceExternalScenario $scenarioDir $scenarioId "deepswe" "whale-taskspace-e3-deepswe-v1" $instruction $fixtureSource $validator $validatorSource $originalValidatorSha $SampleId $SourceVersion "https://github.com/datacurve-ai/deep-swe" "external-benchmark-license-see-source" "pointer_only_no_solution_or_hidden_tests" "DeepSWE long-horizon software engineering task subset"
