function New-TaskspaceContainerIdentity {
    param([string]$RunId, [string]$SampleId, [string]$PairId, $Side)
    @{
        run_id = $RunId
        sample_id = $SampleId
        pair_id = $PairId
        side = [string]$Side.Name
        logical_mode = [string]$Side.LogicalMode
    }
}

function Assert-TaskspaceDockerWhaleArgv {
    param([Parameter(Mandatory = $true)][string[]]$WhaleArgv)
    if ($WhaleArgv -contains '--full-auto' -or $WhaleArgv -contains '--sandbox') {
        throw 'container_agent_nested_sandbox_rejected: Docker is the hard sandbox boundary'
    }
    if ($WhaleArgv -notcontains '--dangerously-bypass-approvals-and-sandbox') {
        throw 'container_agent_bypass_required: Docker agent must not start a nested sandbox'
    }
}

function Invoke-TaskspaceDockerAgent {
    param(
        [string]$RunId,
        [string]$SampleId,
        [string]$PairId,
        $Side,
        $Image,
        $Contract,
        [string]$WhaleBin,
        [string[]]$WhaleArgv,
        [hashtable]$Environment,
        [string]$ProviderSecret,
        [int]$TimeoutSeconds
    )
    Assert-TaskspaceDockerWhaleArgv $WhaleArgv
    $identity = New-TaskspaceContainerIdentity $RunId $SampleId $PairId $Side
    $secretPath = New-TaskspaceContainerSecret $Side.ArtifactDir $ProviderSecret
    $script = @'
set +e
/opt/whale/whale "$@" < /artifacts/user-prompt.txt > /artifacts/whale-exec.jsonl 2> /artifacts/whale-exec.stderr.log
code=$?
whale_home="${WHALE_HOME:-${HOME}/.whale}"
if [[ -d "${whale_home}/sessions" ]]; then
    rollout="$(find "${whale_home}/sessions" -type f -name 'rollout*.jsonl' -printf '%T@ %p\n' | sort -nr | head -n 1 | cut -d' ' -f2-)"
    if [[ -n "${rollout}" ]]; then
        cp "${rollout}" /artifacts/rollout.jsonl
    fi
fi
exit "${code}"
'@
    $command = @('bash', '-lc', $script, 'taskspace-agent') + @($WhaleArgv)
    try {
        $result = Invoke-TaskspaceContainerRole -Role agent -Image $Image -Contract $Contract `
            -WorkspaceDir $Side.RepoDir -ArtifactDir $Side.ArtifactDir -Command $command `
            -TimeoutSeconds $TimeoutSeconds -Identity $identity -WhaleBin $WhaleBin `
            -SecretPath $secretPath -Environment $Environment
        $timingPath = Join-Path $Side.ArtifactDir 'process-timing.json'
        Write-TaskspaceContainerJson ([pscustomobject]@{
                schema_version = 1
                process_launch_wait_ms = 0
                wall_time_ms = [int64]$result.wall_time_ms
                timed_out = [bool]$result.timed_out
                completed = (-not [bool]$result.timed_out)
                exit_code = [int]$result.exit_code
                execution_substrate = 'docker'
                container_id = [string]$result.container_id
            }) $timingPath
        [pscustomobject]@{
            exit_code = [int]$result.exit_code
            timed_out = [bool]$result.timed_out
            reason_code = [string]$result.reason_code
            wall_time_ms = [int64]$result.wall_time_ms
            jsonl_path = Join-Path $Side.ArtifactDir 'whale-exec.jsonl'
            stderr_path = Join-Path $Side.ArtifactDir 'whale-exec.stderr.log'
            last_message_path = Join-Path $Side.ArtifactDir 'last-message.md'
            process_timing_path = $timingPath
            process_launch_wait_ms = 0
            container_id = [string]$result.container_id
        }
    } finally {
        Remove-TaskspaceContainerSecret $secretPath
    }
}

function Copy-TaskspaceAgentTurnArtifacts {
    param(
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [Parameter(Mandatory = $true)][string]$TurnName
    )
    $turnDir = New-Dir (Join-Path $ArtifactDir $TurnName)
    foreach ($name in @('whale-exec.jsonl', 'whale-exec.stderr.log', 'last-message.md', 'process-timing.json', 'rollout.jsonl')) {
        $source = Join-Path $ArtifactDir $name
        if (Test-Path -LiteralPath $source -PathType Leaf) {
            Copy-Item -LiteralPath $source -Destination (Join-Path $turnDir $name) -Force
        }
    }
    $turnDir
}

function Write-TaskspaceContinuationProtocol {
    param(
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [Parameter(Mandatory = $true)]$PreludeExec,
        [Parameter(Mandatory = $true)]$CombinedExec
    )
    $preludeWall = [int64]$PreludeExec.wall_time_ms
    $totalWall = [int64]$CombinedExec.wall_time_ms
    $continuationWall = [Math]::Max(0, $totalWall - $preludeWall)
    Write-TaskspaceContainerJson ([pscustomobject]@{
            schema_version = 'taskspace-live-continuation-v1'
            turns = 2
            prelude_exit_code = [int]$PreludeExec.exit_code
            prelude_timed_out = [bool]$PreludeExec.timed_out
            prelude_wall_time_ms = $preludeWall
            continuation_exit_code = [int]$CombinedExec.exit_code
            continuation_timed_out = [bool]$CombinedExec.timed_out
            continuation_wall_time_ms = $continuationWall
            total_wall_time_ms = $totalWall
            resume_mode = 'exec_resume_last'
        }) (Join-Path $ArtifactDir 'continuation-protocol.json')
    Write-TaskspaceContainerJson ([pscustomobject]@{
            schema_version = 1
            process_launch_wait_ms = 0
            wall_time_ms = $totalWall
            timed_out = [bool]$CombinedExec.timed_out
            completed = (-not [bool]$CombinedExec.timed_out)
            exit_code = [int]$CombinedExec.exit_code
            execution_substrate = 'docker_live_continuation'
        }) (Join-Path $ArtifactDir 'process-timing.json')
}

function Invoke-TaskspaceDockerValidation {
    param(
        [string]$RunId,
        [string]$SampleId,
        [string]$PairId,
        $Side,
        $Image,
        $Contract,
        $Validation,
        [int]$TimeoutSeconds,
        [string[]]$ExtraArgs = @(),
        [string]$ArtifactDir = ""
    )
    if ([string]::IsNullOrWhiteSpace($ArtifactDir)) { $ArtifactDir = [string]$Side.ArtifactDir }
    $containerSide = [pscustomobject]@{
        Name = [string]$Side.Name
        LogicalMode = [string]$Side.LogicalMode
        RepoDir = [string]$Side.RepoDir
        ArtifactDir = $ArtifactDir
    }
    $identity = New-TaskspaceContainerIdentity $RunId $SampleId $PairId $Side
    $validationArgv = @([string]$Validation.command) + @($Validation.args | ForEach-Object { [string]$_ }) + @($ExtraArgs)
    $script = @'
out=/artifacts/validation.stdout.log
err=/artifacts/validation.stderr.log
mkdir -p /artifacts/vrun
export TASKSPACE_VALIDATION_ARTIFACT_DIR=/artifacts/vrun
: > "$out"
: > "$err"
echo 'validator_lifecycle_stage=tests_started' >> "$out"
echo 'validator_tests_started=true' >> "$out"
"$@" >> "$out" 2>> "$err"
code=$?
echo 'validator_lifecycle_stage=tests_completed' >> "$out"
echo 'validator_tests_completed=true' >> "$out"
exit "$code"
'@
    $command = @('bash', '-lc', $script, 'taskspace-validator') + $validationArgv
    $result = Invoke-TaskspaceContainerRole -Role validator -Image $Image -Contract $Contract `
        -WorkspaceDir $containerSide.RepoDir -ArtifactDir $containerSide.ArtifactDir -Command $command `
        -TimeoutSeconds $TimeoutSeconds -Identity $identity
    [pscustomobject]@{
        exit_code = [int]$result.exit_code
        timed_out = [bool]$result.timed_out
        reason_code = [string]$result.reason_code
        stdout_path = Join-Path $containerSide.ArtifactDir 'validation.stdout.log'
        stderr_path = Join-Path $containerSide.ArtifactDir 'validation.stderr.log'
        wall_time_ms = [int64]$result.wall_time_ms
        container_id = [string]$result.container_id
    }
}

function Invoke-TaskspaceDockerOracle {
    param(
        [string]$RunId,
        [string]$SampleId,
        [string]$PairId,
        $Side,
        $Image,
        $Contract,
        [string]$OraclePath,
        [int]$TimeoutSeconds = 120
    )
    $identity = New-TaskspaceContainerIdentity $RunId $SampleId $PairId $Side
    $script = 'exec python /oracle/oracle.py /workspace > /artifacts/hidden-oracle.stdout.log 2> /artifacts/hidden-oracle.stderr.log'
    $result = Invoke-TaskspaceContainerRole -Role oracle -Image $Image -Contract $Contract `
        -WorkspaceDir $Side.RepoDir -ArtifactDir $Side.ArtifactDir `
        -Command @('bash', '-lc', $script) -TimeoutSeconds $TimeoutSeconds `
        -Identity $identity -OraclePath $OraclePath -WorkspaceReadOnly
    [pscustomobject]@{
        exit_code = [int]$result.exit_code
        stdout_path = Join-Path $Side.ArtifactDir 'hidden-oracle.stdout.log'
        stderr_path = Join-Path $Side.ArtifactDir 'hidden-oracle.stderr.log'
        oracle_sha256 = (Get-TaskspaceFileSha256 $OraclePath)
        oracle_isolation_level = 'hard_container_isolation'
        oracle_isolation_failure = $false
        leak = [pscustomobject]@{ leaked = $false; repo_hits = @(); artifact_hits = @() }
        container_id = [string]$result.container_id
    }
}

function Get-TaskspaceDockerOracleIsolationProbe {
    param($Side, [string]$CanaryPath, [string]$CanaryText)
    $inspectPath = Join-Path $Side.ArtifactDir 'container-inspect-agent.json'
    $oracleMounts = @()
    if (Test-Path -LiteralPath $inspectPath) {
        $inspect = @(Get-Content -Raw -Encoding UTF8 -LiteralPath $inspectPath | ConvertFrom-Json)[0]
        $oracleMounts = @($inspect.Mounts | Where-Object { [string]$_.Destination -like '/oracle*' })
    }
    $combined = @(
        Join-Path $Side.ArtifactDir 'whale-exec.jsonl'
        Join-Path $Side.ArtifactDir 'whale-exec.stderr.log'
        Join-Path $Side.ArtifactDir 'last-message.md'
    ) | Where-Object { Test-Path -LiteralPath $_ } | ForEach-Object { Get-Content -Raw -Encoding UTF8 -LiteralPath $_ }
    $text = $combined -join "`n"
    $leaked = $oracleMounts.Count -gt 0 -or $text.Contains($CanaryText)
    [pscustomobject]@{
        exit_code = 0
        canary_path = $CanaryPath
        canary_leaked = $leaked
        canary_materialized_during_probe = (Test-Path -LiteralPath $CanaryPath)
        path_mentioned = $text.Contains($CanaryPath)
        timed_out = $false
        oracle_isolation_level = if ($leaked) { 'failed' } else { 'hard_container_isolation' }
        agent_oracle_mount_count = $oracleMounts.Count
    }
}
