$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
. (Join-Path $repoRoot 'scripts/action-map-real-user-e2e-lib.ps1')
. (Join-Path $PSScriptRoot 'lib/container-contract.ps1')
. (Join-Path $PSScriptRoot 'lib/container-runtime.ps1')

$contract = Read-TaskspaceContainerContract $repoRoot
$image = Resolve-TaskspaceContainerImage $repoRoot $contract
$root = New-Dir (Join-Path $repoRoot ("target/container-runtime-selftest/{0}" -f ([guid]::NewGuid().ToString('N'))))
$workspace = New-Dir (Join-Path $root 'workspace')
$artifacts = New-Dir (Join-Path $root 'artifacts')
$identity = @{ run_id = 'selftest'; sample_id = 'container-runtime'; pair_id = 'pair-001'; side = 'left'; logical_mode = 'standard' }

$success = Invoke-TaskspaceContainerRole -Role validator -Image $image -Contract $contract -WorkspaceDir $workspace -ArtifactDir $artifacts -Command @('bash', '-lc', 'echo stdout-ok; echo stderr-ok >&2; echo artifact-ok > /artifacts/probe.txt') -TimeoutSeconds 20 -Identity $identity
if ($success.exit_code -ne 0 -or $success.timed_out) { throw 'Success fixture failed' }
if ((Get-Content -Raw -LiteralPath (Join-Path $artifacts 'probe.txt')).Trim() -ne 'artifact-ok') { throw 'Artifact mount failed' }
if ((Get-Content -Raw -LiteralPath $success.stdout_path) -notmatch 'stdout-ok') { throw 'stdout collection failed' }
if ((Get-Content -Raw -LiteralPath $success.stderr_path) -notmatch 'stderr-ok') { throw 'stderr collection failed' }
$manifest = @(Get-Content -Raw -LiteralPath $success.manifest_path | ConvertFrom-Json)[0]
if (-not $IsWindows -and [string]$manifest.container_user -ne "$(id -u):$(id -g)") {
    throw 'Container did not use the host uid/gid'
}

$failure = Invoke-TaskspaceContainerRole -Role validator -Image $image -Contract $contract -WorkspaceDir $workspace -ArtifactDir $artifacts -Command @('bash', '-lc', 'exit 17') -TimeoutSeconds 20 -Identity $identity
if ($failure.exit_code -ne 17 -or $failure.reason_code -ne 'container_nonzero_exit') { throw 'Failure fixture classification failed' }

$timeout = Invoke-TaskspaceContainerRole -Role validator -Image $image -Contract $contract -WorkspaceDir $workspace -ArtifactDir $artifacts -Command @('sleep', '10') -TimeoutSeconds 1 -Identity $identity
if ($timeout.exit_code -ne 124 -or -not $timeout.timed_out) { throw 'Timeout fixture classification failed' }
if ($timeout.wall_time_ms -gt 3000) { throw "Timeout enforcement was too late: $($timeout.wall_time_ms)ms" }

$stats = Invoke-TaskspaceContainerRole -Role validator -Image $image -Contract $contract -WorkspaceDir $workspace -ArtifactDir $artifacts -Command @('sleep', '6') -TimeoutSeconds 20 -Identity $identity
if ($stats.exit_code -ne 0 -or -not (Test-Path -LiteralPath $stats.stats_path)) { throw 'Stats fixture failed' }
if ([string]::IsNullOrWhiteSpace((Get-Content -Raw -LiteralPath $stats.stats_path))) { throw 'Stats fixture was empty' }

$invalidImage = $image | Select-Object *
$invalidImage.image_ref = '/home/private-user/secret-image'
$createFailed = $false
try {
    Invoke-TaskspaceContainerRole -Role validator -Image $invalidImage -Contract $contract -WorkspaceDir $workspace -ArtifactDir $artifacts -Command @('true') -TimeoutSeconds 10 -Identity $identity | Out-Null
} catch {
    $createFailed = ([string]$_.Exception.Message -match '^container_create_failed:')
}
if (-not $createFailed) { throw 'Create failure fixture was not classified' }
$failureEvents = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifacts 'container-lifecycle-events.jsonl')
if ($failureEvents.Contains('/home/private-user/secret-image')) { throw 'Container lifecycle persisted a raw host path' }

$secretValue = 'container-selftest-secret-value'
$secretPath = New-TaskspaceContainerSecret $root $secretValue
try {
    $secret = Invoke-TaskspaceContainerRole -Role agent -Image $image -Contract $contract -WorkspaceDir $workspace -ArtifactDir $artifacts -Command @('bash', '-lc', 'test -n "$DEEPSEEK_API_KEY"; echo secret-ok') -TimeoutSeconds 20 -Identity $identity -SecretPath $secretPath
    if ($secret.exit_code -ne 0) { throw 'Secret fixture failed' }
    $scan = Get-ChildItem -LiteralPath $artifacts -File | ForEach-Object { Get-Content -Raw -ErrorAction SilentlyContinue -LiteralPath $_.FullName }
    if (($scan -join "`n").Contains($secretValue)) { throw 'secret_leak_detected' }
} finally {
    Remove-TaskspaceContainerSecret $secretPath
}

$remaining = Invoke-TaskspaceDocker @('ps', '-aq', '--filter', 'label=whalecode.run_id=selftest')
if (-not [string]::IsNullOrWhiteSpace($remaining.stdout)) { throw 'Container cleanup left residual containers' }
if (-not (Test-Path -LiteralPath (Join-Path $artifacts 'container-cleanup-result.json'))) { throw 'Cleanup aggregate is missing' }
$events = Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $artifacts 'container-lifecycle-events.jsonl')
foreach ($event in @('container.created', 'container.preflight_passed', 'container.validator_completed', 'container.validator_failed', 'container.cleanup_completed')) {
    if (-not $events.Contains($event)) { throw "Lifecycle event missing: $event" }
}
Write-Host "container runtime tests passed image=$($image.image_digest)"
