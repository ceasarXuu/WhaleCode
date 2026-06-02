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

function Invoke-TerminalBenchGitQuiet {
    param(
        [Parameter(Mandatory = $true)][string]$GitRoot,
        [Parameter(Mandatory = $true)][string[]]$Arguments
    )
    $oldPreference = $ErrorActionPreference
    try {
        $ErrorActionPreference = "SilentlyContinue"
        $output = & git -C $GitRoot @Arguments 2>$null
        if ($LASTEXITCODE -ne 0) { return "" }
        return (($output | ForEach-Object { [string]$_ }) -join "`n").Trim()
    } finally {
        $ErrorActionPreference = $oldPreference
    }
}

function Get-TerminalBenchOfficialEquivalence {
    param(
        [Parameter(Mandatory = $true)][string]$TaskRoot,
        [Parameter(Mandatory = $true)][string]$SourceVersion
    )
    $required = @(
        "terminal_bench\harness\harness.py",
        "terminal_bench\terminal\docker_compose_manager.py",
        "terminal_bench\handlers\trial_handler.py",
        "terminal_bench\parsers\pytest_parser.py"
    )
    $gitRoot = ""
    if (Get-Command git -ErrorAction SilentlyContinue) {
        $root = & git -C $TaskRoot rev-parse --show-toplevel 2>$null
        if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($root)) { $gitRoot = $root.Trim() }
    }
    $sourceFiles = New-Object System.Collections.Generic.List[object]
    $allPresent = -not [string]::IsNullOrWhiteSpace($gitRoot)
    $allPinned = $allPresent
    if ($allPresent) {
        foreach ($relative in $required) {
            $path = Join-Path $gitRoot $relative
            $relativeUnix = $relative.Replace("\", "/")
            $pinnedBlob = ""
            $currentBlob = ""
            if (Get-Command git -ErrorAction SilentlyContinue) {
                $pinnedBlob = Invoke-TerminalBenchGitQuiet $gitRoot @("rev-parse", "$SourceVersion`:$relativeUnix")
            }
            if (-not (Test-Path -LiteralPath $path)) {
                $allPresent = $false
            } else {
                $currentBlob = Invoke-TerminalBenchGitQuiet $gitRoot @("hash-object", $path)
                $resolved = (Resolve-Path -LiteralPath $path).Path
                $matchesPinned = (-not [string]::IsNullOrWhiteSpace($pinnedBlob) -and $pinnedBlob -eq $currentBlob)
                if (-not $matchesPinned) { $allPinned = $false }
                $sourceFiles.Add([pscustomobject]@{
                    path = $resolved
                    relative_path = $relativeUnix
                    current_sha256 = Get-TaskspaceExternalFileSha256 $resolved
                    pinned_blob_id = $pinnedBlob
                    current_blob_id = $currentBlob
                    matches_pinned_revision = $matchesPinned
                })
            }
        }
    }
    $revisionPinned = $SourceVersion -match '^[0-9a-fA-F]{40}$'
    $taskRelative = ""
    $taskDirty = $true
    if ($allPresent) {
        $taskRelative = (Resolve-Path -LiteralPath $TaskRoot).Path.Substring((Resolve-Path -LiteralPath $gitRoot).Path.Length).TrimStart("\", "/").Replace("\", "/")
        $status = & git -C $gitRoot status --porcelain -- $taskRelative 2>$null
        $taskDirty = @($status | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_) }).Count -gt 0
    }
    [ordered]@{
        protocol = "terminal_bench_post_agent_tests_v1"
        source_root = $gitRoot
        source_revision = $SourceVersion
        source_revision_pinned = $revisionPinned
        source_files_present = $allPresent
        source_files_match_pinned_revision = $allPinned
        task_relative_path = $taskRelative
        task_worktree_dirty = $taskDirty
        source_files = @($sourceFiles.ToArray())
        proven = ($revisionPinned -and $allPresent -and $allPinned -and -not $taskDirty)
    }
}

function Get-TerminalBenchSensitiveFiles {
    param([Parameter(Mandatory = $true)][string]$TaskRoot)
    $files = New-Object System.Collections.Generic.List[string]
    foreach ($name in @("run-tests.sh", "verify.sh", "test.sh", "solution.sh", "solution.yaml")) {
        $path = Join-Path $TaskRoot $name
        if (Test-Path -LiteralPath $path) { $files.Add((Resolve-Path -LiteralPath $path).Path) }
    }
    $tests = Join-Path $TaskRoot "tests"
    if (Test-Path -LiteralPath $tests) {
        foreach ($file in Get-ChildItem -LiteralPath $tests -Recurse -File -Force) {
            $files.Add($file.FullName)
        }
    }
    @($files.ToArray() | Sort-Object -Unique)
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
if ($SourceVersion -match '^[0-9a-fA-F]{40}$' -and (Get-Command git -ErrorAction SilentlyContinue)) {
    $sourceHead = (& git -C $taskRoot rev-parse HEAD 2>$null)
    if ($LASTEXITCODE -eq 0 -and -not [string]::IsNullOrWhiteSpace($sourceHead)) {
        if ($sourceHead.Trim().ToLowerInvariant() -ne $SourceVersion.ToLowerInvariant()) {
            throw "Terminal-Bench SourceVersion mismatch: expected $SourceVersion but task checkout is $($sourceHead.Trim())."
        }
    }
}
$validatorCandidates = @("run-tests.sh", "verify.sh", "test.sh") |
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
$originalPromptSource = $promptSource
$adaptedPrompt = Join-Path $generatedDir "terminal-bench-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-adapted-instruction.md"
$originalPromptText = Get-Content -Raw -Encoding UTF8 -LiteralPath $originalPromptSource
$runnerNote = @"

Local runner environment note:
Treat the current working directory as the task's /app directory. If the instruction names /app/<path>, create or update <path> in the current working directory rather than creating a nested app/ directory or C:\app.
"@
Set-Content -LiteralPath $adaptedPrompt -Encoding UTF8 -Value ($originalPromptText.TrimEnd() + $runnerNote)
$promptSource = $adaptedPrompt
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
Copy-Item -LiteralPath $validatorSource -Destination (Join-Path $validatorSourceDir "run-tests.sh") -Force
if (Test-Path -LiteralPath (Join-Path $taskRoot "tests")) {
    foreach ($item in Get-ChildItem -LiteralPath (Join-Path $taskRoot "tests") -Force) {
        Copy-Item -LiteralPath $item.FullName -Destination $validatorSourceDir -Recurse -Force
    }
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
    'function Invoke-DockerOutput {',
    '    param([Parameter(Mandatory = $true)][string[]]$Arguments)',
    '    $backend = Get-DockerBackend',
    '    $distro = if ($env:TASKSPACE_DOCKER_WSL_DISTRO) { $env:TASKSPACE_DOCKER_WSL_DISTRO } else { "whale-docker" }',
    '    if ($backend -eq "wsl") {',
    '        $output = & wsl -d $distro -- docker @Arguments',
    '        $script:LastDockerExitCode = $LASTEXITCODE',
    '        return @($output)',
    '    }',
    '    $output = & docker @Arguments',
    '    $script:LastDockerExitCode = $LASTEXITCODE',
    '    @($output)',
    '}',
    '$scenarioRoot = $PSScriptRoot',
    '$repoDir = (Resolve-Path -LiteralPath (Get-Location)).Path',
    '$proofDir = if ($env:TASKSPACE_VALIDATION_ARTIFACT_DIR) { $env:TASKSPACE_VALIDATION_ARTIFACT_DIR } else { Join-Path $repoDir ".taskspace-validator-proof" }',
    'New-Item -ItemType Directory -Path $proofDir -Force | Out-Null',
    '$fixtureDir = Join-Path $scenarioRoot "fixture"',
    '$validatorSource = Join-Path $scenarioRoot "external-validator-source"',
    '$testScript = Join-Path $validatorSource "run-tests.sh"',
    'if (-not (Test-Path -LiteralPath $testScript)) { throw "Terminal-Bench run-tests script not found: $testScript" }',
    '$sha = [System.Security.Cryptography.SHA256]::Create()',
    '$repoHashBytes = $sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($repoDir))',
    '$repoHash = (([System.BitConverter]::ToString($repoHashBytes)) -replace "-", "").ToLowerInvariant().Substring(0, 16)',
    '$image = "whale-taskspace-terminal-bench:$repoHash"',
    '$containerName = "whale-tbench-$repoHash"',
    '$entryScript = Join-Path $scenarioRoot "terminal-bench-validator-entry.sh"',
    '$proofNonce = [guid]::NewGuid().ToString("N")',
    '$entryContent = @''',
    'set -euo pipefail',
    'export TEST_DIR=/tests',
    'cd /app',
    'echo validator_proof_nonce=$WHALE_TBENCH_PROOF_NONCE',
    'echo validator_runtime=terminal_bench_equivalent_docker_app',
    'echo container_workdir=$(pwd)',
    'echo test_dir=$TEST_DIR',
    'echo validator_mount=/tests',
    'echo validator_command=bash /tests/run-tests.sh',
    'test "$(pwd)" = "/app"',
    'test -d "$TEST_DIR"',
    'test -f /tests/run-tests.sh',
    'if touch /tests/.whale-write-test 2>/tmp/whale-validator-ro.err; then echo validator_mount_readonly=false; rm -f /tests/.whale-write-test; exit 81; else echo validator_mount_readonly=true; fi',
    'bash /tests/run-tests.sh',
    '''@',
    '$entryContent = $entryContent -replace "`r`n", "`n"',
    '[System.IO.File]::WriteAllText($entryScript, $entryContent, [System.Text.Encoding]::ASCII)',
    '$wrapperSha = (Get-FileHash -Algorithm SHA256 -LiteralPath $PSCommandPath).Hash.ToLowerInvariant()',
    '$entrySha = (Get-FileHash -Algorithm SHA256 -LiteralPath $entryScript).Hash.ToLowerInvariant()',
    '$backend = Get-DockerBackend',
    '$fixtureDockerPath = ConvertTo-DockerPath $fixtureDir $backend',
    '$repoDockerPath = ConvertTo-DockerPath $repoDir $backend',
    '$validatorDockerPath = ConvertTo-DockerPath $validatorSource $backend',
    '$entryDockerPath = ConvertTo-DockerPath $entryScript $backend',
    'Write-Host "validator_runtime_probe=terminal_bench_equivalent_wrapper"',
    'Write-Host "validator_proof_nonce=$proofNonce"',
    'Write-Host "validator_wrapper_sha256=$wrapperSha"',
    'Write-Host "validator_entry_sha256=$entrySha"',
    'Write-Host "docker_backend=$backend"',
    'Write-Host "docker_image=$image"',
    'Write-Host "docker_container=$containerName"',
    'Write-Host "repo_mount=$repoDockerPath"',
    'Write-Host "validator_mount=/tests"',
    '$runtimeManifestPath = Join-Path $proofDir "terminal-bench-runtime-manifest.json"',
    '$inspectPath = Join-Path $proofDir "terminal-bench-docker-inspect.json"',
    '@{',
    '    proof_nonce = $proofNonce',
    '    wrapper_path = $PSCommandPath',
    '    wrapper_sha256 = $wrapperSha',
    '    entry_script_path = $entryScript',
    '    entry_sha256 = $entrySha',
    '    docker_backend = $backend',
    '    image = $image',
    '    container_name = $containerName',
    '    repo_mount = $repoDockerPath',
    '    validator_mount = $validatorDockerPath',
    '    validator_container_path = "/tests"',
    '    entry_mount = $entryDockerPath',
    '    validator_command = "bash /tests/run-tests.sh"',
    '} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $runtimeManifestPath -Encoding UTF8',
    'Write-Host "validator_runtime_manifest_path=$runtimeManifestPath"',
    'Invoke-Docker -Arguments @("version", "--format", "{{.Server.Version}}")',
    'Write-Host "docker_version_exit=$($script:LastDockerExitCode)"',
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
    '        "-v", "${validatorDockerPath}:/tests:ro",',
    '        "-v", "${entryDockerPath}:/tbench-entry.sh:ro",',
    '        "-w", "/app",',
    '        "-e", "TEST_DIR=/tests",',
    '        "-e", "WHALE_TBENCH_PROOF_NONCE=$proofNonce",',
    '        $image, "bash", "/tbench-entry.sh"',
    '    )',
    '    $runArgs = @($runArgs[0..6]) + $networkArgs + @($runArgs[7..($runArgs.Count - 1)])',
    '    Invoke-Docker -Arguments $runArgs',
    '    $exitCode = $script:LastDockerExitCode',
    '    $inspectOutput = Invoke-DockerOutput -Arguments @("inspect", $containerName)',
    '    $inspectOutput | Set-Content -LiteralPath $inspectPath -Encoding UTF8',
    '    Write-Host "docker_inspect_path=$inspectPath"',
    '    Write-Host "docker_inspect_available=$($script:LastDockerExitCode -eq 0)"',
    '} finally {',
    '    Invoke-Docker -Arguments @("rm", "-f", $containerName)',
    '    Write-Host "validator_cleanup_container_exit=$($script:LastDockerExitCode)"',
    '    Invoke-Docker -Arguments @("rmi", "-f", $image)',
    '    Write-Host "validator_cleanup_image_exit=$($script:LastDockerExitCode)"',
    '}',
    'exit $exitCode'
)
$validatorLines | Set-Content -LiteralPath $validator -Encoding UTF8
$scenarioId = "terminal_bench__$($SampleId -replace '[^A-Za-z0-9_.-]', '_')"
$scenarioDir = Join-Path $OutputRoot $scenarioId
$officialEquivalence = Get-TerminalBenchOfficialEquivalence $taskRoot $SourceVersion
$officialProven = [bool]$officialEquivalence["proven"]
$sensitiveFiles = @(Get-TerminalBenchSensitiveFiles $taskRoot)
$validatorFidelity = [ordered]@{
    official_runner_or_equivalent = $officialProven
    docker_runtime = $true
    container_workdir = "/app"
    validator_runtime = "terminal_bench_equivalent_docker_app"
    agent_cannot_read_validator_source = $true
    e3_eligible = $officialProven
    downgrade_reason = if ($officialProven) { "" } else { "Terminal-Bench official protocol source hashes were not available from a pinned checkout." }
}
$adapterMetadata = [ordered]@{
    instruction_extraction_mode = $instructionMode
    instruction_line = $instructionLine
    fixture_mode = $fixtureMode
    generated_fixture_allowlist = @("Dockerfile", "docker-compose.yaml", "task.yaml")
    prompt_adaptation = "current_working_directory_is_terminal_bench_app"
    original_instruction_sha256 = (Get-TaskspaceExternalFileSha256 $originalPromptSource)
    solution_visible_to_agent = $false
    engineering_smoke_only = $false
    validator_mount = "/tests:ro"
    repo_mount = "/app"
    agent_execution_app_alias = $true
    official_equivalence = $officialEquivalence
    sensitive_source_files = $sensitiveFiles
    e3_downgraded_until_runtime_fidelity_proven = -not $officialProven
}
New-TaskspaceExternalScenario $scenarioDir $scenarioId "terminal-bench" "whale-taskspace-e3-terminal-bench-v1" $promptSource $fixtureSource $validator $validatorSourceDir $originalValidatorSha $SampleId $SourceVersion "https://github.com/laude-institute/terminal-bench" "external-benchmark-license-see-source" "pointer_only_no_solution_or_hidden_tests" "Terminal-Bench coding/file/debug/data-processing subset" $validatorFidelity $adapterMetadata
