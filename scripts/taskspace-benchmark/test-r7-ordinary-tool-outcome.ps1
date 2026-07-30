$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "lib/ordinary-tool-outcome.ps1")

function Assert-Equal($Actual, $Expected, [string]$Message) {
    if ($Actual -ne $Expected) {
        throw "$Message`: expected=$Expected actual=$Actual"
    }
}

$valid = [pscustomobject]@{
    output = "Patch contains conflicting operations"
    metadata = [pscustomobject]@{
        execution_outcome = "exited"
        shell_exit_code = 7
    }
} | ConvertTo-Json -Compress
Assert-Equal (
    Get-TaskspaceOrdinaryToolFailureCode $valid
) "shell_exit_7" "Exact metadata shell exit was rejected"

$wrongScope = [pscustomobject]@{
    user_payload = [pscustomobject]@{ shell_exit_code = 7 }
} | ConvertTo-Json -Compress
Assert-Equal (
    Get-TaskspaceOrdinaryToolFailureCode $wrongScope
) "" "Wrong-scope shell exit was trusted"

foreach ($invalid in @(
        '{"metadata":{"shell_exit_code":1.5}}',
        '{"metadata":{"shell_exit_code":"1"}}',
        '{"metadata":{"shell_exit_code":0}}',
        '{"metadata":{"shell_exit_code":1,"shell_exit_code":2}}',
        '{"metadata":{"shell_exit_code":9}',
        '{"exit_code":9}'
    )) {
    Assert-Equal (
        Get-TaskspaceOrdinaryToolFailureCode $invalid
    ) "" "Malformed or ambiguous structured shell exit was trusted"
}

Assert-Equal (
    Get-TaskspaceOrdinaryToolFailureCode "Execution outcome: exited`nShell exit code: 3"
) "shell_exit_3" "Canonical shell output stopped classifying exits"
Assert-Equal (
    Get-TaskspaceOrdinaryToolFailureCode "Exit code: 4"
) "shell_exit_4" "Legacy explicit shell exit line stopped classifying exits"

Write-Output "R7 ordinary Tool outcome contract passed."
