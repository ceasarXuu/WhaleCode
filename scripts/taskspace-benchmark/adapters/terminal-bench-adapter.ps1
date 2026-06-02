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
    '$script:TaskspaceDockerBackend = ""',
    '$script:LastDockerExitCode = 0',
    'function Get-DockerBackend {',
    '    if (-not [string]::IsNullOrWhiteSpace($script:TaskspaceDockerBackend)) { return $script:TaskspaceDockerBackend }',
    '    $requested = if ($env:TASKSPACE_DOCKER_BACKEND) { $env:TASKSPACE_DOCKER_BACKEND.ToLowerInvariant() } else { "" }',
    '    if ($requested -and $requested -notin @("wsl", "native")) { throw "Unsupported TASKSPACE_DOCKER_BACKEND: $requested" }',
    '    $distro = if ($env:TASKSPACE_DOCKER_WSL_DISTRO) { $env:TASKSPACE_DOCKER_WSL_DISTRO } else { "whale-docker" }',
    '    if ($requested -ne "native" -and (Get-Command wsl -ErrorAction SilentlyContinue)) {',
    '        $probe = & wsl -d $distro -- docker version --format "{{.Server.Version}}" 2>$null',
    '        if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($probe)) {',
    '            $script:TaskspaceDockerBackend = "wsl"',
    '            return $script:TaskspaceDockerBackend',
    '        }',
    '        if ($requested -eq "wsl") { throw "Requested WSL Docker backend is unavailable: $distro" }',
    '    }',
    '    if (Get-Command docker -ErrorAction SilentlyContinue) {',
    '        $script:TaskspaceDockerBackend = "native"',
    '        return $script:TaskspaceDockerBackend',
    '    }',
    '    throw "docker command is required for Terminal-Bench validation"',
    '}',
    'function ConvertTo-DockerPath {',
    '    param([Parameter(Mandatory = $true)][string]$Path, [Parameter(Mandatory = $true)][string]$Backend)',
    '    $resolved = (Resolve-Path -LiteralPath $Path).Path',
    '    if ($Backend -eq "native") { return $resolved }',
    '    if ($resolved -match "^([A-Za-z]):\\(.*)$") {',
    '        $drive = $matches[1].ToLowerInvariant()',
    '        $tail = $matches[2].Replace("\", "/")',
    '        return "/mnt/$drive/$tail"',
    '    }',
    '    $distro = if ($env:TASKSPACE_DOCKER_WSL_DISTRO) { $env:TASKSPACE_DOCKER_WSL_DISTRO } else { "whale-docker" }',
    '    if (Get-Command wsl -ErrorAction SilentlyContinue) {',
    '        $converted = & wsl -d $distro -- wslpath -a $resolved 2>$null',
    '        if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($converted)) { return $converted.Trim() }',
    '    }',
    '    return $resolved',
    '}',
    'function Invoke-Docker {',
    '    param([Parameter(Mandatory = $true)][string[]]$Arguments)',
    '    $backend = Get-DockerBackend',
    '    $distro = if ($env:TASKSPACE_DOCKER_WSL_DISTRO) { $env:TASKSPACE_DOCKER_WSL_DISTRO } else { "whale-docker" }',
    '    if ($backend -eq "wsl") {',
    '        & wsl -d $distro -- docker @Arguments',
    '        $script:LastDockerExitCode = $LASTEXITCODE',
    '        return',
    '    }',
    '    & docker @Arguments',
    '    $script:LastDockerExitCode = $LASTEXITCODE',
    '}',
    '$scenarioRoot = $PSScriptRoot',
    '$repoDir = (Resolve-Path -LiteralPath (Get-Location)).Path',
    '$fixtureDir = Join-Path $scenarioRoot "fixture"',
    '$validatorSource = Join-Path $scenarioRoot "external-validator-source"',
    '$testScript = Join-Path $validatorSource "verify.sh"',
    '$testDir = Join-Path $validatorSource "tests"',
    'if (-not (Test-Path -LiteralPath $testScript)) { throw "Terminal-Bench run-tests script not found: $testScript" }',
    'if (-not (Test-Path -LiteralPath $testDir)) { throw "Terminal-Bench tests directory not found: $testDir" }',
    '$sha = [System.Security.Cryptography.SHA256]::Create()',
    '$repoHashBytes = $sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($repoDir))',
    '$repoHash = (([System.BitConverter]::ToString($repoHashBytes)) -replace "-", "").ToLowerInvariant().Substring(0, 16)',
    '$image = "whale-taskspace-terminal-bench:$repoHash"',
    '$containerName = "whale-tbench-$repoHash"',
    '$entryScript = Join-Path $scenarioRoot "terminal-bench-validator-entry.sh"',
    '$entryContent = @''',
    'set -euo pipefail',
    'export TEST_DIR=/tbench-validator/tests',
    'cd /app',
    'echo validator_runtime=terminal_bench_docker_app',
    'echo container_workdir=$(pwd)',
    'echo test_dir=$TEST_DIR',
    'test "$(pwd)" = "/app"',
    'test -d "$TEST_DIR"',
    'test -f /tbench-validator/verify.sh',
    'bash /tbench-validator/verify.sh',
    '''@',
    '$entryContent = $entryContent -replace "`r`n", "`n"',
    '[System.IO.File]::WriteAllText($entryScript, $entryContent, [System.Text.Encoding]::ASCII)',
    '$backend = Get-DockerBackend',
    '$fixtureDockerPath = ConvertTo-DockerPath $fixtureDir $backend',
    '$repoDockerPath = ConvertTo-DockerPath $repoDir $backend',
    '$validatorDockerPath = ConvertTo-DockerPath $validatorSource $backend',
    '$entryDockerPath = ConvertTo-DockerPath $entryScript $backend',
    'Write-Host "validator_runtime_probe=terminal_bench_docker_wrapper"',
    'Write-Host "docker_backend=$backend"',
    'Write-Host "docker_image=$image"',
    'Write-Host "docker_container=$containerName"',
    'Write-Host "repo_mount=$repoDockerPath"',
    'Write-Host "validator_mount=/tbench-validator"',
    '$networkArgs = if ($backend -eq "wsl") { @("--network", "host") } else { @("--add-host", "host.docker.internal:host-gateway") }',
    '$proxyArgs = @()',
    'foreach ($proxyName in @("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy")) {',
    '    $proxyValue = [Environment]::GetEnvironmentVariable($proxyName)',
    '    if ([string]::IsNullOrWhiteSpace($proxyValue)) { continue }',
    '    if ($backend -ne "wsl") {',
    '        $proxyValue = $proxyValue -replace "://127\.0\.0\.1:", "://host.docker.internal:"',
    '        $proxyValue = $proxyValue -replace "://localhost:", "://host.docker.internal:"',
    '    }',
    '    $proxyArgs += @("-e", "$proxyName=$proxyValue")',
    '}',
    'Write-Host "proxy_env_count=$($proxyArgs.Count / 2)"',
    '$exitCode = 0',
    'try {',
    '    Invoke-Docker -Arguments @("build", "--pull", "-t", $image, $fixtureDockerPath)',
    '    if ($script:LastDockerExitCode -ne 0) { exit $script:LastDockerExitCode }',
    '    $runArgs = @(',
    '        "run", "--name", $containerName,',
    '        "--label", "whale.taskspace.terminal_bench=true",',
    '        "--label", "whale.taskspace.repo_hash=$repoHash"',
    '    ) + $proxyArgs + @(',
    '        "-v", "${repoDockerPath}:/app",',
    '        "-v", "${validatorDockerPath}:/tbench-validator:ro",',
    '        "-v", "${entryDockerPath}:/tbench-entry.sh:ro",',
    '        "-w", "/app",',
    '        "-e", "TEST_DIR=/tbench-validator/tests",',
    '        $image, "bash", "/tbench-entry.sh"',
    '    )',
    '    $runArgs = @($runArgs[0..6]) + $networkArgs + @($runArgs[7..($runArgs.Count - 1)])',
    '    Invoke-Docker -Arguments $runArgs',
    '    $exitCode = $script:LastDockerExitCode',
    '} finally {',
    '    Invoke-Docker -Arguments @("rm", "-f", $containerName)',
    '    Invoke-Docker -Arguments @("rmi", "-f", $image)',
    '}',
    'exit $exitCode'
)
$validatorLines | Set-Content -LiteralPath $validator -Encoding UTF8
$scenarioId = "terminal_bench__$($SampleId -replace '[^A-Za-z0-9_.-]', '_')"
$scenarioDir = Join-Path $OutputRoot $scenarioId
$validatorFidelity = [ordered]@{
    official_runner_or_equivalent = $false
    docker_runtime = $true
    container_workdir = "/app"
    validator_runtime = "terminal_bench_docker_app"
    agent_cannot_read_validator_source = $false
    e3_eligible = $false
    downgrade_reason = "Docker /app post-hoc validation is real, but official Terminal-Bench harness equivalence and validator-source isolation are not fully proven."
}
$adapterMetadata = [ordered]@{
    instruction_extraction_mode = $instructionMode
    instruction_line = $instructionLine
    fixture_mode = $fixtureMode
    generated_fixture_allowlist = @("Dockerfile", "docker-compose.yaml", "task.yaml")
    solution_visible_to_agent = $false
    engineering_smoke_only = $false
    validator_mount = "/tbench-validator:ro"
    repo_mount = "/app"
    agent_execution_app_alias = $true
    e3_downgraded_until_runtime_fidelity_proven = $true
}
New-TaskspaceExternalScenario $scenarioDir $scenarioId "terminal-bench" "whale-taskspace-e3-terminal-bench-v1" $promptSource $fixtureSource $validator $validatorSourceDir $originalValidatorSha $SampleId $SourceVersion "https://github.com/laude-institute/terminal-bench" "external-benchmark-license-see-source" "pointer_only_no_solution_or_hidden_tests" "Terminal-Bench coding/file/debug/data-processing subset" $validatorFidelity $adapterMetadata
