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
    if ($Output -match '"exit_code"\s*:\s*([1-9]\d*)') {
        return "shell_exit_$($Matches[1])"
    }
    if ($Output -match 'local_validator_infra_failure') {
        return "local_validator_infra_failure"
    }
    if ($Output -match 'Tool call failed') {
        return "tool_call_failed"
    }
    ""
}
