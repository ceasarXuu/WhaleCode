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
