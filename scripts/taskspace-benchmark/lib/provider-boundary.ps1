$script:TaskspaceProviderProxyScript = (Resolve-Path (Join-Path $PSScriptRoot '../docker/provider_boundary_proxy.py')).Path
$script:TaskspaceProviderVerifierScript = (Resolve-Path (Join-Path $PSScriptRoot '../docker/verify_provider_boundary.py')).Path

function Start-TaskspaceProviderBoundary {
    param(
        [string]$RunId,
        [string]$SampleId,
        [string]$PairId,
        $Side,
        $Image,
        [string]$ProviderSecret,
        [int]$RequestHardLimit,
        [string]$Model,
        [string]$ProviderBaseUrl = 'https://api.deepseek.com'
    )
    if ($RequestHardLimit -lt 1) { throw 'provider_boundary_limit_invalid: hard limit must be positive' }
    if ($Model -notmatch '^deepseek-') {
        throw "provider_boundary_model_mismatch: provider boundary requires a DeepSeek model, got $Model"
    }
    $suffix = [guid]::NewGuid().ToString('N').Substring(0, 8)
    $prefix = "whale-provider-$RunId-$PairId-$($Side.Name)-$suffix" -replace '[^a-zA-Z0-9_.-]', '-'
    $internalNetwork = "$prefix-internal"
    $egressNetwork = "$prefix-egress"
    $containerName = "$prefix-proxy"
    $supervisorDir = New-Dir "$($Side.ArtifactDir).provider-supervisor"
    $secretPath = New-TaskspaceContainerSecret $supervisorDir $ProviderSecret
    $identityLabels = @(
        '--label', "whalecode.run_id=$RunId",
        '--label', "whalecode.sample_id=$SampleId",
        '--label', "whalecode.pair_id=$PairId",
        '--label', "whalecode.side=$($Side.Name)",
        '--label', 'whalecode.role=provider-boundary'
    )
    $containerId = ''
    try {
        $internalArgs = @('network', 'create', '--internal') + $identityLabels + @($internalNetwork)
        $internal = Invoke-TaskspaceDocker $internalArgs
        if ($internal.exit_code -ne 0) { throw "provider_boundary_network_failed: $($internal.stderr.Trim())" }
        $egressArgs = @('network', 'create') + $identityLabels + @($egressNetwork)
        $egress = Invoke-TaskspaceDocker $egressArgs
        if ($egress.exit_code -ne 0) { throw "provider_boundary_network_failed: $($egress.stderr.Trim())" }
        $createArgs = @('create', '--name', $containerName, '--network', $internalNetwork, '--network-alias', 'provider-proxy')
        $createArgs += $identityLabels
        $createArgs += @(Get-TaskspaceContainerMountArg $script:TaskspaceProviderProxyScript '/opt/whale/provider_boundary_proxy.py' -ReadOnly)
        $createArgs += @(Get-TaskspaceContainerMountArg $supervisorDir '/supervisor')
        $createArgs += @(Get-TaskspaceContainerMountArg $secretPath '/run/secrets/deepseek_api_key' -ReadOnly)
        $createArgs += @(
            '--env', "PROVIDER_REQUEST_HARD_LIMIT=$RequestHardLimit",
            '--env', "PROVIDER_ALLOWED_MODEL=$Model",
            '--env', "PROVIDER_UPSTREAM_BASE_URL=$ProviderBaseUrl",
            '--env', 'PROVIDER_BOUNDARY_EVENTS_PATH=/supervisor/events.jsonl',
            [string]$Image.image_ref,
            'python', '/opt/whale/provider_boundary_proxy.py'
        )
        $create = Invoke-TaskspaceDocker $createArgs
        if ($create.exit_code -ne 0) { throw "provider_boundary_create_failed: $($create.stderr.Trim())" }
        $containerId = $create.stdout.Trim()
        $connect = Invoke-TaskspaceDocker @('network', 'connect', $egressNetwork, $containerId)
        if ($connect.exit_code -ne 0) { throw "provider_boundary_network_failed: $($connect.stderr.Trim())" }
        $start = Invoke-TaskspaceDocker @('start', $containerId)
        if ($start.exit_code -ne 0) { throw "provider_boundary_start_failed: $($start.stderr.Trim())" }
        $ready = $false
        for ($attempt = 0; $attempt -lt 50; $attempt++) {
            $health = Invoke-TaskspaceDocker @('exec', $containerId, 'python', '-c', 'import urllib.request; urllib.request.urlopen("http://127.0.0.1:8080/healthz", timeout=1).read()')
            if ($health.exit_code -eq 0) { $ready = $true; break }
            Start-Sleep -Milliseconds 100
        }
        if (-not $ready) { throw 'provider_boundary_health_failed: proxy did not become ready' }
        [pscustomobject]@{
            container_id = $containerId
            container_name = $containerName
            internal_network = $internalNetwork
            egress_network = $egressNetwork
            supervisor_dir = $supervisorDir
            secret_path = $secretPath
            proxy_base_url = 'http://provider-proxy:8080'
            request_hard_limit = $RequestHardLimit
            expected_model = $Model
        }
    } catch {
        if ($containerId) { [void](Invoke-TaskspaceDocker @('rm', '--force', $containerId)) }
        foreach ($network in @($internalNetwork, $egressNetwork)) { [void](Invoke-TaskspaceDocker @('network', 'rm', $network)) }
        Remove-TaskspaceContainerSecret $secretPath
        throw
    }
}

function Stop-TaskspaceProviderBoundary {
    param($Boundary, [string]$ArtifactDir)
    if ($null -eq $Boundary) { return $null }
    $stdoutPath = Join-Path $Boundary.supervisor_dir 'container.stdout.log'
    $stderrPath = Join-Path $Boundary.supervisor_dir 'container.stderr.log'
    [void](Invoke-TaskspaceDocker @('logs', $Boundary.container_id) -StdoutPath $stdoutPath -StderrPath $stderrPath)
    $remove = Invoke-TaskspaceDocker @('rm', '--force', $Boundary.container_id)
    $networkResults = @()
    foreach ($network in @($Boundary.internal_network, $Boundary.egress_network)) {
        $networkResults += Invoke-TaskspaceDocker @('network', 'rm', $network)
    }
    Remove-TaskspaceContainerSecret $Boundary.secret_path
    $eventsPath = Join-Path $ArtifactDir 'provider-boundary-events.jsonl'
    $supervisorEventsPath = Join-Path $Boundary.supervisor_dir 'events.jsonl'
    if (Test-Path -LiteralPath $supervisorEventsPath) {
        Copy-Item -LiteralPath $supervisorEventsPath -Destination $eventsPath -Force
    } else {
        [System.IO.File]::WriteAllText($eventsPath, '', [System.Text.UTF8Encoding]::new($false))
    }
    $evidencePath = Join-Path $ArtifactDir 'provider-boundary-evidence.json'
    $wirePath = Join-Path $ArtifactDir 'provider-wire-trace.jsonl'
    & python $script:TaskspaceProviderVerifierScript --events $eventsPath --wire $wirePath --model ([string]$Boundary.expected_model) --output $evidencePath
    $reconcileExit = $LASTEXITCODE
    $result = [pscustomobject]@{
        schema_version = 1
        status = if ($remove.exit_code -eq 0 -and @($networkResults | Where-Object { $_.exit_code -ne 0 }).Count -eq 0) { 'removed' } else { 'cleanup_failed' }
        container_id = [string]$Boundary.container_id
        request_hard_limit = [int]$Boundary.request_hard_limit
        events_path = [string]$eventsPath
        evidence_path = [string]$evidencePath
        evidence_status = if ($reconcileExit -eq 0) { 'reconciled' } else { 'mismatch' }
        stdout_path = [string]$stdoutPath
        stderr_path = [string]$stderrPath
    }
    Write-TaskspaceContainerJson $result (Join-Path $ArtifactDir 'provider-boundary-result.json')
    $result
}
