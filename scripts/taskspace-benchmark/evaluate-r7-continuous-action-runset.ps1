param(
    [Parameter(Mandatory = $true)][string]$RunSetPath,
    [string]$EvaluationContractPath = "",
    [string]$RunArtifactRoot = "",
    [string]$OutputPath = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "r7-v2-toolchain-core.ps1")
. (Join-Path $PSScriptRoot "lib/r7-continuous-action-evaluator.ps1")

if ([string]::IsNullOrWhiteSpace($EvaluationContractPath)) {
    $EvaluationContractPath = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/continuous-action-evaluation-v1.json"
}
$result = Invoke-R7ContinuousActionEvaluation $RunSetPath $EvaluationContractPath $RunArtifactRoot
$schema = Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/continuous-action-evaluation-result-v1.schema.json"
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $json = $result | ConvertTo-Json -Depth 100
    [void]($json | Test-Json -SchemaFile $schema -ErrorAction Stop)
    Write-Output $json
} else {
    Write-R7JsonFile $OutputPath $result
    [void](Read-R7StrictJson $OutputPath $schema)
}
