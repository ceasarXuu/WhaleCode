function Invoke-TaskspaceValidationCommand {
    param(
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][object]$Validation,
        [Parameter(Mandatory = $true)][string]$StdoutPath,
        [Parameter(Mandatory = $true)][string]$StderrPath,
        [int]$TimeoutSeconds = 120,
        [string]$ProofDir = "",
        [string[]]$ExtraArgs = @()
    )
    $args = @($Validation.args | ForEach-Object { [string]$_ }) + @($ExtraArgs | ForEach-Object { [string]$_ })
    $oldProofDir = $env:TASKSPACE_VALIDATION_ARTIFACT_DIR
    $exitCode = 0
    $timedOut = $false
    try {
        if (-not [string]::IsNullOrWhiteSpace($ProofDir)) {
            New-Dir $ProofDir | Out-Null
            $env:TASKSPACE_VALIDATION_ARTIFACT_DIR = (Resolve-Path -LiteralPath $ProofDir).Path
        }
        try {
            $exitCode = Invoke-RealProcess ([string]$Validation.command) $args $RepoDir $StdoutPath $StderrPath $TimeoutSeconds
        } catch {
            if ([string]$_.Exception.Message -notmatch "^Process timed out after ") { throw }
            $timedOut = $true
            $exitCode = 124
            if (-not (Test-Path -LiteralPath $StdoutPath)) { Write-Text $StdoutPath "" }
            Write-Text $StderrPath "Validation timed out after $TimeoutSeconds seconds.`n$($_.Exception.Message)`n"
        }
        $cleanupReason = if ($timedOut) { "timeout" } else { "post_validation" }
        Invoke-TaskspaceValidationCleanupNoThrow $ProofDir $cleanupReason $StderrPath | Out-Null
        $exitCode
    } finally {
        if ($null -eq $oldProofDir) {
            Remove-Item Env:\TASKSPACE_VALIDATION_ARTIFACT_DIR -ErrorAction SilentlyContinue
        } else {
            $env:TASKSPACE_VALIDATION_ARTIFACT_DIR = $oldProofDir
        }
    }
}

function Invoke-TaskspaceValidationCleanupNoThrow {
    param(
        [string]$ProofDir = "",
        [string]$Reason = "post_validation",
        [string]$StderrPath = ""
    )
    try {
        return Invoke-TaskspaceValidationCleanup $ProofDir $Reason $StderrPath
    } catch {
        if ([string]::IsNullOrWhiteSpace($ProofDir)) { return $null }
        try {
            New-Item -ItemType Directory -Path $ProofDir -Force -ErrorAction SilentlyContinue | Out-Null
            $cleanupResultPath = Join-Path $ProofDir "validation-cleanup-result.json"
            $result = [pscustomobject]@{
                schema_version = 1
                reason = $Reason
                proof_dir = $ProofDir
                cleanup_attempted = $false
                identity_matched = $false
                classification = "validation_cleanup_exception"
                error = [string]$_.Exception.Message
                timestamp = (Get-Date).ToString("o")
            }
            ($result | ConvertTo-Json -Depth 8) | Set-Content -LiteralPath $cleanupResultPath -Encoding UTF8 -ErrorAction SilentlyContinue
            if (-not [string]::IsNullOrWhiteSpace($StderrPath)) {
            Add-Content -LiteralPath $StderrPath -Encoding UTF8 -Value "`nvalidation_cleanup_result_path=$cleanupResultPath" -ErrorAction SilentlyContinue
            }
            return $result
        } catch {
            return [pscustomobject]@{
                schema_version = 1
                reason = $Reason
                proof_dir = $ProofDir
                cleanup_attempted = $false
                identity_matched = $false
                classification = "validation_cleanup_exception_unrecorded"
                error = [string]$_.Exception.Message
                timestamp = (Get-Date).ToString("o")
            }
        }
    }
}

function ConvertTo-TaskspaceProcessArgumentString {
    param([string[]]$Arguments)
    (($Arguments | ForEach-Object {
                $arg = [string]$_
                if ($arg -match '[\s"]') { '"' + ($arg -replace '"', '\"') + '"' } else { $arg }
            }) -join " ")
}

function Invoke-TaskspaceCleanupProcess {
    param(
        [Parameter(Mandatory = $true)][string]$FilePath,
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [int]$TimeoutSeconds = 30
    )
    $encoding = [System.Text.UTF8Encoding]::new($false)
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $resolved = Get-Command $FilePath -ErrorAction SilentlyContinue | Select-Object -First 1
    $startInfo.FileName = if ($resolved) { [string]$resolved.Source } else { $FilePath }
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.StandardOutputEncoding = $encoding
    $startInfo.StandardErrorEncoding = $encoding
    $startInfo.Arguments = ConvertTo-TaskspaceProcessArgumentString $Arguments
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    try {
        [void]$process.Start()
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        $timedOut = -not $process.WaitForExit($TimeoutSeconds * 1000)
        if ($timedOut) {
            try { $process.Kill($true) } catch { try { $process.Kill() } catch {} }
            return [pscustomobject]@{
                exit_code = 124
                timed_out = $true
                stdout = ""
                stderr = "Cleanup process timed out after $TimeoutSeconds seconds: $FilePath $($Arguments -join ' ')"
            }
        }
        $stdoutTask.Wait()
        $stderrTask.Wait()
        [pscustomobject]@{
            exit_code = [int]$process.ExitCode
            timed_out = $false
            stdout = [string]$stdoutTask.Result
            stderr = [string]$stderrTask.Result
        }
    } catch {
        [pscustomobject]@{
            exit_code = -1
            timed_out = $false
            stdout = ""
            stderr = [string]$_.Exception.Message
        }
    } finally {
        $process.Dispose()
    }
}

function Invoke-TaskspaceValidationDockerCommand {
    param(
        [Parameter(Mandatory = $true)][string]$Backend,
        [Parameter(Mandatory = $true)][string[]]$DockerArguments
    )
    $backendNormalized = $Backend.ToLowerInvariant()
    if ($backendNormalized -eq "wsl") {
        $distro = if ($script:TaskspaceValidationCleanupWslDistro) { $script:TaskspaceValidationCleanupWslDistro } elseif ($env:TASKSPACE_DOCKER_WSL_DISTRO) { $env:TASKSPACE_DOCKER_WSL_DISTRO } else { "whale-docker" }
        return Invoke-TaskspaceCleanupProcess "wsl" (@("-d", $distro, "--", "docker") + $DockerArguments) 30
    }
    Invoke-TaskspaceCleanupProcess "docker" $DockerArguments 30
}

function Get-TaskspaceValidationManifestValue {
    param(
        $Manifest,
        [Parameter(Mandatory = $true)][string]$Name
    )
    if ($null -ne $Manifest -and $Manifest.PSObject.Properties.Name -contains $Name) {
        return [string]$Manifest.$Name
    }
    ""
}

function Invoke-TaskspaceValidationCleanup {
    param(
        [string]$ProofDir = "",
        [string]$Reason = "post_validation",
        [string]$StderrPath = ""
    )
    if ([string]::IsNullOrWhiteSpace($ProofDir)) { return $null }
    New-Item -ItemType Directory -Path $ProofDir -Force | Out-Null
    $runtimeManifestPath = Join-Path $ProofDir "terminal-bench-runtime-manifest.json"
    $cleanupResultPath = Join-Path $ProofDir "validation-cleanup-result.json"
    if (-not (Test-Path -LiteralPath $runtimeManifestPath)) {
        $result = [pscustomobject]@{
            schema_version = 1
            reason = $Reason
            proof_dir = $ProofDir
            runtime_manifest_path = $runtimeManifestPath
            cleanup_attempted = $false
            identity_matched = $false
            classification = "cleanup_not_attempted_manifest_missing"
            timestamp = (Get-Date).ToString("o")
        }
        ($result | ConvertTo-Json -Depth 8) | Set-Content -LiteralPath $cleanupResultPath -Encoding UTF8
        if (-not [string]::IsNullOrWhiteSpace($StderrPath)) {
            Add-Content -LiteralPath $StderrPath -Encoding UTF8 -Value "`nvalidation_cleanup_result_path=$cleanupResultPath"
        }
        return $result
    }

    $manifest = $null
    try {
        $manifest = Get-Content -Raw -Encoding UTF8 -LiteralPath $runtimeManifestPath | ConvertFrom-Json
    } catch {
        $manifest = $null
    }
    $containerName = Get-TaskspaceValidationManifestValue $manifest "container_name"
    $image = Get-TaskspaceValidationManifestValue $manifest "image"
    $backend = Get-TaskspaceValidationManifestValue $manifest "docker_backend"
    $wslDistro = Get-TaskspaceValidationManifestValue $manifest "wsl_distro"
    $repoHash = Get-TaskspaceValidationManifestValue $manifest "repo_hash"
    $proofNonce = Get-TaskspaceValidationManifestValue $manifest "proof_nonce"
    $proofDirHash = Get-TaskspaceValidationManifestValue $manifest "proof_dir_hash"
    if ([string]::IsNullOrWhiteSpace($backend)) { $backend = "native" }
    $script:TaskspaceValidationCleanupWslDistro = $wslDistro
    $validContainerName = ($containerName -match '^whale-tbench-[0-9a-f]{16}(-[0-9a-f]{8})?$')
    $validImage = ($image -match '^whale-taskspace-terminal-bench:[0-9a-f]{16}(-[0-9a-f]{8})?$')

    $inspect = $null
    $remove = $null
    $imageInspect = $null
    $imageRemove = $null
    $classification = "ok"
    $identityMatched = $false
    $inspectContainerId = ""
    if ($validContainerName) {
        $inspect = Invoke-TaskspaceValidationDockerCommand $backend @("inspect", $containerName)
        if ([int]$inspect.exit_code -eq 0) {
            $inspectJson = $null
            try { $inspectJson = [string]$inspect.stdout | ConvertFrom-Json } catch { $inspectJson = $null }
            $container = @($inspectJson | Select-Object -First 1)[0]
            $labels = if ($container -and $container.PSObject.Properties.Name -contains "Config" -and $container.Config.PSObject.Properties.Name -contains "Labels") { $container.Config.Labels } else { $null }
            $inspectContainerId = if ($container -and $container.PSObject.Properties.Name -contains "Id") { [string]$container.Id } else { "" }
            $identityMatched = (
                $labels -and
                $labels.PSObject.Properties.Name -contains "whale.taskspace.terminal_bench" -and [string]$labels."whale.taskspace.terminal_bench" -eq "true" -and
                $labels.PSObject.Properties.Name -contains "whale.taskspace.repo_hash" -and [string]$labels."whale.taskspace.repo_hash" -eq $repoHash -and
                $labels.PSObject.Properties.Name -contains "whale.taskspace.proof_nonce" -and [string]$labels."whale.taskspace.proof_nonce" -eq $proofNonce -and
                $labels.PSObject.Properties.Name -contains "whale.taskspace.proof_dir_hash" -and [string]$labels."whale.taskspace.proof_dir_hash" -eq $proofDirHash)
            if ($identityMatched) {
                $remove = Invoke-TaskspaceValidationDockerCommand $backend @("rm", "-f", $containerName)
                if ([int]$remove.exit_code -ne 0) { $classification = "docker_cleanup_container_failure" }
            } else {
                $classification = "docker_cleanup_identity_mismatch"
            }
        } elseif ([string]$inspect.stderr -notmatch "No such object|No such container|not found") {
            $classification = "docker_cleanup_container_inspect_failure"
        }
    } else {
        $classification = "docker_cleanup_manifest_invalid"
    }
    if ($validImage) {
        $imageInspect = Invoke-TaskspaceValidationDockerCommand $backend @("image", "inspect", $image)
        if ([int]$imageInspect.exit_code -eq 0) {
            $imageRemove = Invoke-TaskspaceValidationDockerCommand $backend @("rmi", "-f", $image)
            if ([int]$imageRemove.exit_code -ne 0 -and $classification -eq "ok") { $classification = "docker_cleanup_image_failure" }
        }
    }

    $result = [pscustomobject]@{
        schema_version = 1
        reason = $Reason
        proof_dir = $ProofDir
        runtime_manifest_path = $runtimeManifestPath
        container_name = $containerName
        container_id = $inspectContainerId
        image = $image
        docker_backend = $backend
        wsl_distro = $wslDistro
        repo_hash = $repoHash
        proof_nonce = $proofNonce
        proof_dir_hash = $proofDirHash
        identity_matched = $identityMatched
        cleanup_attempted = ($identityMatched -and $null -ne $inspect -and [int]$inspect.exit_code -eq 0)
        classification = $classification
        container_inspect_exit_code = if ($null -ne $inspect) { [int]$inspect.exit_code } else { $null }
        container_rm_exit_code = if ($null -ne $remove) { [int]$remove.exit_code } else { $null }
        image_inspect_exit_code = if ($null -ne $imageInspect) { [int]$imageInspect.exit_code } else { $null }
        image_rm_exit_code = if ($null -ne $imageRemove) { [int]$imageRemove.exit_code } else { $null }
        timestamp = (Get-Date).ToString("o")
    }
    ($result | ConvertTo-Json -Depth 8) | Set-Content -LiteralPath $cleanupResultPath -Encoding UTF8
    if (-not [string]::IsNullOrWhiteSpace($StderrPath)) {
        Add-Content -LiteralPath $StderrPath -Encoding UTF8 -Value "`nvalidation_cleanup_result_path=$cleanupResultPath"
    }
    $result
}

function Test-TaskspaceOracleLeak {
    param(
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [Parameter(Mandatory = $true)][string]$HiddenOraclePath,
        [string]$ScenarioOraclePath = ""
    )
    $targets = @("private-oracle", "reviewer-only")
    $targets += $HiddenOraclePath
    if ($ScenarioOraclePath) { $targets += $ScenarioOraclePath }
    $visibleRepoHits = @(Get-ChildItem -LiteralPath $RepoDir -Recurse -Force -ErrorAction SilentlyContinue |
        Where-Object {
            $full = [string]$_.FullName
            @($targets | Where-Object { $full -like "*$_*" }).Count -gt 0
        })
    $textHits = New-Object System.Collections.Generic.List[string]
    foreach ($file in @(Get-ChildItem -LiteralPath $ArtifactDir -Recurse -File -ErrorAction SilentlyContinue)) {
        try {
            $text = Get-Content -Raw -Encoding UTF8 -LiteralPath $file.FullName
        } catch {
            continue
        }
        if ($null -eq $text) { continue }
        foreach ($target in $targets) {
            if (-not [string]::IsNullOrWhiteSpace($target) -and $text.Contains($target)) {
                $textHits.Add($file.FullName)
                break
            }
        }
    }
    [pscustomobject]@{
        leaked = ($visibleRepoHits.Count -gt 0 -or $textHits.Count -gt 0)
        repo_hits = @($visibleRepoHits | ForEach-Object { $_.FullName })
        artifact_hits = @($textHits.ToArray())
    }
}

function Invoke-TaskspaceHiddenOracle {
    param(
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [Parameter(Mandatory = $true)][string]$HiddenOraclePath,
        [string]$ScenarioOraclePath = "",
        [switch]$BypassSandbox
    )
    $stdoutPath = Join-Path $ArtifactDir "hidden-oracle.stdout.log"
    $stderrPath = Join-Path $ArtifactDir "hidden-oracle.stderr.log"
    $exitCode = Invoke-RealProcess "python" @($HiddenOraclePath, $RepoDir) $RepoDir $stdoutPath $stderrPath 120
    $leak = Test-TaskspaceOracleLeak $RepoDir $ArtifactDir $HiddenOraclePath $ScenarioOraclePath
    $level = if ($leak.leaked) { "failed" } elseif ($BypassSandbox) { "soft_denylist" } else { "hard_sandbox" }
    [pscustomobject]@{
        exit_code = $exitCode
        stdout_path = $stdoutPath
        stderr_path = $stderrPath
        oracle_sha256 = (Get-TaskspaceFileSha256 $HiddenOraclePath)
        oracle_isolation_level = $level
        oracle_isolation_failure = ($level -eq "failed")
        leak = $leak
    }
}

function Invoke-TaskspaceOracleIsolationProbe {
    param(
        [Parameter(Mandatory = $true)][string]$WhaleBin,
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [Parameter(Mandatory = $true)][string]$CanaryPath,
        [Parameter(Mandatory = $true)][string]$CanaryText,
        [Parameter(Mandatory = $true)][string]$Model,
        [string]$SandboxMode = "bypass",
        [string[]]$ConfigOverrides = @(),
        [int]$TimeoutSeconds = 180
    )
    $probeDir = New-Dir (Join-Path $ArtifactDir "oracle-isolation-probe")
    $promptPath = Join-Path $probeDir "prompt.txt"
    $jsonlPath = Join-Path $probeDir "whale-exec.jsonl"
    $stderrPath = Join-Path $probeDir "whale-exec.stderr.log"
    $lastMessagePath = Join-Path $probeDir "last-message.md"
    $prompt = "Run a shell command that attempts to read this exact file path and print its content if accessible: $CanaryPath"
    Write-Text $promptPath $prompt
    $args = New-TaskspaceWhaleArgv "standard" $Model $RepoDir $lastMessagePath $SandboxMode $ConfigOverrides
    $exitCode = Invoke-RealProcess $WhaleBin $args $RepoDir $jsonlPath $stderrPath $TimeoutSeconds $promptPath
    $combined = ""
    foreach ($path in @($jsonlPath, $stderrPath, $lastMessagePath)) {
        if (Test-Path -LiteralPath $path) {
            $combined += "`n" + (Get-Content -Raw -Encoding UTF8 -LiteralPath $path)
        }
    }
    $canaryLeaked = $combined.Contains($CanaryText)
    $canaryMaterializedDuringProbe = Test-Path -LiteralPath $CanaryPath
    $pathMentioned = $combined.Contains($CanaryPath)
    $level = if ($canaryLeaked) {
        "failed"
    } elseif (-not $canaryMaterializedDuringProbe) {
        "hard_deferred_materialization"
    } elseif ($SandboxMode -eq "bypass") {
        "soft_denylist"
    } else {
        "hard_sandbox"
    }
    [pscustomobject]@{
        exit_code = $exitCode
        jsonl_path = $jsonlPath
        stderr_path = $stderrPath
        last_message_path = $lastMessagePath
        canary_path = $CanaryPath
        canary_leaked = $canaryLeaked
        canary_materialized_during_probe = $canaryMaterializedDuringProbe
        path_mentioned = $pathMentioned
        oracle_isolation_level = $level
    }
}
