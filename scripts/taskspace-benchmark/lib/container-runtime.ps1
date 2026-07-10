function Invoke-TaskspaceDocker {
    param(
        [Parameter(Mandatory = $true)][string[]]$Arguments,
        [string]$StdoutPath = "",
        [string]$StderrPath = ""
    )
    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = "docker"
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.StandardOutputEncoding = [System.Text.UTF8Encoding]::new($false)
    $start.StandardErrorEncoding = [System.Text.UTF8Encoding]::new($false)
    foreach ($argument in $Arguments) { [void]$start.ArgumentList.Add([string]$argument) }
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $start
    try {
        [void]$process.Start()
        $stdoutTask = $process.StandardOutput.ReadToEndAsync()
        $stderrTask = $process.StandardError.ReadToEndAsync()
        $process.WaitForExit()
        $stdout = $stdoutTask.GetAwaiter().GetResult()
        $stderr = $stderrTask.GetAwaiter().GetResult()
        if ($StdoutPath) { Write-Text $StdoutPath $stdout }
        if ($StderrPath) { Write-Text $StderrPath $stderr }
        [pscustomobject]@{ exit_code = [int]$process.ExitCode; stdout = $stdout; stderr = $stderr }
    } finally {
        $process.Dispose()
    }
}

function Write-TaskspaceContainerEvent {
    param([string]$Path, [string]$Event, [hashtable]$Fields = @{})
    $record = [ordered]@{
        schema_version = 1
        timestamp = (Get-Date).ToUniversalTime().ToString("o")
        event = $Event
    }
    foreach ($key in @($Fields.Keys | Sort-Object)) { $record[$key] = $Fields[$key] }
    Add-Content -LiteralPath $Path -Encoding UTF8 -Value ($record | ConvertTo-Json -Compress -Depth 12)
}

function Write-TaskspaceContainerJson {
    param($Value, [Parameter(Mandatory = $true)][string]$Path)
    $parent = Split-Path -Parent $Path
    if ($parent) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    [System.IO.File]::WriteAllText(
        $Path,
        ($Value | ConvertTo-Json -Depth 20),
        [System.Text.UTF8Encoding]::new($false)
    )
}

function Test-TaskspaceDockerBackend {
    $result = Invoke-TaskspaceDocker @('version', '--format', '{{.Server.Version}}')
    if ($result.exit_code -ne 0 -or [string]::IsNullOrWhiteSpace($result.stdout)) {
        throw "docker_unavailable: $($result.stderr.Trim())"
    }
    $result.stdout.Trim()
}

function Resolve-TaskspaceContainerImage {
    param(
        [Parameter(Mandatory = $true)][string]$RepoRoot,
        [Parameter(Mandatory = $true)]$Contract,
        [switch]$ForceBuild
    )
    $serverVersion = Test-TaskspaceDockerBackend
    $tag = [string]$Contract.image_tag
    $inspect = Invoke-TaskspaceDocker @('image', 'inspect', $tag, '--format', '{{.Id}}')
    $buildStarted = Get-Date
    if ($ForceBuild -or $inspect.exit_code -ne 0) {
        $dockerDir = Join-Path $RepoRoot 'scripts/taskspace-benchmark/docker'
        $build = Invoke-TaskspaceDocker @('build', '--pull=false', '--tag', $tag, $dockerDir)
        if ($build.exit_code -ne 0) { throw "image_build_failed: $($build.stderr.Trim())" }
    }
    $image = Invoke-TaskspaceDocker @('image', 'inspect', $tag, '--format', '{{json .}}')
    if ($image.exit_code -ne 0) { throw "image_digest_missing: $($image.stderr.Trim())" }
    $metadata = $image.stdout | ConvertFrom-Json
    $preflight = Invoke-TaskspaceDocker @(
        'run', '--rm', $tag, 'bash', '-lc',
        'python --version && python -m pytest --version && git --version && rg --version | head -n 1'
    )
    if ($preflight.exit_code -ne 0) {
        throw "container_preflight_failed: $($preflight.stderr.Trim())"
    }
    [pscustomobject]@{
        image_ref = $tag
        image_id = [string]$metadata.Id
        image_digest = [string]$metadata.Id
        base_image = [string]$Contract.base_image
        docker_server_version = $serverVersion
        build_duration_ms = [int64](((Get-Date) - $buildStarted).TotalMilliseconds)
        preflight_output = $preflight.stdout.Trim()
    }
}

function New-TaskspaceContainerSecret {
    param([Parameter(Mandatory = $true)][string]$Root, [Parameter(Mandatory = $true)][string]$Value)
    if ([string]::IsNullOrWhiteSpace($Value)) { throw 'secret_materialization_failed: empty value' }
    $secretRoot = New-Dir (Join-Path $Root '.container-secrets')
    $path = Join-Path $secretRoot ("deepseek-{0}.secret" -f ([guid]::NewGuid().ToString('N')))
    Write-Text $path $Value
    if (-not $IsWindows) { & chmod 600 $path }
    $path
}

function Remove-TaskspaceContainerSecret {
    param([string]$Path)
    if (-not [string]::IsNullOrWhiteSpace($Path) -and (Test-Path -LiteralPath $Path)) {
        Remove-Item -LiteralPath $Path -Force
    }
}

function Add-TaskspaceContainerManifestEntry {
    param([string]$Path, $Entry)
    $entries = @()
    if (Test-Path -LiteralPath $Path) {
        try { $entries = @(Get-Content -Raw -Encoding UTF8 -LiteralPath $Path | ConvertFrom-Json) } catch {}
    }
    $entries += $Entry
    Write-TaskspaceContainerJson @($entries) $Path
}

function Get-TaskspaceContainerMountArg {
    param([string]$Source, [string]$Destination, [switch]$ReadOnly)
    $resolved = (Resolve-Path -LiteralPath $Source).Path
    $value = "type=bind,src=$resolved,dst=$Destination"
    if ($ReadOnly) { $value += ',readonly' }
    @('--mount', $value)
}

function Invoke-TaskspaceContainerRole {
    param(
        [Parameter(Mandatory = $true)][ValidateSet('agent', 'validator', 'oracle')][string]$Role,
        [Parameter(Mandatory = $true)]$Image,
        [Parameter(Mandatory = $true)]$Contract,
        [Parameter(Mandatory = $true)][string]$WorkspaceDir,
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [Parameter(Mandatory = $true)][string[]]$Command,
        [Parameter(Mandatory = $true)][int]$TimeoutSeconds,
        [Parameter(Mandatory = $true)][hashtable]$Identity,
        [string]$WhaleBin = "",
        [string]$SecretPath = "",
        [string]$OraclePath = "",
        [hashtable]$Environment = @{},
        [switch]$WorkspaceReadOnly
    )
    New-Dir $ArtifactDir | Out-Null
    $eventsPath = Join-Path $ArtifactDir 'container-lifecycle-events.jsonl'
    $manifestPath = Join-Path $ArtifactDir 'container-runtime-manifest.json'
    $statsPath = Join-Path $ArtifactDir 'container-stats.jsonl'
    $stdoutPath = Join-Path $ArtifactDir "container-$Role.stdout.log"
    $stderrPath = Join-Path $ArtifactDir "container-$Role.stderr.log"
    $inspectPath = Join-Path $ArtifactDir "container-inspect-$Role.json"
    $cleanupPath = Join-Path $ArtifactDir "container-cleanup-$Role.json"
    $cleanupAggregatePath = Join-Path $ArtifactDir 'container-cleanup-result.json'
    $name = "whale-r5-$($Identity.run_id)-$($Identity.pair_id)-$($Identity.side)-$Role-$([guid]::NewGuid().ToString('N').Substring(0,8))" -replace '[^a-zA-Z0-9_.-]', '-'
    $createArgs = @('create', '--name', $name)
    foreach ($key in @('run_id', 'sample_id', 'pair_id', 'side', 'logical_mode')) {
        $createArgs += @('--label', "whalecode.$key=$([string]$Identity[$key])")
    }
    $createArgs += @(Get-TaskspaceContainerResourceArgs $Contract)
    $createArgs += @(Get-TaskspaceContainerLogArgs $Contract)
    $createArgs += @(Get-TaskspaceContainerMountArg $WorkspaceDir ([string]$Contract.paths.workspace) -ReadOnly:$WorkspaceReadOnly)
    $createArgs += @(Get-TaskspaceContainerMountArg $ArtifactDir ([string]$Contract.paths.artifacts))
    if ($WhaleBin) { $createArgs += @(Get-TaskspaceContainerMountArg $WhaleBin ([string]$Contract.paths.whale_binary) -ReadOnly) }
    if ($SecretPath) { $createArgs += @(Get-TaskspaceContainerMountArg $SecretPath ([string]$Contract.paths.provider_secret) -ReadOnly) }
    if ($OraclePath) { $createArgs += @(Get-TaskspaceContainerMountArg $OraclePath '/oracle/oracle.py' -ReadOnly) }
    $createArgs += @('--workdir', [string]$Contract.paths.workspace, '--env', "HOME=$([string]$Contract.paths.home)")
    foreach ($key in @($Environment.Keys | Sort-Object)) { $createArgs += @('--env', "$key=$([string]$Environment[$key])") }
    $createArgs += @([string]$Image.image_ref)
    $createArgs += @($Command)
    $containerId = ""
    $startedAt = Get-Date
    $timedOut = $false
    $exitCode = 125
    $reasonCode = 'container_create_failed'
    try {
        Write-TaskspaceContainerEvent $eventsPath 'container.create_started' ($Identity + @{ role = $Role; container_name = $name })
        $create = Invoke-TaskspaceDocker $createArgs
        if ($create.exit_code -ne 0) { throw "container_create_failed: $($create.stderr.Trim())" }
        $containerId = $create.stdout.Trim()
        Write-TaskspaceContainerEvent $eventsPath 'container.created' ($Identity + @{ role = $Role; container_id = $containerId })
        $preflight = Invoke-TaskspaceDocker @('inspect', $containerId, '--format', '{{.State.Status}}')
        if ($preflight.exit_code -ne 0) { throw "container_preflight_failed: $($preflight.stderr.Trim())" }
        Write-TaskspaceContainerEvent $eventsPath 'container.preflight_passed' ($Identity + @{ role = $Role; container_id = $containerId })
        $start = Invoke-TaskspaceDocker @('start', $containerId)
        if ($start.exit_code -ne 0) { throw "container_start_failed: $($start.stderr.Trim())" }
        $reasonCode = ''
        Write-TaskspaceContainerEvent $eventsPath "container.$Role`_started" ($Identity + @{ role = $Role; container_id = $containerId })
        $nextStatsAt = $startedAt.AddMilliseconds([int]$Contract.logging.stats_interval_ms)
        while ($true) {
            $state = Invoke-TaskspaceDocker @('inspect', $containerId, '--format', '{{json .State}}')
            if ($state.exit_code -ne 0) { throw "container_inspect_failed: $($state.stderr.Trim())" }
            $stateValue = $state.stdout | ConvertFrom-Json
            if (-not [bool]$stateValue.Running) { $exitCode = [int]$stateValue.ExitCode; break }
            $now = Get-Date
            if (($now - $startedAt).TotalSeconds -ge $TimeoutSeconds) {
                $timedOut = $true
                $reasonCode = 'container_timeout'
                [void](Invoke-TaskspaceDocker @('kill', $containerId))
                $exitCode = 124
                break
            }
            if ($now -ge $nextStatsAt) {
                $stats = Invoke-TaskspaceDocker @('stats', '--no-stream', '--format', '{{json .}}', $containerId)
                if ($stats.exit_code -eq 0 -and -not [string]::IsNullOrWhiteSpace($stats.stdout)) {
                    Add-Content -LiteralPath $statsPath -Encoding UTF8 -Value $stats.stdout.Trim()
                }
                $nextStatsAt = (Get-Date).AddMilliseconds([int]$Contract.logging.stats_interval_ms)
            }
            Start-Sleep -Milliseconds 100
        }
        if (-not $timedOut -and $exitCode -ne 0) { $reasonCode = 'container_nonzero_exit' }
        $finishedAt = Get-Date
        [void](Invoke-TaskspaceDocker @('inspect', $containerId) -StdoutPath $inspectPath -StderrPath $stderrPath)
        [void](Invoke-TaskspaceDocker @('logs', '--timestamps', $containerId) -StdoutPath $stdoutPath -StderrPath $stderrPath)
        Write-TaskspaceContainerEvent $eventsPath "container.$Role`_completed" ($Identity + @{ role = $Role; container_id = $containerId; exit_code = $exitCode; timed_out = $timedOut; reason_code = $reasonCode })
        Add-TaskspaceContainerManifestEntry $manifestPath ([pscustomobject]@{
                schema_version = 1; role = $Role; container_id = $containerId; container_name = $name
                image_digest = [string]$Image.image_digest; docker_server_version = [string]$Image.docker_server_version
                run_id = [string]$Identity.run_id; sample_id = [string]$Identity.sample_id; pair_id = [string]$Identity.pair_id
                side = [string]$Identity.side; logical_mode = [string]$Identity.logical_mode
                container_workdir = [string]$Contract.paths.workspace; workspace_mount_mode = if ($WorkspaceReadOnly) { 'ro' } else { 'rw' }
                artifact_mount_mode = 'rw'; cpu_limit = [double]$Contract.resources.cpus; memory_limit = [int64]$Contract.resources.memory_bytes
                started_at = $startedAt.ToUniversalTime().ToString('o'); finished_at = $finishedAt.ToUniversalTime().ToString('o')
                duration_ms = [int64](($finishedAt - $startedAt).TotalMilliseconds); exit_code = $exitCode; timeout = $timedOut; reason_code = $reasonCode
            })
        [pscustomobject]@{
            exit_code = $exitCode; timed_out = $timedOut; reason_code = $reasonCode
            wall_time_ms = [int64](($finishedAt - $startedAt).TotalMilliseconds)
            container_id = $containerId; stdout_path = $stdoutPath; stderr_path = $stderrPath
            inspect_path = $inspectPath; stats_path = $statsPath; manifest_path = $manifestPath; lifecycle_path = $eventsPath
        }
    } catch {
        $message = [string]$_.Exception.Message
        $failureReason = @($Contract.reason_codes | Where-Object { $message.StartsWith("$_`:") } | Select-Object -First 1)
        if ($failureReason.Count -eq 0) { $failureReason = @('container_create_failed') }
        Write-TaskspaceContainerEvent $eventsPath "container.$Role`_failed" ($Identity + @{
                role = $Role
                container_id = $containerId
                reason_code = [string]$failureReason[0]
                error = $message
            })
        throw
    } finally {
        $cleanup = [ordered]@{ schema_version = 1; role = $Role; container_id = $containerId; removed = $false; reason_code = '' }
        if ($containerId) {
            $remove = Invoke-TaskspaceDocker @('rm', '--force', $containerId)
            $cleanup.removed = ($remove.exit_code -eq 0)
            if ($remove.exit_code -ne 0) { $cleanup.reason_code = 'container_cleanup_failed'; $cleanup.error = $remove.stderr.Trim() }
        } else { $cleanup.removed = $true }
        $cleanup.finished_at = (Get-Date).ToUniversalTime().ToString('o')
        Write-TaskspaceContainerJson ([pscustomobject]$cleanup) $cleanupPath
        Add-TaskspaceContainerManifestEntry $cleanupAggregatePath ([pscustomobject]$cleanup)
        Write-TaskspaceContainerEvent $eventsPath 'container.cleanup_completed' ($Identity + @{ role = $Role; container_id = $containerId; removed = [bool]$cleanup.removed; reason_code = [string]$cleanup.reason_code })
    }
}
