function Get-R7ResolvedIdentityProperty {
    param($Object, [string]$Name, $Default = $null)
    if ($null -ne $Object -and $Object.PSObject.Properties.Name -contains $Name) {
        return $Object.$Name
    }
    $Default
}

function Test-R7Sha256Text {
    param($Value)
    $Value -is [string] -and [string]$Value -match '^[a-fA-F0-9]{64}$'
}

function Get-R7ResolvedManifestIdentityCheck {
    param(
        [Parameter(Mandatory = $true)]$Authority,
        [Parameter(Mandatory = $true)]$MatrixManifest,
        [Parameter(Mandatory = $true)]$Run,
        [Parameter(Mandatory = $true)]$Resolved,
        [Parameter(Mandatory = $true)]$WireCapability
    )
    $findings = [Collections.Generic.List[string]]::new()
    $side = [string](Get-R7ResolvedIdentityProperty $Run "run_side" "")
    $mode = [string](Get-R7ResolvedIdentityProperty $Run "logical_mode" "")
    $arm = [string](Get-R7ResolvedIdentityProperty $Run "arm" "")
    $projection = [string](Get-R7ResolvedIdentityProperty $Run "projection_policy" "")
    $sample = [string](Get-R7ResolvedIdentityProperty $Run "sample" "")
    $repeat = [int](Get-R7ResolvedIdentityProperty $Run "repeat" 0)
    $binarySha = (
        [string](Get-R7ResolvedIdentityProperty $MatrixManifest "whale_sha256" "")
    ).ToLowerInvariant()
    $logicalModeMap = Get-R7ResolvedIdentityProperty $Resolved "logical_mode_map"
    $provider = Get-R7ResolvedIdentityProperty $Resolved "provider_param_status"
    $explicit = Get-R7ResolvedIdentityProperty $provider "explicit"
    $selectedSides = @(Get-R7ResolvedIdentityProperty $Resolved "selected_sides" @())
    $promptLeft = [string](Get-R7ResolvedIdentityProperty $Resolved "prompt_sha256_left" "")
    $promptRight = [string](Get-R7ResolvedIdentityProperty $Resolved "prompt_sha256_right" "")
    $fixtureLeft = [string](Get-R7ResolvedIdentityProperty $Resolved "fixture_sha256_left" "")
    $fixtureRight = [string](Get-R7ResolvedIdentityProperty $Resolved "fixture_sha256_right" "")
    $modelField = "model_$side"
    $binaryField = "whale_sha256_$side"
    $expectedMode = if ($side -eq "left") { "standard" } elseif ($side -eq "right") {
        "taskspace"
    } else {
        ""
    }
    $expectedProjection = if ($expectedMode -eq "standard") { "map-request" } else { $arm }
    $scenarioIdentity = if ($Authority.scenarios.ContainsKey($sample)) {
        $Authority.scenarios[$sample]
    } else {
        $null
    }
    $profile = Get-R7ResolvedIdentityProperty `
        $Authority.tool_capability_profiles `
        $expectedMode

    if ($sample -notin @($Authority.samples) -or
        $repeat -lt 1 -or
        $repeat -gt [int]$Authority.repeats -or
        [string](Get-R7ResolvedIdentityProperty $Resolved "scenario" "") -ne $sample -or
        [int](Get-R7ResolvedIdentityProperty $Resolved "repeat" 0) -ne 1) {
        $findings.Add("sample_identity_mismatch")
    }
    if ($mode -ne $expectedMode -or
        [string](Get-R7ResolvedIdentityProperty $Resolved "run_side" "") -ne $side -or
        (Compare-Object @($side) $selectedSides -SyncWindow 0) -or
        [string](Get-R7ResolvedIdentityProperty $logicalModeMap $side "") -ne $mode) {
        $findings.Add("side_mode_identity_mismatch")
    }
    if ($projection -ne $expectedProjection -or
        [string](Get-R7ResolvedIdentityProperty $Resolved "taskspace_projection_policy" "") -ne
            $expectedProjection -or
        ($mode -eq "standard" -and $arm -ne "standard") -or
        ($mode -eq "taskspace" -and ($arm -ne $projection -or
                $arm -notin @("map-always", "map-append", "map-request")))) {
        $findings.Add("projection_identity_mismatch")
    }
    if ([string](Get-R7ResolvedIdentityProperty $Resolved $modelField "") -ne
            [string]$Authority.model -or
        (
            [string](Get-R7ResolvedIdentityProperty $Resolved $binaryField "")
        ).ToLowerInvariant() -ne $binarySha) {
        $findings.Add("model_or_binary_identity_mismatch")
    }
    if ($null -eq $scenarioIdentity -or
        -not (Test-R7Sha256Text $promptLeft) -or
        $promptLeft -ne $promptRight -or
        $promptLeft.ToLowerInvariant() -ne [string]$scenarioIdentity.prompt_sha256 -or
        -not (Test-R7Sha256Text $fixtureLeft) -or
        $fixtureLeft -ne $fixtureRight -or
        $fixtureLeft.ToLowerInvariant() -ne [string]$scenarioIdentity.fixture_sha256) {
        $findings.Add("sample_content_identity_mismatch")
    }
    $requiredProviderParams = @("model", "model_reasoning_effort", "sandbox_mode")
    if ([string](Get-R7ResolvedIdentityProperty $Resolved "execution_substrate" "") -ne
            [string]$Authority.execution -or
        [string](Get-R7ResolvedIdentityProperty $Resolved "container_image_digest" "") -ne
            [string]$Authority.container_image_digest -or
        [string](Get-R7ResolvedIdentityProperty $Resolved "sandbox_mode" "") -ne
            [string]$Authority.sandbox_mode -or
        -not [bool](Get-R7ResolvedIdentityProperty $provider "complete" $false) -or
        @((Get-R7ResolvedIdentityProperty $provider "missing" @())).Count -ne 0 -or
        -not (Test-R7ExactSequence $requiredProviderParams @(
                Get-R7ResolvedIdentityProperty $provider "required" @()
            )) -or
        [string](Get-R7ResolvedIdentityProperty $explicit "model" "") -ne
            [string]$Authority.model -or
        [string](Get-R7ResolvedIdentityProperty $explicit "model_reasoning_effort" "") -ne
            [string]$Authority.reasoning_effort -or
        [string](Get-R7ResolvedIdentityProperty $explicit "sandbox_mode" "") -ne
            [string]$Authority.sandbox_mode) {
        $findings.Add("execution_capability_identity_mismatch")
    }
    if ($null -eq $profile -or
        [string](Get-R7ResolvedIdentityProperty $WireCapability "schema_version" "") -ne
            "provider-chat-wire-trace-v10" -or
        [string](Get-R7ResolvedIdentityProperty $WireCapability "provider_wire_api" "") -ne
            [string]$Authority.provider_wire_api -or
        [string](Get-R7ResolvedIdentityProperty $WireCapability "transport" "") -ne
            [string]$Authority.provider_transport -or
        [string](Get-R7ResolvedIdentityProperty $WireCapability "tools_hash" "") -ne
            [string](Get-R7ResolvedIdentityProperty $profile "tools_hash" "") -or
        [int64](Get-R7ResolvedIdentityProperty $WireCapability "tools_count" 0) -ne
            [int64](Get-R7ResolvedIdentityProperty $profile "tools_count" -1)) {
        $findings.Add("provider_wire_capability_identity_mismatch")
    }

    [pscustomobject]@{
        status = if ($findings.Count) { "invalid" } else { "valid" }
        findings = @($findings)
        sample_repeat = "$sample|$repeat"
        arm = $arm
        prompt_sha256 = $promptLeft.ToLowerInvariant()
        fixture_sha256 = $fixtureLeft.ToLowerInvariant()
        model = [string]$Authority.model
        reasoning_effort = [string]$Authority.reasoning_effort
        sandbox_mode = [string]$Authority.sandbox_mode
        container_image_digest = [string](
            Get-R7ResolvedIdentityProperty $Resolved "container_image_digest" ""
        )
        provider_wire_api = [string](
            Get-R7ResolvedIdentityProperty $WireCapability "provider_wire_api" ""
        )
        provider_transport = [string](
            Get-R7ResolvedIdentityProperty $WireCapability "transport" ""
        )
        tools_hash = [string](
            Get-R7ResolvedIdentityProperty $WireCapability "tools_hash" ""
        )
        tools_count = Get-R7ResolvedIdentityProperty $WireCapability "tools_count" 0
        whale_sha256 = $binarySha
    }
}
