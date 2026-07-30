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
        [Parameter(Mandatory = $true)]$MatrixManifest,
        [Parameter(Mandatory = $true)]$Run,
        [Parameter(Mandatory = $true)]$Resolved
    )
    $findings = [Collections.Generic.List[string]]::new()
    $side = [string](Get-R7ResolvedIdentityProperty $Run "run_side" "")
    $mode = [string](Get-R7ResolvedIdentityProperty $Run "logical_mode" "")
    $arm = [string](Get-R7ResolvedIdentityProperty $Run "arm" "")
    $projection = [string](Get-R7ResolvedIdentityProperty $Run "projection_policy" "")
    $sample = [string](Get-R7ResolvedIdentityProperty $Run "sample" "")
    $repeat = [int](Get-R7ResolvedIdentityProperty $Run "repeat" 0)
    $model = [string](Get-R7ResolvedIdentityProperty $MatrixManifest "model" "")
    $binarySha = (
        [string](Get-R7ResolvedIdentityProperty $MatrixManifest "whale_sha256" "")
    ).ToLowerInvariant()
    $execution = [string](Get-R7ResolvedIdentityProperty $MatrixManifest "execution" "")
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

    if ($sample -notin @(
            Get-R7ResolvedIdentityProperty $MatrixManifest "samples" @()
        ) -or
        $repeat -lt 1 -or
        $repeat -gt [int](
            Get-R7ResolvedIdentityProperty $MatrixManifest "repeats_per_arm_per_sample" 0
        ) -or
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
    if ([string](Get-R7ResolvedIdentityProperty $Resolved "taskspace_projection_policy" "") -ne
        $projection -or
        ($mode -eq "standard" -and $arm -ne "standard") -or
        ($mode -eq "taskspace" -and ($arm -ne $projection -or
                $arm -notin @("map-always", "map-append", "map-request")))) {
        $findings.Add("projection_identity_mismatch")
    }
    if ([string](Get-R7ResolvedIdentityProperty $Resolved $modelField "") -ne $model -or
        (
            [string](Get-R7ResolvedIdentityProperty $Resolved $binaryField "")
        ).ToLowerInvariant() -ne $binarySha) {
        $findings.Add("model_or_binary_identity_mismatch")
    }
    if (-not (Test-R7Sha256Text $promptLeft) -or $promptLeft -ne $promptRight -or
        -not (Test-R7Sha256Text $fixtureLeft) -or $fixtureLeft -ne $fixtureRight) {
        $findings.Add("sample_content_identity_mismatch")
    }
    $requiredProviderParams = @("model", "model_reasoning_effort", "sandbox_mode")
    if ($execution -ne "docker" -or
        [string](Get-R7ResolvedIdentityProperty $Resolved "execution_substrate" "") -ne
            $execution -or
        [string](Get-R7ResolvedIdentityProperty $Resolved "container_image_digest" "") -notmatch
            '^sha256:[a-fA-F0-9]{64}$' -or
        -not [bool](Get-R7ResolvedIdentityProperty $provider "complete" $false) -or
        @((Get-R7ResolvedIdentityProperty $provider "missing" @())).Count -ne 0 -or
        (Compare-Object $requiredProviderParams @(
                Get-R7ResolvedIdentityProperty $provider "required" @()
            )) -or
        [string](Get-R7ResolvedIdentityProperty $explicit "model" "") -ne $model -or
        [string]::IsNullOrWhiteSpace(
            [string](Get-R7ResolvedIdentityProperty $explicit "model_reasoning_effort" "")
        ) -or
        [string](Get-R7ResolvedIdentityProperty $explicit "sandbox_mode" "") -ne
            [string](Get-R7ResolvedIdentityProperty $Resolved "sandbox_mode" "")) {
        $findings.Add("execution_capability_identity_mismatch")
    }

    [pscustomobject]@{
        status = if ($findings.Count) { "invalid" } else { "valid" }
        findings = @($findings)
        sample_repeat = "$sample|$repeat"
        arm = $arm
        prompt_sha256 = $promptLeft.ToLowerInvariant()
        fixture_sha256 = $fixtureLeft.ToLowerInvariant()
        model = $model
        reasoning_effort = [string](
            Get-R7ResolvedIdentityProperty $explicit "model_reasoning_effort" ""
        )
        sandbox_mode = [string](Get-R7ResolvedIdentityProperty $explicit "sandbox_mode" "")
        container_image_digest = [string](
            Get-R7ResolvedIdentityProperty $Resolved "container_image_digest" ""
        )
        whale_sha256 = $binarySha
    }
}
