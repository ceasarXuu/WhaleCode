param(
    [Parameter(Mandatory = $true)][string]$WhaleBin,
    [Parameter(Mandatory = $true)][string]$Model,
    [Parameter(Mandatory = $true)][string]$OutputPath,
    [Parameter(Mandatory = $true)][string]$RunId,
    [Parameter(Mandatory = $true)][string]$DescriptorKeyPath,
    [Parameter(Mandatory = $true)][string]$TransientDir,
    [int]$TimeoutSeconds = 60
)

$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
. (Join-Path $repoRoot 'scripts/action-map-real-user-e2e-lib.ps1')
. (Join-Path $PSScriptRoot 'lib/container-contract.ps1')
. (Join-Path $PSScriptRoot 'lib/container-runtime.ps1')
. (Join-Path $PSScriptRoot 'lib/container-benchmark-runner.ps1')

function Get-ProviderRouteTextSha256 {
    param([Parameter(Mandatory = $true)][string]$Text)
    $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
    $sha = [Security.Cryptography.SHA256]::Create()
    try { ([BitConverter]::ToString($sha.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}

function Get-ProviderRouteHmacSha256 {
    param(
        [Parameter(Mandatory = $true)][byte[]]$Key,
        [Parameter(Mandatory = $true)][string]$Text
    )
    $hmac = [Security.Cryptography.HMACSHA256]::new($Key)
    try {
        $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
        ([BitConverter]::ToString($hmac.ComputeHash($bytes))).Replace('-', '').ToLowerInvariant()
    } finally { $hmac.Dispose() }
}

function Get-ProviderRouteFileSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)
    (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}

function Assert-ResolvedProviderShape {
    param($Resolved, [string]$ExpectedProviderId, [string]$ExpectedModel)
    if ([string]$Resolved.schema_version -cne 'whalecode-resolved-provider-v1' -or
        [string]$Resolved.model_provider_id -cne $ExpectedProviderId -or
        [string]$Resolved.model -cne $ExpectedModel -or
        [string]$Resolved.provider_descriptor_sha256 -notmatch '^[0-9a-f]{64}$') {
        throw 'provider_route_resolution_mismatch: resolved provider does not match the route contract'
    }
    $providerJson = $Resolved.provider | ConvertTo-Json -Depth 20 -Compress
    if ([string]$Resolved.provider_descriptor_sha256 -cne (Get-ProviderRouteTextSha256 $providerJson)) {
        throw 'provider_route_descriptor_mismatch: resolved provider descriptor is not self-consistent'
    }
}

function Assert-ProviderAliasEquivalent {
    param($Alias, $Builtin, $Route, [byte[]]$DescriptorKey)
    Assert-ResolvedProviderShape $Alias ([string]$Route.transport_provider_id) $Model
    Assert-ResolvedProviderShape $Builtin ([string]$Route.logical_provider_id) $Model
    if ([string]$Alias.provider.name -cne [string]$Route.name -or
        [string]$Alias.provider.env_key -cne [string]$Route.env_key -or
        [string]$Alias.provider.wire_api -cne [string]$Route.wire_api -or
        -not [bool]$Alias.provider.is_deepseek -or
        -not [bool]$Builtin.provider.is_deepseek -or
        [string]$Alias.provider.base_url_hmac_sha256 -cne (Get-ProviderRouteHmacSha256 $DescriptorKey ([string]$Route.base_url)) -or
        [string]$Builtin.provider.base_url_hmac_sha256 -cne (Get-ProviderRouteHmacSha256 $DescriptorKey 'https://api.deepseek.com')) {
        throw 'provider_route_resolution_mismatch: provider identity does not match DeepSeek'
    }
    $normalizedAlias = ($Alias.provider | ConvertTo-Json -Depth 20 | ConvertFrom-Json)
    $normalizedAlias.base_url_hmac_sha256 = [string]$Builtin.provider.base_url_hmac_sha256
    if (($normalizedAlias | ConvertTo-Json -Depth 20 -Compress) -cne
        ($Builtin.provider | ConvertTo-Json -Depth 20 -Compress)) {
        throw 'provider_route_behavior_drift: alias differs from built-in DeepSeek beyond provider id and base URL'
    }
}

$artifactDir = Split-Path -Parent $OutputPath
if ([string]::IsNullOrWhiteSpace($artifactDir)) { throw 'OutputPath must include a parent directory' }
New-Item -ItemType Directory -Force -Path $artifactDir | Out-Null
$rawInspectPaths = [Collections.Generic.List[string]]::new()
$attestation = [ordered]@{
    schema_version = 'whalecode-provider-route-preflight-v1'
    status = 'failed'
    model = $Model
    preflight_started_at = (Get-Date).ToUniversalTime().ToString('o')
    operation = 'config_resolution_only'
    network_mode = 'none'
}

try {
    if ($Model -notmatch '^deepseek-') {
        throw "provider_route_model_mismatch: provider boundary requires a DeepSeek model, got $Model"
    }
    if (-not (Test-Path -LiteralPath $WhaleBin -PathType Leaf)) {
        throw "provider_route_whale_missing: Whale binary not found: $WhaleBin"
    }
    $WhaleBin = (Resolve-Path -LiteralPath $WhaleBin).Path
    if (-not (Test-Path -LiteralPath $DescriptorKeyPath -PathType Leaf)) {
        throw 'provider_route_descriptor_key_missing: caller-owned HMAC key file is missing'
    }
    $DescriptorKeyPath = (Resolve-Path -LiteralPath $DescriptorKeyPath).Path
    if (-not (Test-Path -LiteralPath $TransientDir -PathType Container)) {
        throw 'provider_route_transient_dir_missing: caller-owned transient directory is missing'
    }
    $descriptorKeyMaterial = [IO.File]::ReadAllBytes($DescriptorKeyPath)
    if ($descriptorKeyMaterial.Count -eq 0) {
        throw 'provider_route_descriptor_key_empty: caller-owned HMAC key is empty'
    }
    $contract = Read-TaskspaceContainerContract $repoRoot
    $route = $contract.provider_boundary
    $routeEvidence = Get-TaskspaceProviderBoundaryRouteEvidence $contract
    $baseOverrides = @($contract.agent_config_overrides | ForEach-Object { [string]$_ })
    $baseOverrides += @(Get-TaskspaceProviderBoundaryConfigOverrides $contract)
    $image = Resolve-TaskspaceContainerImage $repoRoot $contract
    if ($RunId -notmatch '^provider-route-preflight-[0-9a-f]{12}$') {
        throw 'provider_route_run_id_invalid: caller-owned cleanup label is invalid'
    }
    $identityBase = $RunId
    $profiles = New-Object System.Collections.Generic.List[object]

    foreach ($profile in @('standard', 'taskspace')) {
        $overrides = @($baseOverrides)
        if ($profile -eq 'taskspace') {
            $overrides += @(
                'features.multi_agent_v2.enabled=true',
                'taskspace_projection_policy="map-request"'
            )
        }
        $resolvedName = "resolved-provider-$profile.json"
        $builtinName = "builtin-provider-$profile.json"
        $resolvedValues = @{}
        foreach ($kind in @('alias', 'builtin')) {
            $artifactName = if ($kind -eq 'alias') { $resolvedName } else { $builtinName }
            $effectiveOverrides = @($overrides)
            if ($kind -eq 'builtin') { $effectiveOverrides += 'model_provider="deepseek"' }
            $argv = @('-m', $Model, '-C', [string]$contract.paths.workspace, 'debug', 'provider')
            foreach ($override in $effectiveOverrides) { $argv += @('-c', $override) }
            $stderrName = "$($artifactName).stderr.log"
            $script = "/opt/whale/whale `"`$@`" > /artifacts/$artifactName 2> /artifacts/$stderrName"
            $side = [pscustomobject]@{
                Name = "$profile-$kind"
                LogicalMode = $profile
                RepoDir = $repoRoot
                ArtifactDir = $artifactDir
            }
            $result = Invoke-TaskspaceContainerRole -Role agent -Image $image -Contract $contract `
                -WorkspaceDir $repoRoot -ArtifactDir $artifactDir `
                -Command (@('bash', '-lc', $script, 'provider-route-preflight') + $argv) `
                -TimeoutSeconds $TimeoutSeconds -Identity (New-TaskspaceContainerIdentity $identityBase 'cache-route' 'preflight' $side) `
                -WhaleBin $WhaleBin -SecretPath $DescriptorKeyPath -NetworkName 'none' `
                -InspectDirectory $TransientDir `
                -Environment @{ WHALE_PROVIDER_DESCRIPTOR_HMAC_KEY_FILE = [string]$contract.paths.provider_secret } `
                -WorkspaceReadOnly
            $inspectName = "container-inspect-$profile-$kind.json"
            $inspectPath = Join-Path $artifactDir $inspectName
            $rawInspectPath = [string]$result.inspect_path
            $rawInspectPaths.Add($rawInspectPath)
            try {
                $inspect = @(Get-Content -Raw -Encoding UTF8 -LiteralPath $rawInspectPath | ConvertFrom-Json)[0]
                $networkMode = [string]$inspect.HostConfig.NetworkMode
                $secretMount = Get-TaskspaceProviderSecretMountEvidence @($inspect.Mounts) $DescriptorKeyPath ([string]$contract.paths.provider_secret)
                $secretMounted = [bool]$secretMount.destination_unique
                $secretReadOnly = [bool]$secretMount.read_only
                $secretSourceMountUnique = [bool]$secretMount.source_unique
                $secretMountIdentityConfirmed = [bool]$secretMount.identity_confirmed
                $descriptorKeyEnv = "WHALE_PROVIDER_DESCRIPTOR_HMAC_KEY_FILE=$([string]$contract.paths.provider_secret)"
                $descriptorKeyEnvPresent = @($inspect.Config.Env | Where-Object { [string]$_ -ceq $descriptorKeyEnv }).Count -eq 1
                $workspaceReadOnly = @($inspect.Mounts | Where-Object { [string]$_.Destination -eq [string]$contract.paths.workspace -and [bool]$_.RW -eq $false }).Count -eq 1
                [ordered]@{
                    schema_version = 'whalecode-provider-route-container-inspect-v1'
                    profile = $profile
                    provider_kind = $kind
                    network_mode = $networkMode
                    workspace_read_only = $workspaceReadOnly
                    descriptor_key_secret_mounted = $secretMounted
                    descriptor_key_read_only = $secretReadOnly
                    descriptor_key_source_mount_unique = $secretSourceMountUnique
                    descriptor_key_mount_identity_confirmed = $secretMountIdentityConfirmed
                    descriptor_key_env_file = if ($descriptorKeyEnvPresent) { [string]$contract.paths.provider_secret } else { $null }
                } | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $inspectPath -Encoding UTF8
            } finally {
                if (Test-Path -LiteralPath $rawInspectPath) {
                    Remove-Item -LiteralPath $rawInspectPath -Force
                }
            }
            if ([int]$result.exit_code -ne 0 -or [bool]$result.timed_out) {
                throw "provider_route_cli_failed: $profile $kind debug provider exited $([int]$result.exit_code)"
            }
            $resolvedPath = Join-Path $artifactDir $artifactName
            if (-not (Test-Path -LiteralPath $resolvedPath -PathType Leaf)) {
                throw "provider_route_output_missing: $artifactName"
            }
            if ($networkMode -cne 'none' -or -not $secretMounted -or -not $secretReadOnly -or -not $secretSourceMountUnique -or -not $secretMountIdentityConfirmed -or -not $descriptorKeyEnvPresent -or -not $workspaceReadOnly) {
                throw "provider_route_network_mismatch: $profile $kind container did not use network=none"
            }
            $resolvedValues[$kind] = [pscustomobject]@{
                artifact = $artifactName
                artifact_sha256 = Get-ProviderRouteFileSha256 $resolvedPath
                argv_sha256 = Get-ProviderRouteTextSha256 ($argv -join "`n")
                config_overrides_sha256 = Get-ProviderRouteTextSha256 ($effectiveOverrides -join "`n")
                inspect_artifact = $inspectName
                inspect_artifact_sha256 = Get-ProviderRouteFileSha256 $inspectPath
                value = Get-Content -Raw -Encoding UTF8 -LiteralPath $resolvedPath | ConvertFrom-Json
            }
        }
        $resolved = $resolvedValues.alias.value
        $builtin = $resolvedValues.builtin.value
        Assert-ProviderAliasEquivalent $resolved $builtin $route $descriptorKeyMaterial
        $profiles.Add([pscustomobject]@{
                profile = $profile
                projection_policy = if ($profile -eq 'taskspace') { 'map-request' } else { $null }
                multi_agent_v2_enabled = ($profile -eq 'taskspace')
                config_overrides_sha256 = [string]$resolvedValues.alias.config_overrides_sha256
                argv_sha256 = [string]$resolvedValues.alias.argv_sha256
                resolved_provider_artifact = $resolvedName
                resolved_provider_artifact_sha256 = [string]$resolvedValues.alias.artifact_sha256
                provider_descriptor_sha256 = [string]$resolved.provider_descriptor_sha256
                builtin_provider_artifact = $builtinName
                builtin_provider_artifact_sha256 = [string]$resolvedValues.builtin.artifact_sha256
                builtin_provider_descriptor_sha256 = [string]$builtin.provider_descriptor_sha256
                builtin_config_overrides_sha256 = [string]$resolvedValues.builtin.config_overrides_sha256
                builtin_argv_sha256 = [string]$resolvedValues.builtin.argv_sha256
                container_inspect_artifact = [string]$resolvedValues.alias.inspect_artifact
                container_inspect_artifact_sha256 = [string]$resolvedValues.alias.inspect_artifact_sha256
                builtin_container_inspect_artifact = [string]$resolvedValues.builtin.inspect_artifact
                builtin_container_inspect_artifact_sha256 = [string]$resolvedValues.builtin.inspect_artifact_sha256
                equivalent_to_builtin_deepseek = $true
            })
    }
    if ([string]$profiles[0].provider_descriptor_sha256 -cne [string]$profiles[1].provider_descriptor_sha256) {
        throw 'provider_route_profile_drift: Standard and TaskSpace resolved different providers'
    }
    $attestation.provider_routing = $routeEvidence
    $attestation.whale_binary_sha256 = Get-ProviderRouteFileSha256 $WhaleBin
    $attestation.container_image_digest = [string]$image.image_digest
    $attestation.provider_descriptor_sha256 = [string]$profiles[0].provider_descriptor_sha256
    $attestation.profiles = @($profiles | ForEach-Object { $_ })
    $attestation.preflight_completed_at = (Get-Date).ToUniversalTime().ToString('o')
    $attestation.status = 'passed'
} catch {
    $message = [string]$_.Exception.Message
    $attestation.status = 'failed'
    $attestation.preflight_completed_at = (Get-Date).ToUniversalTime().ToString('o')
    $attestation.failure_code = ($message -split ':', 2)[0]
    $attestation.failure_summary = 'provider route preflight failed before authorization claim'
    $attestation | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
    [Console]::Error.WriteLine($message)
    exit 1
} finally {
    foreach ($rawInspectPath in $rawInspectPaths) {
        if (-not [string]::IsNullOrWhiteSpace($rawInspectPath) -and (Test-Path -LiteralPath $rawInspectPath)) {
            Remove-Item -LiteralPath $rawInspectPath -Force
        }
    }
}

$attestation | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
Write-Host "ProviderRoutePreflight: $OutputPath"
