$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
. (Join-Path $PSScriptRoot 'lib/bootstrap.ps1') -RepoRoot $repoRoot -BenchmarkRoot $PSScriptRoot

$contract = Read-TaskspaceContainerContract $repoRoot
$image = Resolve-TaskspaceContainerImage $repoRoot $contract
$root = New-Dir (Join-Path $repoRoot ("target/container-benchmark-runner-selftest/{0}" -f ([guid]::NewGuid().ToString('N'))))
$workspace = New-Dir (Join-Path $root 'workspace')
$artifacts = New-Dir (Join-Path $root 'artifacts')
$side = [pscustomobject]@{ Name = 'left'; LogicalMode = 'standard'; RepoDir = $workspace; ArtifactDir = $artifacts }
$identity = New-TaskspaceContainerIdentity 'selftest' 'benchmark-runner' 'pair-001' $side

$bypassArgv = New-TaskspaceWhaleArgv 'standard' 'model-x' '/workspace' '/artifacts/last-message.md'
Assert-TaskspaceDockerWhaleArgv $bypassArgv
$resumeArgv = New-TaskspaceWhaleResumeArgv 'taskspace' 'model-x' '/artifacts/last-message.md'
Assert-TaskspaceDockerWhaleArgv $resumeArgv
if (($resumeArgv -join ' ') -notmatch '^exec resume --last --json --taskspace') { throw 'TaskSpace resume argv is malformed' }
$nestedSandboxRejected = $false
try {
    $nestedArgv = @($bypassArgv | Where-Object { $_ -ne '--dangerously-bypass-approvals-and-sandbox' }) + @('--full-auto')
    Assert-TaskspaceDockerWhaleArgv $nestedArgv
} catch {
    $nestedSandboxRejected = [string]$_.Exception.Message -match '^container_agent_nested_sandbox_rejected:'
}
if (-not $nestedSandboxRejected) { throw 'Docker agent accepted a nested sandbox argv' }

$agentContainer = Invoke-TaskspaceContainerRole -Role agent -Image $image -Contract $contract -WorkspaceDir $workspace -ArtifactDir $artifacts -Command @('true') -TimeoutSeconds 10 -Identity $identity
if ($agentContainer.exit_code -ne 0) { throw 'Agent isolation fixture failed' }
$probe = Get-TaskspaceDockerOracleIsolationProbe $side (Join-Path $root 'canary') 'private-canary-value'
if ($probe.oracle_isolation_level -ne 'hard_container_isolation' -or $probe.agent_oracle_mount_count -ne 0) {
    throw 'Agent oracle isolation probe failed'
}

$validation = [pscustomobject]@{ command = 'python'; args = @('--version') }
$validator = Invoke-TaskspaceDockerValidation 'selftest' 'benchmark-runner' 'pair-001' $side $image $contract $validation 20
if ($validator.exit_code -ne 0) { throw 'Validator wrapper failed' }
if ((Get-Content -Raw -LiteralPath $validator.stdout_path) -notmatch 'validator_tests_completed=true') {
    throw 'Validator lifecycle output is incomplete'
}

$oraclePath = Join-Path $root 'oracle.py'
Write-Text $oraclePath "import pathlib, sys`nassert pathlib.Path(sys.argv[1]).is_dir()`nprint('oracle-ok')`n"
$oracle = Invoke-TaskspaceDockerOracle 'selftest' 'benchmark-runner' 'pair-001' $side $image $contract $oraclePath 20
if ($oracle.exit_code -ne 0 -or $oracle.oracle_isolation_level -ne 'hard_container_isolation') {
    throw 'Oracle wrapper failed'
}
if ((Get-Content -Raw -LiteralPath $oracle.stdout_path) -notmatch 'oracle-ok') { throw 'Oracle output missing' }

Write-Text (Join-Path $artifacts 'whale-exec.jsonl') "{}`n"
Write-Text (Join-Path $artifacts 'rollout.jsonl') "{}`n"
$preludeDir = Copy-TaskspaceAgentTurnArtifacts $artifacts 'prelude'
if (-not (Test-Path -LiteralPath (Join-Path $preludeDir 'rollout.jsonl'))) { throw 'Prelude rollout was not preserved' }
$preludeExec = [pscustomobject]@{ exit_code = 0; timed_out = $false; wall_time_ms = 120 }
$combinedExec = [pscustomobject]@{ exit_code = 0; timed_out = $false; wall_time_ms = 200 }
Write-TaskspaceContinuationProtocol $artifacts $preludeExec $combinedExec
$protocol = Get-Content -Raw -LiteralPath (Join-Path $artifacts 'continuation-protocol.json') | ConvertFrom-Json
if ($protocol.continuation_wall_time_ms -ne 80 -or $protocol.total_wall_time_ms -ne 200) { throw 'Continuation timing was not partitioned' }

$persistedRollout = Join-Path $artifacts 'rollout.jsonl'
Write-Text $persistedRollout "{}`n"
$rolloutSource = Resolve-TaskspaceRolloutSource $artifacts (Get-Date).AddHours(1) 'container-thread'
if ([string]$rolloutSource.FullName -ne [string](Get-Item -LiteralPath $persistedRollout).FullName) {
    throw 'Container rollout was not preferred as the observability source'
}

$manifests = @(Get-Content -Raw -LiteralPath (Join-Path $artifacts 'container-runtime-manifest.json') | ConvertFrom-Json)
if (@($manifests | Where-Object { $_.role -eq 'agent' }).Count -ne 1) { throw 'Agent manifest missing' }
if (@($manifests | Where-Object { $_.role -eq 'validator' }).Count -ne 1) { throw 'Validator manifest missing' }
if (@($manifests | Where-Object { $_.role -eq 'oracle' -and $_.workspace_mount_mode -eq 'ro' }).Count -ne 1) { throw 'Oracle manifest missing or writable' }

Write-Host "container benchmark runner tests passed"
