function Invoke-TaskspaceRequestFactsGenerator {
    param(
        [AllowEmptyString()][string]$RolloutJsonlPath = "",
        [AllowEmptyString()][string]$WireTracePath = "",
        [AllowEmptyString()][string]$BoundaryEventsPath = "",
        [AllowEmptyString()][string]$ExpectedModel = "",
        [AllowEmptyString()][string]$OutputPath = ""
    )
    $builder = Join-Path (Split-Path -Parent $PSScriptRoot) "build-request-facts.py"
    $arguments = @($builder)
    if (-not [string]::IsNullOrWhiteSpace($RolloutJsonlPath)) {
        $arguments += @("--rollout", $RolloutJsonlPath)
    }
    if (-not [string]::IsNullOrWhiteSpace($WireTracePath)) {
        $arguments += @("--wire", $WireTracePath)
    }
    if (-not [string]::IsNullOrWhiteSpace($BoundaryEventsPath)) {
        $arguments += @("--boundary", $BoundaryEventsPath)
    }
    if (-not [string]::IsNullOrWhiteSpace($ExpectedModel)) {
        $arguments += @("--model", $ExpectedModel)
    }
    if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
        $arguments += @("--output", $OutputPath)
    }
    $rendered = @(& python3 @arguments 2>&1)
    $exitCode = $LASTEXITCODE
    if ($exitCode -notin @(0, 3)) {
        throw "request facts generator failed with exit=$exitCode output=$($rendered -join ' ')"
    }
    $raw = if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
        Get-Content -Raw -Encoding UTF8 -LiteralPath $OutputPath
    } else {
        $rendered -join "`n"
    }
    $facts = $raw | ConvertFrom-Json -Depth 100
    if ([string]$facts.schema_version -ne "whalecode-request-facts-v1") {
        throw "request facts generator returned an unsupported schema"
    }
    $facts
}
