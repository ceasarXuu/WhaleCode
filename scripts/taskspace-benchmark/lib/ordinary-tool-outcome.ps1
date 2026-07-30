function Get-TaskspaceStructuredShellFailureCode {
    param([AllowEmptyString()][string]$Output = "")
    $trimmed = $Output.Trim()
    if (-not $trimmed.StartsWith("{") -or -not $trimmed.EndsWith("}")) {
        return ""
    }
    $settings = [Newtonsoft.Json.Linq.JsonLoadSettings]::new()
    $settings.DuplicatePropertyNameHandling =
        [Newtonsoft.Json.Linq.DuplicatePropertyNameHandling]::Error
    try {
        $root = [Newtonsoft.Json.Linq.JToken]::Parse($trimmed, $settings)
    } catch {
        return ""
    }
    if ($root -isnot [Newtonsoft.Json.Linq.JObject]) { return "" }
    $metadata = ([Newtonsoft.Json.Linq.JObject]$root).GetValue(
        "metadata",
        [StringComparison]::Ordinal
    )
    if ($metadata -isnot [Newtonsoft.Json.Linq.JObject]) {
        return ""
    }
    $exitCode = ([Newtonsoft.Json.Linq.JObject]$metadata).GetValue(
        "shell_exit_code",
        [StringComparison]::Ordinal
    )
    if ($exitCode -isnot [Newtonsoft.Json.Linq.JValue] -or
        $exitCode.Value -isnot [int64]) {
        return ""
    }
    [int64]$number = $exitCode.Value
    if ($number -lt 1) { return "" }
    "shell_exit_$number"
}

function Get-TaskspaceOrdinaryToolFailureCode {
    param([AllowEmptyString()][string]$Output = "")

    if ($Output -match 'apply_patch verification failed') {
        return "apply_patch_verification_failed"
    }
    if ($Output -match '(?m)^Shell exit code:\s*([1-9]\d*)\s*$') {
        return "shell_exit_$($Matches[1])"
    }
    if ($Output -match '(?m)^Exit code:\s*([1-9]\d*)\s*$') {
        return "shell_exit_$($Matches[1])"
    }
    $structuredExit = Get-TaskspaceStructuredShellFailureCode $Output
    if (-not [string]::IsNullOrWhiteSpace($structuredExit)) { return $structuredExit }
    if ($Output -match 'local_validator_infra_failure') {
        return "local_validator_infra_failure"
    }
    if ($Output -match 'Tool call failed') {
        return "tool_call_failed"
    }
    ""
}
