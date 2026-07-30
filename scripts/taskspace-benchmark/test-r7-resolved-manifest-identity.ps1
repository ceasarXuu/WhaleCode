$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/r7-resolved-manifest-identity.ps1")

$matrix = [pscustomobject]@{
    model = "deepseek-v4-flash"
    whale_sha256 = "a" * 64
    execution = "docker"
    samples = @("sample")
    repeats_per_arm_per_sample = 1
}
$run = [pscustomobject]@{
    sample = "sample"
    repeat = 1
    arm = "map-request"
    logical_mode = "taskspace"
    projection_policy = "map-request"
    run_side = "right"
}
$resolved = [pscustomobject]@{
    scenario = "sample"
    repeat = 1
    prompt_sha256_left = "b" * 64
    prompt_sha256_right = "b" * 64
    fixture_sha256_left = "c" * 64
    fixture_sha256_right = "c" * 64
    whale_sha256_left = "a" * 64
    whale_sha256_right = "a" * 64
    model_left = "deepseek-v4-flash"
    model_right = "deepseek-v4-flash"
    execution_substrate = "docker"
    container_image_digest = "sha256:$("d" * 64)"
    provider_param_status = [pscustomobject]@{
        complete = $true
        required = @("model", "model_reasoning_effort", "sandbox_mode")
        explicit = [pscustomobject]@{
            model = "deepseek-v4-flash"
            model_reasoning_effort = "max"
            sandbox_mode = "docker_hard_boundary"
        }
        missing = @()
    }
    taskspace_projection_policy = "map-request"
    sandbox_mode = "docker_hard_boundary"
    logical_mode_map = [pscustomobject]@{
        left = "standard"
        right = "taskspace"
    }
    run_side = "right"
    selected_sides = @("right")
}

function Copy-JsonObject($Value) {
    $Value | ConvertTo-Json -Depth 30 | ConvertFrom-Json -Depth 30
}

function Assert-IdentityRejected(
    [string]$Name,
    [string]$ExpectedFinding,
    [scriptblock]$Mutation
) {
    $candidate = Copy-JsonObject $resolved
    & $Mutation $candidate
    $result = Get-R7ResolvedManifestIdentityCheck $matrix $run $candidate
    if ([string]$result.status -ne "invalid" -or
        $ExpectedFinding -notin @($result.findings)) {
        throw "$Name resolved identity was accepted"
    }
}

$valid = Get-R7ResolvedManifestIdentityCheck $matrix $run $resolved
if ([string]$valid.status -ne "valid") {
    throw "Valid resolved manifest identity was rejected: $($valid.findings -join ',')"
}
Assert-IdentityRejected "sample" "sample_identity_mismatch" {
    param($value) $value.scenario = "forged"
}
Assert-IdentityRejected "side" "side_mode_identity_mismatch" {
    param($value) $value.run_side = "left"
}
Assert-IdentityRejected "mode" "side_mode_identity_mismatch" {
    param($value) $value.logical_mode_map.right = "standard"
}
Assert-IdentityRejected "projection" "projection_identity_mismatch" {
    param($value) $value.taskspace_projection_policy = "map-append"
}
Assert-IdentityRejected "model" "model_or_binary_identity_mismatch" {
    param($value) $value.model_right = "forged"
}
Assert-IdentityRejected "binary" "model_or_binary_identity_mismatch" {
    param($value) $value.whale_sha256_right = "e" * 64
}
Assert-IdentityRejected "prompt" "sample_content_identity_mismatch" {
    param($value) $value.prompt_sha256_right = "e" * 64
}
Assert-IdentityRejected "provider" "execution_capability_identity_mismatch" {
    param($value) $value.provider_param_status.complete = $false
}
Assert-IdentityRejected "reasoning" "execution_capability_identity_mismatch" {
    param($value) $value.provider_param_status.explicit.model_reasoning_effort = ""
}
Write-Output "R7 resolved manifest identity passed."
