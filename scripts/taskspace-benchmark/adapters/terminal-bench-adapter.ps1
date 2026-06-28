param(
    [Parameter(Mandatory = $true)][string]$TaskDir,
    [Parameter(Mandatory = $true)][string]$OutputRoot,
    [string]$SampleId = "",
    [string]$SourceVersion = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "external-benchmark-common.ps1")
. (Join-Path $PSScriptRoot "terminal-bench-uv-cache.ps1")
. (Join-Path $PSScriptRoot "terminal-bench-remote-assets.ps1")
. (Join-Path $PSScriptRoot "terminal-bench-equivalence.ps1")

$OutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)

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

function Test-TerminalBenchPublicFixtureRelativePath {
    param([Parameter(Mandatory = $true)][string]$RelativePath)
    $segments = @($RelativePath.Replace("\", "/") -split "/" | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    foreach ($segment in $segments) {
        $lower = $segment.ToLowerInvariant()
        if ($lower -in @("tests", "run-tests.sh", "verify.sh", "test.sh", "solution.sh", "solution.yaml")) { return $false }
        if (Test-TaskspaceExternalLeakyName $lower) { return $false }
    }
    return $true
}

function Get-TerminalBenchDockerfileBaseImageProof {
    param([Parameter(Mandatory = $true)][string]$DockerfilePath)
    if (-not (Test-Path -LiteralPath $DockerfilePath)) { return [pscustomobject]@{ from_images = @(); digest_pinned = $false } }
    $fromImages = New-Object System.Collections.Generic.List[string]
    foreach ($line in Get-Content -Encoding UTF8 -LiteralPath $DockerfilePath) {
        if ($line -match '^\s*FROM\s+([^\s]+)') { $fromImages.Add($matches[1]) }
    }
    $images = @($fromImages.ToArray())
    [pscustomobject]@{
        from_images = $images
        digest_pinned = ($images.Count -gt 0 -and @($images | Where-Object { $_ -notmatch '@sha256:[0-9a-fA-F]{64}$' }).Count -eq 0)
    }
}

$taskRoot = (Resolve-Path -LiteralPath $TaskDir).Path
Repair-TaskspaceExternalStaleDenyTreeForCurrentUser $taskRoot | Out-Null
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
$uvCache = New-TerminalBenchUvCache $OutputRoot
if (-not [bool]$uvCache.enabled) { throw "Terminal-Bench uv dependency cache could not be materialized; refusing to generate a partial external validator scenario." }
$remoteAssets = @(Get-TerminalBenchRemoteAssets $taskRoot $SampleId $OutputRoot $SourceVersion $uvCache)
$remoteAssetsE3Eligible = ($remoteAssets.Count -eq 0)
$uvCacheLiteral = "'" + $uvCache.root.Replace("'", "''") + "'"
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
$upstreamPromptText = $originalPromptText.TrimEnd()
$runnerNote = @"

Local runner environment note:
Treat the current working directory as the task's /app directory. If the instruction names /app/<path>, create or update <path> in the current working directory rather than creating a nested app/ directory or C:\app.
"@
$adaptedPromptText = $upstreamPromptText + $runnerNote
Set-Content -LiteralPath $adaptedPrompt -Encoding UTF8 -Value $adaptedPromptText
$promptSource = $adaptedPrompt
$fixtureMode = "environment"
$generatedFixtureAllowlist = @()
$fixtureSource = if (Test-Path -LiteralPath $fixtureSource) {
    $generatedFixtureAllowlist = @("environment")
    $fixtureSource
} else {
    $fixtureMode = "generated_public_allowlist"
    $generatedFixture = New-TaskspaceExternalDir (Join-Path $generatedDir "terminal-bench-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-fixture")
    $copiedFiles = New-Object System.Collections.Generic.List[string]
    foreach ($file in Get-ChildItem -LiteralPath $taskRoot -Recurse -File -Force) {
        $relative = $file.FullName.Substring($taskRoot.Length).TrimStart("\", "/").Replace("\", "/")
        if (-not (Test-TerminalBenchPublicFixtureRelativePath $relative)) { continue }
        $dest = Join-Path $generatedFixture ($relative.Replace("/", [System.IO.Path]::DirectorySeparatorChar))
        New-Item -ItemType Directory -Path (Split-Path -Parent $dest) -Force | Out-Null
        Copy-Item -LiteralPath $file.FullName -Destination $dest -Force
        $copiedFiles.Add($relative)
    }
    $generatedFixtureAllowlist = @($copiedFiles.ToArray())
    $generatedFixture
}
$remoteInjection = Initialize-TerminalBenchRemoteAssetInjection $fixtureSource $generatedDir $remoteAssets $SampleId
$fixtureSource = [string]$remoteInjection.fixture_source
$remoteAssets = @($remoteInjection.remote_assets)
$remoteAssetsE3Eligible = @($remoteAssets | Where-Object { $_.required_for_e3 -and -not [bool]$_.equivalence_proven }).Count -eq 0
$validatorSourceDir = New-TaskspaceExternalDir (Join-Path $generatedDir "terminal-bench-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-validator-source")
Copy-TaskspaceExternalShellScript $validatorSource (Join-Path $validatorSourceDir "run-tests.sh")
if (Test-Path -LiteralPath (Join-Path $taskRoot "tests")) {
    foreach ($item in Get-ChildItem -LiteralPath (Join-Path $taskRoot "tests") -Force) {
        if (-not $item.PSIsContainer -and [System.IO.Path]::GetExtension($item.FullName).ToLowerInvariant() -eq ".sh") {
            Copy-TaskspaceExternalShellScript $item.FullName (Join-Path $validatorSourceDir $item.Name)
        } else {
            Copy-Item -LiteralPath $item.FullName -Destination $validatorSourceDir -Recurse -Force
        }
    }
}
$originalValidatorSha = Get-TaskspaceExternalTreeSha256 $validatorSourceDir
$fixtureTreeSha = Get-TaskspaceExternalTreeSha256 $fixtureSource
$dockerfilePath = Join-Path $fixtureSource "Dockerfile"
$dockerfileSha = if (Test-Path -LiteralPath $dockerfilePath) { Get-TaskspaceExternalFileSha256 $dockerfilePath } else { "" }
$baseImageProof = Get-TerminalBenchDockerfileBaseImageProof $dockerfilePath
$adapterSha = Get-TaskspaceExternalFileSha256 $PSCommandPath
$uvInstallSha = if (Test-Path -LiteralPath (Join-Path $uvCache.root "install.sh")) { Get-TaskspaceExternalFileSha256 (Join-Path $uvCache.root "install.sh") } else { "" }
$uvArchiveSha = if (Test-Path -LiteralPath (Join-Path $uvCache.root "uv-x86_64-unknown-linux-gnu.tar.gz")) { Get-TaskspaceExternalFileSha256 (Join-Path $uvCache.root "uv-x86_64-unknown-linux-gnu.tar.gz") } else { "" }
$dockerCacheSchemaVersion = "terminal-bench-image-cache-v2"
$dockerPlatform = "default"
$dockerNetworkMode = "default"
$dockerBuildEnvironmentMode = "host-proxy-forwarded"
$cacheMaterial = @(
    "schema=$dockerCacheSchemaVersion",
    "source=$SourceVersion",
    "fixture=$fixtureTreeSha",
    "dockerfile=$dockerfileSha",
    "validator_source=$originalValidatorSha",
    "adapter=$adapterSha",
    "uv_install=$uvInstallSha",
    "uv_archive=$uvArchiveSha",
    "platform=$dockerPlatform",
    "network=$dockerNetworkMode",
    "build_environment=$dockerBuildEnvironmentMode",
    "from_images=$((@($baseImageProof.from_images) | Sort-Object) -join ',')"
) -join "`n"
$cacheKeyBytes = [System.Security.Cryptography.SHA256]::Create().ComputeHash([System.Text.Encoding]::UTF8.GetBytes($cacheMaterial))
$dockerCacheKey = (([System.BitConverter]::ToString($cacheKeyBytes)) -replace "-", "").ToLowerInvariant().Substring(0, 32)
$dockerCacheKeyLiteral = "'" + $dockerCacheKey.Replace("'", "''") + "'"
$dockerCacheSchemaLiteral = "'" + $dockerCacheSchemaVersion.Replace("'", "''") + "'"
$fixtureTreeShaLiteral = "'" + $fixtureTreeSha.Replace("'", "''") + "'"
$dockerfileShaLiteral = "'" + $dockerfileSha.Replace("'", "''") + "'"
$validatorSourceShaLiteral = "'" + $originalValidatorSha.Replace("'", "''") + "'"
$adapterShaLiteral = "'" + $adapterSha.Replace("'", "''") + "'"
$uvInstallShaLiteral = "'" + $uvInstallSha.Replace("'", "''") + "'"
$uvArchiveShaLiteral = "'" + $uvArchiveSha.Replace("'", "''") + "'"
$dockerPlatformLiteral = "'" + $dockerPlatform.Replace("'", "''") + "'"
$dockerNetworkModeLiteral = "'" + $dockerNetworkMode.Replace("'", "''") + "'"
$dockerBuildEnvironmentModeLiteral = "'" + $dockerBuildEnvironmentMode.Replace("'", "''") + "'"
$cacheEligibleLiteral = if ([bool]$baseImageProof.digest_pinned) { '$true' } else { '$false' }
$baseImagesLiteral = "@(" + ((@($baseImageProof.from_images) | ForEach-Object { "'" + ([string]$_).Replace("'", "''") + "'" }) -join ", ") + ")"
$validator = Join-Path $generatedDir "terminal-bench-$($SampleId -replace '[^A-Za-z0-9_.-]', '_')-validator.ps1"
$validatorLines = @(
    'param([switch]$ProbeOnly, [switch]$ProbeDocker)',
    '$ErrorActionPreference = "Stop"',
    '$script:TaskspaceDockerBackend = ""',
    '$script:LastDockerExitCode = 0',
    '$script:TaskspaceDockerProbeFailure = ""',
    'function Invoke-DockerBackendProbe {',
    '    param([Parameter(Mandatory = $true)][string]$Command, [Parameter(Mandatory = $true)][string[]]$Arguments, [int]$TimeoutSeconds = 20)',
    '    $job = Start-Job -ScriptBlock {',
    '        param([string]$InnerCommand, [string[]]$InnerArguments)',
    '        try {',
    '            $output = & $InnerCommand @InnerArguments 2>&1',
    '            $exitCode = if ($null -eq $LASTEXITCODE) { 0 } else { [int]$LASTEXITCODE }',
    '            [pscustomobject]@{ exit_code = $exitCode; output = @($output | ForEach-Object { [string]$_ }); error = "" }',
    '        } catch {',
    '            [pscustomobject]@{ exit_code = 1; output = @(); error = [string]$_.Exception.Message }',
    '        }',
    '    } -ArgumentList $Command, $Arguments',
    '    if (-not (Wait-Job -Job $job -Timeout $TimeoutSeconds)) {',
    '        Stop-Job -Job $job -ErrorAction SilentlyContinue | Out-Null',
    '        Remove-Job -Job $job -Force -ErrorAction SilentlyContinue',
    '        return [pscustomobject]@{ exit_code = 124; output = @(); error = "probe timed out after $TimeoutSeconds seconds"; timed_out = $true }',
    '    }',
    '    $result = Receive-Job -Job $job',
    '    Remove-Job -Job $job -Force -ErrorAction SilentlyContinue',
    '    if ($null -eq $result) { return [pscustomobject]@{ exit_code = 1; output = @(); error = "probe produced no result"; timed_out = $false } }',
    '    $result | Select-Object -First 1',
    '}',
    'function Format-DockerBackendProbeFailure {',
    '    param([Parameter(Mandatory = $true)]$Probe)',
    '    $text = (@($Probe.output) + @($Probe.error)) -join " "',
    '    $text = ($text -replace "\s+", " ").Trim()',
    '    if ($text.Length -gt 320) { $text = $text.Substring(0, 320) }',
    '    if ([string]::IsNullOrWhiteSpace($text)) { $text = "no output" }',
    '    "exit=$($Probe.exit_code); $text"',
    '}',
    'function Test-DockerCommandIsWslWrapper {',
    '    param($CommandInfo)',
    '    if ($null -eq $CommandInfo -or [string]::IsNullOrWhiteSpace([string]$CommandInfo.Source)) { return $false }',
    '    if ([System.IO.Path]::GetExtension([string]$CommandInfo.Source).ToLowerInvariant() -notin @(".cmd", ".bat")) { return $false }',
    '    try { return ((Get-Content -Raw -Encoding UTF8 -LiteralPath ([string]$CommandInfo.Source)) -match "(?i)\bwsl(\.exe)?\b.*\bdocker\b") } catch { return $false }',
    '}',
    'function Get-DockerBackend {',
    '    if (-not [string]::IsNullOrWhiteSpace($script:TaskspaceDockerBackend)) { return $script:TaskspaceDockerBackend }',
    '    $requested = if ($env:TASKSPACE_DOCKER_BACKEND) { $env:TASKSPACE_DOCKER_BACKEND.ToLowerInvariant() } else { "" }',
    '    if ($requested -and $requested -notin @("wsl", "native")) { throw "Unsupported TASKSPACE_DOCKER_BACKEND: $requested" }',
    '    $distro = if ($env:TASKSPACE_DOCKER_WSL_DISTRO) { $env:TASKSPACE_DOCKER_WSL_DISTRO } else { "whale-docker" }',
    '    if ($requested -ne "native" -and (Get-Command wsl -ErrorAction SilentlyContinue)) {',
    '        $probe = Invoke-DockerBackendProbe "wsl" @("-d", $distro, "--", "docker", "version", "--format", "{{.Server.Version}}")',
    '        if ([int]$probe.exit_code -eq 0 -and -not [string]::IsNullOrWhiteSpace((@($probe.output) -join "").Trim())) {',
    '            $script:TaskspaceDockerBackend = "wsl"',
    '            return $script:TaskspaceDockerBackend',
    '        }',
    '        $script:TaskspaceDockerProbeFailure = "WSL[$distro] " + (Format-DockerBackendProbeFailure $probe)',
    '        if ($requested -eq "wsl") { throw "Requested WSL Docker backend is unavailable: $distro; $script:TaskspaceDockerProbeFailure" }',
    '    }',
    '    $dockerCommand = Get-Command docker -ErrorAction SilentlyContinue',
    '    if ($requested -ne "wsl" -and $dockerCommand) {',
    '        if (Test-DockerCommandIsWslWrapper $dockerCommand) {',
    '            $nativeFailure = "native[$($dockerCommand.Source)] is a WSL docker wrapper; use WSL backend or a native Docker CLI"',
    '            $script:TaskspaceDockerProbeFailure = (($script:TaskspaceDockerProbeFailure, $nativeFailure) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join "; "',
    '            if ($requested -eq "native") { throw "Requested native Docker backend is unavailable: $nativeFailure" }',
    '        } else {',
    '        $probe = Invoke-DockerBackendProbe $dockerCommand.Source @("version", "--format", "{{.Server.Version}}")',
    '        if ([int]$probe.exit_code -eq 0 -and -not [string]::IsNullOrWhiteSpace((@($probe.output) -join "").Trim())) {',
    '            $script:TaskspaceDockerBackend = "native"',
    '            return $script:TaskspaceDockerBackend',
    '        }',
    '        $nativeFailure = "native[$($dockerCommand.Source)] " + (Format-DockerBackendProbeFailure $probe)',
    '        $script:TaskspaceDockerProbeFailure = (($script:TaskspaceDockerProbeFailure, $nativeFailure) | Where-Object { -not [string]::IsNullOrWhiteSpace($_) }) -join "; "',
    '        if ($requested -eq "native") { throw "Requested native Docker backend is unavailable: $nativeFailure" }',
    '        }',
    '    }',
    '    if ([string]::IsNullOrWhiteSpace($script:TaskspaceDockerProbeFailure)) { throw "docker command is required for Terminal-Bench validation" }',
    '    throw "Docker backend unavailable for Terminal-Bench validation: $script:TaskspaceDockerProbeFailure"',
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
    '$probeResultPath = Join-Path $proofDir "validator-probe-result.json"',
    'function Write-ValidatorProbeResult {',
    '    param([string]$Status, [string]$Stage, [string]$StableCode = "", [string]$Message = "")',
    '    @{',
    '        schema_version = 1',
    '        status = $Status',
    '        stage = $Stage',
    '        runtime_manifest_path = if ($script:runtimeManifestPath) { $script:runtimeManifestPath } else { "" }',
    '        uv_cache_path = if ($script:uvCacheDir) { $script:uvCacheDir } else { "" }',
    '        docker_backend = $script:TaskspaceDockerBackend',
    '        failure_signature = if ($StableCode) { @{ schema_version = 1; category = "harness_materialization_failure"; stage = $Stage; stable_code = $StableCode; normalized_message = $Message; key = "harness_materialization_failure/$StableCode" } } else { $null }',
    '    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $probeResultPath -Encoding UTF8',
    '    Write-Host "validator_probe_result_path=$probeResultPath"',
    '}',
    'trap {',
    '    $message = [string]$_.Exception.Message',
    '    $code = if ($message -match "docker command is required|Docker backend unavailable|Requested WSL Docker backend is unavailable|Requested native Docker backend is unavailable|Unsupported TASKSPACE_DOCKER_BACKEND|getpwnam\(root\) failed|getpwuid\(0\) failed|Wsl/Service/E_UNEXPECTED|I/O error @util\.cpp") { "docker_backend_unavailable" } elseif ($message -match "Resolve-Path|Cannot find path|PathNotFound") { "path_unresolvable" } elseif ($message -match "run-tests script not found|validator script not found") { "validator_source_missing" } elseif ($message -match "uv[-_ ]cache|uv-x86_64|install\.sh") { "uv_cache_missing" } else { "validator_probe_failed" }',
    '    Write-ValidatorProbeResult "fail" "validator_pretest" $code $message',
    '    Write-Error $message',
    '    exit 3',
    '}',
    'Write-Host "validator_probe_started=true"',
    '$fixtureDir = Join-Path $scenarioRoot "fixture"',
    '$validatorSource = Join-Path $scenarioRoot "external-validator-source"',
    '$testScript = Join-Path $validatorSource "run-tests.sh"',
    'if (-not (Test-Path -LiteralPath $testScript)) { throw "Terminal-Bench run-tests script not found: $testScript" }',
    '$sha = [System.Security.Cryptography.SHA256]::Create()',
    '$repoHashBytes = $sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($repoDir))',
    '$repoHash = (([System.BitConverter]::ToString($repoHashBytes)) -replace "-", "").ToLowerInvariant().Substring(0, 16)',
    '$entryScript = Join-Path $scenarioRoot "terminal-bench-validator-entry.sh"',
    '$proofNonce = [guid]::NewGuid().ToString("N")',
    '$proofNoncePrefix = $proofNonce.Substring(0, 8)',
    '$proofDirHashBytes = $sha.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($proofDir))',
    '$proofDirHash = (([System.BitConverter]::ToString($proofDirHashBytes)) -replace "-", "").ToLowerInvariant().Substring(0, 16)',
    "`$dockerCacheSchemaVersion = $dockerCacheSchemaLiteral",
    "`$dockerCacheKey = $dockerCacheKeyLiteral",
    "`$fixtureTreeSha256 = $fixtureTreeShaLiteral",
    "`$dockerfileSha256 = $dockerfileShaLiteral",
    "`$validatorSourceSha256 = $validatorSourceShaLiteral",
    "`$adapterSha256 = $adapterShaLiteral",
    "`$uvInstallSha256 = $uvInstallShaLiteral",
    "`$uvArchiveSha256 = $uvArchiveShaLiteral",
    "`$dockerPlatform = $dockerPlatformLiteral",
    "`$dockerNetworkMode = $dockerNetworkModeLiteral",
    "`$dockerBuildEnvironmentMode = $dockerBuildEnvironmentModeLiteral",
    "`$cacheBaseImages = $baseImagesLiteral",
    "`$cacheEligible = $cacheEligibleLiteral",
    '$cacheEnabled = ([string]$env:TASKSPACE_DOCKER_IMAGE_CACHE -eq "1" -and $cacheEligible)',
    '$cacheBypassReason = if ($cacheEligible) { "" } else { "dockerfile_base_image_not_digest_pinned" }',
    '$cacheImage = "whale-taskspace-terminal-bench-cache:$dockerCacheKey"',
    '$cacheLockRoot = Join-Path ([System.IO.Path]::GetTempPath()) "whale-taskspace-docker-cache-locks"',
    '$cacheLockPath = Join-Path $cacheLockRoot "$dockerCacheKey.lock"',
    '$cacheManifestPath = Join-Path $proofDir "docker-cache-manifest.json"',
    '$script:TaskspaceDockerCacheLockWaitMs = 0',
    '$script:TaskspaceDockerCacheLockAcquired = $false',
    '$image = if ($cacheEnabled) { $cacheImage } else { "whale-taskspace-terminal-bench:$repoHash-$proofNoncePrefix" }',
    '$containerName = "whale-tbench-$repoHash-$proofNoncePrefix"',
    '$entryContent = @''',
    'set -euo pipefail',
    'export TEST_DIR=/tests',
    'export PATH=/tbench-uv-cache/bin:$PATH',
    'cd /app',
    'echo validator_lifecycle_stage=entry_started',
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
    'echo validator_lifecycle_stage=tests_started',
    'echo validator_tests_started=true',
    'set +e',
    'bash /tests/run-tests.sh',
    'test_exit=$?',
    'echo validator_lifecycle_stage=tests_completed',
    'echo validator_tests_completed=true',
    'exit $test_exit',
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
    "`$script:uvCacheDir = $uvCacheLiteral",
    '$uvCacheDir = $script:uvCacheDir',
    '$uvCacheDockerPath = ConvertTo-DockerPath $uvCacheDir $backend',
    '$uvInstallPath = Join-Path $uvCacheDir "install.sh"',
    '$uvArchivePath = Join-Path $uvCacheDir "uv-x86_64-unknown-linux-gnu.tar.gz"',
    'Write-Host "validator_runtime_probe=terminal_bench_equivalent_wrapper"',
    'Write-Host "validator_proof_nonce=$proofNonce"',
    'Write-Host "validator_wrapper_sha256=$wrapperSha"',
    'Write-Host "validator_entry_sha256=$entrySha"',
    'Write-Host "docker_backend=$backend"',
    'Write-Host "docker_cache_enabled=$cacheEnabled"',
    'Write-Host "docker_cache_eligible=$cacheEligible"',
    'Write-Host "docker_cache_bypass_reason=$cacheBypassReason"',
    'Write-Host "docker_cache_key=$dockerCacheKey"',
    'Write-Host "docker_image=$image"',
    'Write-Host "docker_container=$containerName"',
    'Write-Host "repo_mount=$repoDockerPath"',
    'Write-Host "validator_mount=/tests"',
    '$script:runtimeManifestPath = Join-Path $proofDir "terminal-bench-runtime-manifest.json"',
    '$runtimeManifestPath = $script:runtimeManifestPath',
    '$inspectPath = Join-Path $proofDir "terminal-bench-docker-inspect.json"',
    '$dockerResultPath = Join-Path $proofDir "docker-build-result.json"',
    '$dockerPhases = New-Object System.Collections.Generic.List[object]',
    'function Invoke-WithDockerCacheLock {',
    '    param([scriptblock]$Body)',
    '    if (-not $cacheEnabled) { & $Body; return }',
    '    New-Item -ItemType Directory -Path $cacheLockRoot -Force | Out-Null',
    '    $started = Get-Date',
    '    $lockStream = $null',
    '    while (-not $lockStream) {',
    '        try {',
    '            $lockStream = [System.IO.File]::Open($cacheLockPath, [System.IO.FileMode]::OpenOrCreate, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)',
    '        } catch [System.IO.IOException] {',
    '            Start-Sleep -Milliseconds 100',
    '            if (((Get-Date) - $started).TotalSeconds -gt 300) { throw "Timed out waiting for Docker cache lock: $cacheLockPath" }',
    '        }',
    '    }',
    '    $script:TaskspaceDockerCacheLockWaitMs = [int64](((Get-Date) - $started).TotalMilliseconds)',
    '    $script:TaskspaceDockerCacheLockAcquired = $true',
    '    try { & $Body } finally { if ($lockStream) { $lockStream.Dispose() } }',
    '}',
    'function Add-DockerPhaseResult {',
    '    param([string]$Phase, [int]$ExitCode, [string]$Classification, [datetime]$StartedAt, [datetime]$FinishedAt)',
    '    if ($StartedAt -eq [datetime]::MinValue) { $StartedAt = $FinishedAt }',
    '    if ($FinishedAt -eq [datetime]::MinValue) { $FinishedAt = Get-Date }',
    '    $script:dockerPhases.Add([pscustomobject]@{ phase = $Phase; exit_code = $ExitCode; classification = $Classification; started_at = $StartedAt.ToString("o"); finished_at = $FinishedAt.ToString("o"); duration_ms = [int64](($FinishedAt - $StartedAt).TotalMilliseconds); timestamp = $FinishedAt.ToString("o") })',
    '    @{',
    '        schema_version = 1',
    '        docker_backend = $script:TaskspaceDockerBackend',
    '        image = $image',
    '        cache_enabled = $cacheEnabled',
    '        cache_eligible = $cacheEligible',
    '        cache_bypass_reason = $cacheBypassReason',
        '        cache_hit = if ($script:TaskspaceDockerCacheHit) { $true } else { $false }',
        '        cache_key = $dockerCacheKey',
        '        cache_image = $cacheImage',
        '        cache_lock_path = $cacheLockPath',
        '        cache_lock_wait_ms = [int64]$script:TaskspaceDockerCacheLockWaitMs',
        '        cache_lock_acquired = [bool]$script:TaskspaceDockerCacheLockAcquired',
        '        cache_manifest_path = $cacheManifestPath',
        '        fixture_sha256 = $fixtureTreeSha256',
    '        dockerfile_sha256 = $dockerfileSha256',
    '        validator_source_sha256 = $validatorSourceSha256',
    '        adapter_sha256 = $adapterSha256',
    '        uv_install_sha256 = $uvInstallSha256',
    '        uv_archive_sha256 = $uvArchiveSha256',
    '        docker_platform = $dockerPlatform',
    '        docker_network_mode = $dockerNetworkMode',
    '        docker_build_environment_mode = $dockerBuildEnvironmentMode',
    '        dockerfile_from_images = @($cacheBaseImages)',
    '        container_name = $containerName',
    '        result_path = $dockerResultPath',
    '        phases = @($script:dockerPhases.ToArray())',
    '    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $dockerResultPath -Encoding UTF8',
    '}',
    '@{',
    '    proof_nonce = $proofNonce',
    '    wrapper_path = $PSCommandPath',
    '    wrapper_sha256 = $wrapperSha',
    '    entry_script_path = $entryScript',
    '    entry_sha256 = $entrySha',
    '    docker_backend = $backend',
    '    docker_cache_enabled = $cacheEnabled',
    '    docker_cache_eligible = $cacheEligible',
    '    docker_cache_bypass_reason = $cacheBypassReason',
    '    docker_cache_schema_version = $dockerCacheSchemaVersion',
    '    docker_cache_key = $dockerCacheKey',
    '    docker_cache_image = $cacheImage',
    '    fixture_sha256 = $fixtureTreeSha256',
    '    dockerfile_sha256 = $dockerfileSha256',
    '    validator_source_sha256 = $validatorSourceSha256',
    '    adapter_sha256 = $adapterSha256',
    '    uv_install_sha256 = $uvInstallSha256',
    '    uv_archive_sha256 = $uvArchiveSha256',
    '    docker_platform = $dockerPlatform',
    '    docker_network_mode = $dockerNetworkMode',
    '    docker_build_environment_mode = $dockerBuildEnvironmentMode',
    '    dockerfile_from_images = @($cacheBaseImages)',
    '    wsl_distro = if ($env:TASKSPACE_DOCKER_WSL_DISTRO) { $env:TASKSPACE_DOCKER_WSL_DISTRO } else { "whale-docker" }',
    '    image = $image',
    '    container_name = $containerName',
    '    repo_hash = $repoHash',
    '    proof_dir_hash = $proofDirHash',
    '    repo_mount = $repoDockerPath',
    '    validator_mount = $validatorDockerPath',
    '    validator_container_path = "/tests"',
    '    entry_mount = $entryDockerPath',
    '    uv_cache_mount = $uvCacheDockerPath',
    '    uv_cache_container_path = "/tbench-uv-cache"',
    '    uv_installer_sha256 = $uvInstallSha256',
    '    validator_command = "bash /tests/run-tests.sh"',
    '} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $runtimeManifestPath -Encoding UTF8',
    'Write-Host "validator_runtime_manifest_path=$runtimeManifestPath"',
    'if ($ProbeOnly -and -not $ProbeDocker) {',
    '    Write-Host "validator_probe_completed=true"',
    '    Write-ValidatorProbeResult "pass" "probe" "" ""',
    '    exit 0',
    '}',
    'Invoke-Docker -Arguments @("version", "--format", "{{.Server.Version}}")',
    'Write-Host "docker_version_exit=$($script:LastDockerExitCode)"',
    'if ($ProbeOnly) {',
    '    if ($script:LastDockerExitCode -ne 0) { Write-ValidatorProbeResult "fail" "probe_docker" "docker_backend_unavailable" "docker version failed"; exit 3 }',
    '    Write-Host "validator_probe_completed=true"',
    '    Write-ValidatorProbeResult "pass" "probe_docker" "" ""',
    '    exit 0',
    '}',
    '$networkArgs = if ($backend -eq "wsl") { @("--network", "host") } else { @("--add-host", "host.docker.internal:host-gateway") }',
    '$proxyArgs = @()',
    'foreach ($proxyName in @("HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy")) {',
    '    $proxyValue = [Environment]::GetEnvironmentVariable($proxyName)',
    '    if ([string]::IsNullOrWhiteSpace($proxyValue)) { continue }',
    '    if ($backend -eq "wsl" -and $proxyValue -match "://(127\.0\.0\.1|localhost):") {',
    '        Write-Host "proxy_env_skipped_loopback=$proxyName"',
    '        continue',
    '    }',
    '    if ($backend -ne "wsl") {',
    '        $proxyValue = $proxyValue -replace "://127\.0\.0\.1:", "://host.docker.internal:"',
    '        $proxyValue = $proxyValue -replace "://localhost:", "://host.docker.internal:"',
    '    }',
    '    $proxyArgs += @("-e", "$proxyName=$proxyValue")',
    '}',
    'Write-Host "proxy_env_count=$($proxyArgs.Count / 2)"',
    '$exitCode = 0',
    '$script:TaskspaceDockerCacheHit = $false',
    'try {',
    '    $phaseStartedAt = Get-Date',
    '    Invoke-WithDockerCacheLock {',
    '        if ($cacheEnabled) {',
    '            Invoke-DockerOutput -Arguments @("image", "inspect", $cacheImage) | Out-Null',
    '            if ($script:LastDockerExitCode -eq 0) {',
    '                $script:TaskspaceDockerCacheHit = $true',
    '            } else {',
    '                Invoke-Docker -Arguments @("build", "--pull", "-t", $cacheImage, $fixtureDockerPath)',
    '            }',
    '        } else {',
    '            Invoke-Docker -Arguments @("build", "--pull", "-t", $image, $fixtureDockerPath)',
    '        }',
    '    }',
    '    $phaseFinishedAt = Get-Date',
    '    Add-DockerPhaseResult "build" $script:LastDockerExitCode $(if ($script:TaskspaceDockerCacheHit) { "cache_hit" } elseif ($script:LastDockerExitCode -eq 0) { "ok" } else { "docker_build_environment_failure" }) $phaseStartedAt $phaseFinishedAt',
    '    @{',
    '        schema_version = 1',
        '        cache_enabled = $cacheEnabled',
        '        cache_eligible = $cacheEligible',
        '        cache_hit = [bool]$script:TaskspaceDockerCacheHit',
        '        cache_schema_version = $dockerCacheSchemaVersion',
        '        cache_key = $dockerCacheKey',
        '        cache_image = $cacheImage',
    '        cache_lock_path = $cacheLockPath',
    '        cache_lock_wait_ms = [int64]$script:TaskspaceDockerCacheLockWaitMs',
    '        cache_lock_acquired = [bool]$script:TaskspaceDockerCacheLockAcquired',
        '        fixture_sha256 = $fixtureTreeSha256',
        '        dockerfile_sha256 = $dockerfileSha256',
        '        validator_source_sha256 = $validatorSourceSha256',
        '        adapter_sha256 = $adapterSha256',
        '        uv_install_sha256 = $uvInstallSha256',
        '        uv_archive_sha256 = $uvArchiveSha256',
        '        docker_platform = $dockerPlatform',
        '        docker_network_mode = $dockerNetworkMode',
        '        docker_build_environment_mode = $dockerBuildEnvironmentMode',
        '        dockerfile_from_images = @($cacheBaseImages)',
    '        generated_at = (Get-Date).ToString("o")',
    '    } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $cacheManifestPath -Encoding UTF8',
    '    Write-Host "docker_cache_hit=$script:TaskspaceDockerCacheHit"',
    '    Write-Host "docker_cache_lock_wait_ms=$script:TaskspaceDockerCacheLockWaitMs"',
    '    Write-Host "docker_cache_manifest_path=$cacheManifestPath"',
    '    Write-Host "docker_build_result_path=$dockerResultPath"',
    '    if ($script:LastDockerExitCode -ne 0) { exit $script:LastDockerExitCode }',
    '    $runArgs = @(',
    '        "run", "--name", $containerName,',
    '        "--label", "whale.taskspace.terminal_bench=true",',
    '        "--label", "whale.taskspace.repo_hash=$repoHash",',
    '        "--label", "whale.taskspace.proof_nonce=$proofNonce",',
    '        "--label", "whale.taskspace.proof_dir_hash=$proofDirHash"',
    '    ) + $proxyArgs + @(',
    '        "-v", "${repoDockerPath}:/app",',
    '        "-v", "${validatorDockerPath}:/tests:ro",',
    '        "-v", "${entryDockerPath}:/tbench-entry.sh:ro",',
    '        "-v", "${uvCacheDockerPath}:/tbench-uv-cache:ro",',
    '        "-w", "/app",',
    '        "-e", "TEST_DIR=/tests",',
    '        "-e", "WHALE_TBENCH_PROOF_NONCE=$proofNonce",',
    '        $image, "bash", "/tbench-entry.sh"',
    '    )',
    '    $runArgs = @($runArgs[0..6]) + $networkArgs + @($runArgs[7..($runArgs.Count - 1)])',
    '    $phaseStartedAt = Get-Date',
    '    Invoke-Docker -Arguments $runArgs',
    '    $phaseFinishedAt = Get-Date',
    '    $exitCode = $script:LastDockerExitCode',
    '    Add-DockerPhaseResult "run" $script:LastDockerExitCode $(if ($script:LastDockerExitCode -eq 0) { "ok" } else { "docker_run_failure" }) $phaseStartedAt $phaseFinishedAt',
    '    $phaseStartedAt = Get-Date',
    '    $inspectOutput = Invoke-DockerOutput -Arguments @("inspect", $containerName)',
    '    $phaseFinishedAt = Get-Date',
    '    $inspectOutput | Set-Content -LiteralPath $inspectPath -Encoding UTF8',
    '    Add-DockerPhaseResult "inspect" $script:LastDockerExitCode $(if ($script:LastDockerExitCode -eq 0) { "ok" } else { "docker_inspect_failure" }) $phaseStartedAt $phaseFinishedAt',
    '    Write-Host "docker_inspect_path=$inspectPath"',
    '    Write-Host "docker_inspect_available=$($script:LastDockerExitCode -eq 0)"',
    '} finally {',
    '    Write-Host "validator_cleanup_deferred_to_runner=true"',
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
    e3_eligible = ($officialProven -and $remoteAssetsE3Eligible)
    downgrade_reason = if (-not $officialProven) {
        "Terminal-Bench official protocol source hashes were not available from a pinned checkout."
    } elseif (-not $remoteAssetsE3Eligible) {
        "Terminal-Bench sample references remote assets without content-addressed local equivalence proof."
    } else { "" }
}
$adapterMetadata = [ordered]@{
    instruction_extraction_mode = $instructionMode
    instruction_line = $instructionLine
    fixture_mode = $fixtureMode
    generated_fixture_allowlist = @($generatedFixtureAllowlist)
    validator_dependency_cache = $uvCache
    docker_image_cache = [ordered]@{
        schema_version = 2
        cache_schema_version = $dockerCacheSchemaVersion
        opt_in_env = "TASKSPACE_DOCKER_IMAGE_CACHE=1"
        cache_key = $dockerCacheKey
        cache_image = "whale-taskspace-terminal-bench-cache:$dockerCacheKey"
        cache_eligible = [bool]$baseImageProof.digest_pinned
        cache_bypass_reason = if ([bool]$baseImageProof.digest_pinned) { "" } else { "dockerfile_base_image_not_digest_pinned" }
        fixture_sha256 = $fixtureTreeSha
        dockerfile_sha256 = $dockerfileSha
        validator_source_sha256 = $originalValidatorSha
        adapter_sha256 = $adapterSha
        uv_install_sha256 = $uvInstallSha
        uv_archive_sha256 = $uvArchiveSha
        docker_platform = $dockerPlatform
        docker_network_mode = $dockerNetworkMode
        docker_build_environment_mode = $dockerBuildEnvironmentMode
        dockerfile_from_images = @($baseImageProof.from_images)
        source_version = $SourceVersion
        default_enabled = $false
    }
    uv_cache_root = [string]$uvCache.root
    validator_source_dir = $validatorSourceDir
    fixture_source = $fixtureSource
    generated_validator_path = $validator
    validator_probe_supported = $true
    prompt_adaptation = "current_working_directory_is_terminal_bench_app"
    original_instruction_sha256 = (Get-TaskspaceExternalFileSha256 $originalPromptSource)
    solution_visible_to_agent = $false
    engineering_smoke_only = $false
    validator_mount = "/tests:ro"
    repo_mount = "/app"
    agent_execution_app_alias = $true
    official_equivalence = $officialEquivalence
    sensitive_source_files = $sensitiveFiles
    remote_assets = @($remoteAssets)
    e3_downgraded_until_runtime_fidelity_proven = -not $officialProven
    e3_downgraded_until_remote_assets_proven = -not $remoteAssetsE3Eligible
}
$promptGuard = [ordered]@{
    allowed_domain_terms = @(
        "(?i)\bmulti-agent\b",
        "(?i)\bmultiple\s+agents?\b"
    )
    source_spans = @(
        [ordered]@{
            source_kind = "upstream_task"
            source_path = $originalPromptSource
            line_start = 1
            line_end = @($upstreamPromptText -split "`r?`n").Count
            byte_start = 0
            byte_end = [System.Text.Encoding]::UTF8.GetByteCount($upstreamPromptText)
            raw_sha256 = Get-TerminalBenchStringSha256 $upstreamPromptText
            adapted_sha256 = Get-TerminalBenchStringSha256 $adaptedPromptText
            start = 0
            end = $upstreamPromptText.Length
        },
        [ordered]@{
            source_kind = "adapter_wrapper"
            source_path = $adaptedPrompt
            line_start = @($upstreamPromptText -split "`r?`n").Count + 1
            line_end = @($adaptedPromptText -split "`r?`n").Count
            byte_start = [System.Text.Encoding]::UTF8.GetByteCount($upstreamPromptText)
            byte_end = [System.Text.Encoding]::UTF8.GetByteCount($adaptedPromptText)
            raw_sha256 = Get-TerminalBenchStringSha256 $runnerNote
            adapted_sha256 = Get-TerminalBenchStringSha256 $adaptedPromptText
            start = $upstreamPromptText.Length
            end = $adaptedPromptText.Length
        }
    )
}
New-TaskspaceExternalScenario $scenarioDir $scenarioId "terminal-bench" "whale-taskspace-e3-terminal-bench-v1" $promptSource $fixtureSource $validator $validatorSourceDir $originalValidatorSha $SampleId $SourceVersion "https://github.com/laude-institute/terminal-bench" "external-benchmark-license-see-source" "pointer_only_no_solution_or_hidden_tests" "Terminal-Bench coding/file/debug/data-processing subset" $validatorFidelity $adapterMetadata $promptGuard
