function Read-TaskspaceContainerContract {
    param([Parameter(Mandatory = $true)][string]$RepoRoot)
    $path = Join-Path $RepoRoot "benchmarks/taskspace/container-runtime-contract.json"
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Container runtime contract not found: $path"
    }
    $contract = Get-Content -Raw -Encoding UTF8 -LiteralPath $path | ConvertFrom-Json
    Assert-TaskspaceContainerContract $contract
    $contract
}

function Assert-TaskspaceContainerContract {
    param([Parameter(Mandatory = $true)]$Contract)
    if ([int]$Contract.schema_version -ne 1) { throw "Unsupported container contract schema" }
    if ([string]$Contract.base_image -notmatch '@sha256:[0-9a-f]{64}$') {
        throw "Container base_image must be digest pinned"
    }
    foreach ($path in @(
            [string]$Contract.paths.workspace,
            [string]$Contract.paths.artifacts,
            [string]$Contract.paths.whale_binary,
            [string]$Contract.paths.provider_secret,
            [string]$Contract.paths.home
        )) {
        if (-not $path.StartsWith('/')) { throw "Container path must be absolute: $path" }
    }
    if ([double]$Contract.resources.cpus -le 0) { throw "Container cpus must be positive" }
    if ([int64]$Contract.resources.memory_bytes -lt 1073741824) { throw "Container memory is too low" }
    if ([int64]$Contract.resources.memory_swap_bytes -ne [int64]$Contract.resources.memory_bytes) {
        throw "Container swap policy must equal the memory limit"
    }
    if ([int]$Contract.logging.stats_interval_ms -lt 500) { throw "Stats interval is too aggressive" }
    foreach ($role in @('agent', 'validator', 'oracle')) {
        if (-not ($Contract.mount_policy.PSObject.Properties.Name -contains $role)) {
            throw "Container mount policy is missing role: $role"
        }
    }
    if ([string]$Contract.mount_policy.agent.oracle -ne 'none') {
        throw "Agent container must not receive an oracle mount"
    }
    if ([string]$Contract.mount_policy.oracle.oracle -ne 'ro') {
        throw "Oracle container requires a read-only oracle mount"
    }
    $requiredAgentOverrides = @('features.plugins=false')
    foreach ($override in $requiredAgentOverrides) {
        if (@($Contract.agent_config_overrides) -notcontains $override) {
            throw "Container agent config override is missing: $override"
        }
    }
    foreach ($forbiddenOverride in @(
            'skills.bundled.enabled=false',
            'skills.include_instructions=false'
        )) {
        if (@($Contract.agent_config_overrides) -contains $forbiddenOverride) {
            throw "Container benchmark must not disable product Skill behavior: $forbiddenOverride"
        }
    }
    $providerBoundary = $Contract.provider_boundary
    if ($null -eq $providerBoundary) { throw "Container provider boundary contract is missing" }
    $expectedProviderBoundary = [ordered]@{
        logical_provider_id = 'deepseek'
        transport_provider_id = 'deepseek-boundary'
        name = 'DeepSeek'
        base_url = 'http://provider-proxy:8080'
        env_key = 'DEEPSEEK_API_KEY'
        env_key_instructions = 'Set DEEPSEEK_API_KEY to a DeepSeek API key before starting Whale.'
        wire_api = 'responses'
    }
    foreach ($field in $expectedProviderBoundary.Keys) {
        if ([string]$providerBoundary.$field -cne [string]$expectedProviderBoundary[$field]) {
            throw "Container provider boundary $field is invalid"
        }
    }
    $intentionalDifferences = @($providerBoundary.intentional_differences | ForEach-Object { [string]$_ })
    if ($intentionalDifferences.Count -ne 2 -or
        $intentionalDifferences[0] -cne 'provider_id' -or
        $intentionalDifferences[1] -cne 'base_url') {
        throw "Container provider boundary intentional differences are invalid"
    }
    $requiredCodes = @(
        'docker_unavailable',
        'container_preflight_failed',
        'container_timeout',
        'container_cleanup_failed',
        'secret_leak_detected',
        'oracle_mount_leak_detected'
    )
    foreach ($code in $requiredCodes) {
        if (@($Contract.reason_codes) -notcontains $code) {
            throw "Container reason code is missing: $code"
        }
    }
}

function Get-TaskspaceProviderBoundaryRouteEvidence {
    param([Parameter(Mandatory = $true)]$Contract)
    $route = $Contract.provider_boundary
    [ordered]@{
        route_kind = 'custom_provider_transport_alias'
        logical_provider_id = [string]$route.logical_provider_id
        transport_provider_id = [string]$route.transport_provider_id
        provider_name = [string]$route.name
        base_url = [string]$route.base_url
        env_key = [string]$route.env_key
        wire_api = [string]$route.wire_api
        intentional_differences = @($route.intentional_differences | ForEach-Object { [string]$_ })
    }
}

function Get-TaskspaceProviderBoundaryConfigOverrides {
    param([Parameter(Mandatory = $true)]$Contract)
    $route = $Contract.provider_boundary
    @(
        "model_provider=`"$([string]$route.transport_provider_id)`""
        "model_providers.$([string]$route.transport_provider_id).name=`"$([string]$route.name)`""
        "model_providers.$([string]$route.transport_provider_id).base_url=`"$([string]$route.base_url)`""
        "model_providers.$([string]$route.transport_provider_id).env_key=`"$([string]$route.env_key)`""
        "model_providers.$([string]$route.transport_provider_id).env_key_instructions=`"$([string]$route.env_key_instructions)`""
        "model_providers.$([string]$route.transport_provider_id).wire_api=`"$([string]$route.wire_api)`""
    )
}

function Get-TaskspaceContainerResourceArgs {
    param([Parameter(Mandatory = $true)]$Contract)
    @(
        '--cpus', ([string]$Contract.resources.cpus),
        '--memory', ([string]$Contract.resources.memory_bytes),
        '--memory-swap', ([string]$Contract.resources.memory_swap_bytes),
        '--pids-limit', ([string]$Contract.resources.pids_limit)
    )
}

function Get-TaskspaceContainerLogArgs {
    param([Parameter(Mandatory = $true)]$Contract)
    @(
        '--log-driver', ([string]$Contract.logging.driver),
        '--log-opt', "max-size=$([string]$Contract.logging.max_size)",
        '--log-opt', "max-file=$([string]$Contract.logging.max_files)"
    )
}

function Get-TaskspaceContainerPermissionMatrix {
    param([Parameter(Mandatory = $true)]$Contract)
    @('agent', 'validator', 'oracle') | ForEach-Object {
        $role = $_
        $policy = $Contract.mount_policy.$role
        [pscustomobject]@{
            role = $role
            workspace = [string]$policy.workspace
            artifacts = [string]$policy.artifacts
            whale_binary = [string]$policy.whale_binary
            provider_secret = [string]$policy.provider_secret
            oracle = [string]$policy.oracle
        }
    }
}
