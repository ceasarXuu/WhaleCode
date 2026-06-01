param(
    [Parameter(Mandatory = $true)][string]$TaskDir,
    [Parameter(Mandatory = $true)][string]$OutputRoot,
    [string]$SampleId = "",
    [string]$SourceVersion = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "external-benchmark-common.ps1")

function ConvertFrom-TerminalBenchYamlScalar {
    param([Parameter(Mandatory = $true)][string]$Scalar)
    $text = $Scalar.Trim()
    if (($text.StartsWith('"') -and $text.EndsWith('"')) -or ($text.StartsWith("'") -and $text.EndsWith("'"))) {
        $text = $text.Substring(1, $text.Length - 2)
    }
    $text.Replace('\"', '"').Replace("''", "'")
}

function Read-TerminalBenchInstruction {
    param([Parameter(Mandatory = $true)][string]$TaskYaml)
    $yamlLines = Get-Content -Encoding UTF8 -LiteralPath $TaskYaml
    for ($index = 0; $index -lt $yamlLines.Count; $index++) {
        $line = $yamlLines[$index]
        if ($line -notmatch '^instruction:\s*(.*)$') { continue }
        $scalar = $matches[1].Trim()
        if (-not [string]::IsNullOrWhiteSpace($scalar) -and $scalar[0] -notin @('|', '>')) {
            $inline = ConvertFrom-TerminalBenchYamlScalar $scalar
            if ([string]::IsNullOrWhiteSpace($inline)) { throw "Terminal-Bench task.yaml inline instruction is empty: $TaskYaml" }
            return [pscustomobject]@{ text = $inline; mode = "inline"; line = ($index + 1) }
        }
        $isFolded = (-not [string]::IsNullOrWhiteSpace($scalar) -and $scalar[0] -eq '>')
        $blockLines = New-Object System.Collections.Generic.List[string]
        $baseIndent = 0
        for ($blockIndex = $index + 1; $blockIndex -lt $yamlLines.Count; $blockIndex++) {
            $blockLine = $yamlLines[$blockIndex]
            if ($blockLine -match '^\S') { break }
            if ([string]::IsNullOrWhiteSpace($blockLine)) {
                $blockLines.Add("")
                continue
            }
            if ($baseIndent -eq 0 -and $blockLine -match '^(\s+)') { $baseIndent = $matches[1].Length }
            $trimCount = [Math]::Min($baseIndent, $blockLine.Length)
            $blockLines.Add($blockLine.Substring($trimCount))
        }
        if ($blockLines.Count -eq 0) { throw "Terminal-Bench task.yaml instruction block is empty: $TaskYaml" }
        if ($isFolded) {
            $folded = New-Object System.Collections.Generic.List[string]
            $paragraph = New-Object System.Collections.Generic.List[string]
            foreach ($blockLine in $blockLines) {
                if ([string]::IsNullOrWhiteSpace($blockLine)) {
                    if ($paragraph.Count -gt 0) {
                        $folded.Add(($paragraph -join " "))
                        $paragraph.Clear()
                    }
                    $folded.Add("")
                } else {
                    $paragraph.Add($blockLine.Trim())
                }
            }
            if ($paragraph.Count -gt 0) { $folded.Add(($paragraph -join " ")) }
            $text = (($folded.ToArray()) -join "`n").Trim()
            if ([string]::IsNullOrWhiteSpace($text)) { throw "Terminal-Bench task.yaml folded instruction is empty: $TaskYaml" }
            return [pscustomobject]@{ text = $text; mode = "folded"; line = ($index + 1) }
        }
        $literal = (($blockLines.ToArray()) -join "`n").Trim()
        if ([string]::IsNullOrWhiteSpace($literal)) { throw "Terminal-Bench task.yaml literal instruction is empty: $TaskYaml" }
        return [pscustomobject]@{ text = $literal; mode = "literal"; line = ($index + 1) }
    }
    throw "Terminal-Bench task.yaml instruction field not found: $TaskYaml"
}

$taskRoot = (Resolve-Path -LiteralPath $TaskDir).Path
if ([string]::IsNullOrWhiteSpace($SampleId)) { $SampleId = Split-Path -Leaf $taskRoot }
$instructionCandidates = @("instruction.md", "prompt.md", "task.md", "README.md") |
    ForEach-Object { Join-Path $taskRoot $_ } |
    Where-Object { Test-Path -LiteralPath $_ }
$taskYaml = Join-Path $taskRoot "task.yaml"
if (@($instructionCandidates).Count -eq 0 -and -not (Test-Path -LiteralPath $taskYaml)) { throw "Terminal-Bench instruction file not found in: $taskRoot" }
$fixtureSource = Join-Path $taskRoot "environment"
if ([string]::IsNullOrWhiteSpace($SourceVersion)) { throw "Terminal-Bench SourceVersion must pin the external source revision." }
$validatorCandidates = @("verify.sh", "test.sh", "run-tests.sh") |
    ForEach-Object { Join-Path $taskRoot $_ } |
    Where-Object { Test-Path -LiteralPath $_ }
if (@($validatorCandidates).Count -eq 0) { throw "Terminal-Bench validator script not found in: $taskRoot" }
$validatorSource = @($validatorCandidates)[0]
$generatedDir = New-TaskspaceExternalDir (Join-Path $OutputRoot "_adapter-generated")
$instructionMode = "file"
$instructionLine = 0
$promptSource = if (@($instructionCandidates).Count -gt 0) {
    @($instructionCandidates)[0]
} else {
    $generatedPrompt = Join-Path $generatedDir "terminal-bench-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-instruction.md"
    $promptInfo = Read-TerminalBenchInstruction $taskYaml
    $instructionMode = $promptInfo.mode
    $instructionLine = $promptInfo.line
    Set-Content -LiteralPath $generatedPrompt -Encoding UTF8 -Value $promptInfo.text
    $generatedPrompt
}
$fixtureMode = "environment"
$fixtureSource = if (Test-Path -LiteralPath $fixtureSource) {
    $fixtureSource
} else {
    $fixtureMode = "generated_public_allowlist"
    $generatedFixture = New-TaskspaceExternalDir (Join-Path $generatedDir "terminal-bench-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-fixture")
    foreach ($name in @("Dockerfile", "docker-compose.yaml", "task.yaml")) {
        $source = Join-Path $taskRoot $name
        if (Test-Path -LiteralPath $source) { Copy-Item -LiteralPath $source -Destination $generatedFixture -Force }
    }
    $generatedFixture
}
$validatorSourceDir = New-TaskspaceExternalDir (Join-Path $generatedDir "terminal-bench-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-validator-source")
Copy-Item -LiteralPath $validatorSource -Destination (Join-Path $validatorSourceDir "verify.sh") -Force
if (Test-Path -LiteralPath (Join-Path $taskRoot "tests")) {
    Copy-Item -LiteralPath (Join-Path $taskRoot "tests") -Destination (Join-Path $validatorSourceDir "tests") -Recurse -Force
}
$originalValidatorSha = Get-TaskspaceExternalTreeSha256 $validatorSourceDir
$validator = Join-Path $generatedDir "terminal-bench-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-validator.ps1"
$validatorLines = @(
    '$ErrorActionPreference = "Stop"',
    '$script = Join-Path $PSScriptRoot "external-validator-source\verify.sh"',
    '$env:TEST_DIR = Join-Path $PSScriptRoot "external-validator-source\tests"',
    'Write-Host "validator_runtime=windows_git_bash_non_docker"',
    'Write-Host "docker_available=$([bool](Get-Command docker -ErrorAction SilentlyContinue))"',
    'if (-not (Get-Command bash -ErrorAction SilentlyContinue)) { throw "bash is required for Terminal-Bench validation" }',
    '& bash -lc ''echo "bash_uname=$(uname -a 2>/dev/null)"; echo "bash_pwd=$PWD"; echo "test_dir=$TEST_DIR"''',
    '& bash $script',
    'exit $LASTEXITCODE'
)
$validatorLines | Set-Content -LiteralPath $validator -Encoding UTF8
$scenarioId = "terminal_bench__$($SampleId -replace '[^A-Za-z0-9_.-]', '_')"
$scenarioDir = Join-Path $OutputRoot $scenarioId
$validatorFidelity = [ordered]@{
    official_runner_or_equivalent = $false
    docker_runtime = $false
    container_workdir = ""
    validator_runtime = "windows_git_bash_non_docker"
    agent_cannot_read_validator_source = $false
    e3_eligible = $false
    downgrade_reason = "Terminal-Bench adapter currently uses a local PowerShell/Git Bash wrapper, not the official Docker runner, and validator source is materialized before agent execution."
}
$adapterMetadata = [ordered]@{
    instruction_extraction_mode = $instructionMode
    instruction_line = $instructionLine
    fixture_mode = $fixtureMode
    generated_fixture_allowlist = @("Dockerfile", "docker-compose.yaml", "task.yaml")
    solution_visible_to_agent = $false
    engineering_smoke_only = $true
}
New-TaskspaceExternalScenario $scenarioDir $scenarioId "terminal-bench" "whale-taskspace-e3-terminal-bench-v1" $promptSource $fixtureSource $validator $validatorSourceDir $originalValidatorSha $SampleId $SourceVersion "https://github.com/laude-institute/terminal-bench" "external-benchmark-license-see-source" "pointer_only_no_solution_or_hidden_tests" "Terminal-Bench coding/file/debug/data-processing subset" $validatorFidelity $adapterMetadata
