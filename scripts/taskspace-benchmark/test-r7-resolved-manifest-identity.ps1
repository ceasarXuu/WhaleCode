$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/r7-evaluation-authority.ps1")
. (Join-Path $PSScriptRoot "lib/r7-resolved-manifest-identity.ps1")

$authority = Get-R7EvaluationAuthority $repoRoot "initial"
$sample = "single-file-fast-fix"
$scenario = $authority.scenarios[$sample]
$matrix = [pscustomobject]@{
    model = $authority.model
    whale_sha256 = "a" * 64
}
$run = [pscustomobject]@{
    sample = $sample
    repeat = 1
    arm = "map-request"
    logical_mode = "taskspace"
    projection_policy = "map-request"
    run_side = "right"
}
$resolved = [pscustomobject]@{
    scenario = $sample
    repeat = 1
    prompt_sha256_left = $scenario.prompt_sha256
    prompt_sha256_right = $scenario.prompt_sha256
    fixture_sha256_left = $scenario.fixture_sha256
    fixture_sha256_right = $scenario.fixture_sha256
    whale_sha256_left = "a" * 64
    whale_sha256_right = "a" * 64
    model_left = $authority.model
    model_right = $authority.model
    execution_substrate = "docker"
    container_image_digest = $authority.container_image_digest
    provider_param_status = [pscustomobject]@{
        complete = $true
        required = @("model", "model_reasoning_effort", "sandbox_mode")
        explicit = [pscustomobject]@{
            model = $authority.model
            model_reasoning_effort = $authority.reasoning_effort
            sandbox_mode = $authority.sandbox_mode
        }
        missing = @()
    }
    taskspace_projection_policy = "map-request"
    sandbox_mode = $authority.sandbox_mode
    logical_mode_map = [pscustomobject]@{
        left = "standard"
        right = "taskspace"
    }
    run_side = "right"
    selected_sides = @("right")
}
$wire = [pscustomobject]@{
    schema_version = "provider-chat-wire-trace-v11"
    provider_wire_api = $authority.provider_wire_api
    transport = $authority.provider_transport
    tools_hash = $authority.tool_capability_profiles.taskspace.tools_hash
    tools_count = [int64]$authority.tool_capability_profiles.taskspace.tools_count
}

function Copy-JsonObject($Value) {
    $Value | ConvertTo-Json -Depth 30 | ConvertFrom-Json -Depth 30
}

function Assert-IdentityRejected(
    [string]$Name,
    [string]$ExpectedFinding,
    [scriptblock]$Mutation
) {
    $candidateMatrix = Copy-JsonObject $matrix
    $candidateRun = Copy-JsonObject $run
    $candidateResolved = Copy-JsonObject $resolved
    $candidateWire = Copy-JsonObject $wire
    & $Mutation $candidateMatrix $candidateRun $candidateResolved $candidateWire
    $result = Get-R7ResolvedManifestIdentityCheck `
        $authority `
        $candidateMatrix `
        $candidateRun `
        $candidateResolved `
        $candidateWire
    if ([string]$result.status -ne "invalid" -or
        $ExpectedFinding -notin @($result.findings)) {
        throw "$Name resolved identity was accepted"
    }
}

$valid = Get-R7ResolvedManifestIdentityCheck $authority $matrix $run $resolved $wire
if ([string]$valid.status -ne "valid") {
    throw "Valid resolved manifest identity was rejected: $($valid.findings -join ',')"
}
Assert-IdentityRejected "self-consistent-sample-forgery" "sample_identity_mismatch" {
    param($matrixValue, $runValue, $resolvedValue)
    $runValue.sample = "forged"
    $resolvedValue.scenario = "forged"
    $resolvedValue.prompt_sha256_left = "b" * 64
    $resolvedValue.prompt_sha256_right = "b" * 64
}
Assert-IdentityRejected "side" "side_mode_identity_mismatch" {
    param($matrixValue, $runValue, $resolvedValue) $resolvedValue.run_side = "left"
}
Assert-IdentityRejected "projection" "projection_identity_mismatch" {
    param($matrixValue, $runValue, $resolvedValue)
    $resolvedValue.taskspace_projection_policy = "map-append"
}
Assert-IdentityRejected "self-consistent-model-forgery" "model_or_binary_identity_mismatch" {
    param($matrixValue, $runValue, $resolvedValue)
    $matrixValue.model = "forged"
    $resolvedValue.model_right = "forged"
}
Assert-IdentityRejected "prompt" "sample_content_identity_mismatch" {
    param($matrixValue, $runValue, $resolvedValue)
    $resolvedValue.prompt_sha256_left = "e" * 64
    $resolvedValue.prompt_sha256_right = "e" * 64
}
Assert-IdentityRejected "empty-sandbox" "execution_capability_identity_mismatch" {
    param($matrixValue, $runValue, $resolvedValue)
    $resolvedValue.sandbox_mode = ""
    $resolvedValue.provider_param_status.explicit.sandbox_mode = ""
}
Assert-IdentityRejected "image" "execution_capability_identity_mismatch" {
    param($matrixValue, $runValue, $resolvedValue)
    $resolvedValue.container_image_digest = "sha256:$("d" * 64)"
}
Assert-IdentityRejected "wire-tools" "provider_wire_capability_identity_mismatch" {
    param($matrixValue, $runValue, $resolvedValue, $wireValue)
    $wireValue.tools_hash = "e" * 64
}
Assert-IdentityRejected "wire-count" "provider_wire_capability_identity_mismatch" {
    param($matrixValue, $runValue, $resolvedValue, $wireValue)
    $wireValue.tools_count = 12
}
Assert-IdentityRejected "standard-map-append" "projection_identity_mismatch" {
    param($matrixValue, $runValue, $resolvedValue, $wireValue)
    $runValue.arm = "standard"
    $runValue.logical_mode = "standard"
    $runValue.run_side = "left"
    $runValue.projection_policy = "map-append"
    $resolvedValue.run_side = "left"
    $resolvedValue.selected_sides = @("left")
    $resolvedValue.taskspace_projection_policy = "map-append"
    $wireValue.tools_hash = $authority.tool_capability_profiles.standard.tools_hash
    $wireValue.tools_count = $authority.tool_capability_profiles.standard.tools_count
}
Write-Output "R7 resolved manifest identity passed."
