$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$benchmarkRoot = Join-Path $repoRoot 'scripts/taskspace-benchmark'
$failures = New-Object System.Collections.Generic.List[string]

function Assert-DockerOnly([bool]$Condition, [string]$Message) {
    if (-not $Condition) { $script:failures.Add($Message) }
}

$runnerPath = Join-Path $benchmarkRoot 'run-taskspace-benchmark.ps1'
$workspacePath = Join-Path $benchmarkRoot 'lib/workspace.ps1'
$bootstrapPath = Join-Path $benchmarkRoot 'lib/bootstrap.ps1'
$productionPaths = @(
    $runnerPath,
    (Join-Path $benchmarkRoot 'run-taskspace-e2-matrix.ps1'),
    (Join-Path $benchmarkRoot 'run-taskspace-e3-suite.ps1'),
    (Join-Path $benchmarkRoot 'run-taskspace-e3-external.ps1'),
    (Join-Path $benchmarkRoot 'run-taskspace-external-benchmark.ps1'),
    (Join-Path $benchmarkRoot 'lib/container-benchmark-runner.ps1'),
    $workspacePath,
    $bootstrapPath
)
$productionText = @($productionPaths | ForEach-Object { Get-Content -Raw -Encoding UTF8 -LiteralPath $_ }) -join "`n"
$runnerText = Get-Content -Raw -Encoding UTF8 -LiteralPath $runnerPath
$workspaceText = Get-Content -Raw -Encoding UTF8 -LiteralPath $workspacePath

Assert-DockerOnly ($runnerText.Contains('Invoke-TaskspaceDockerAgent')) 'primary runner does not use Docker Agent execution'
Assert-DockerOnly ($runnerText.Contains('Invoke-TaskspaceDockerValidation')) 'primary runner does not use Docker validation'
Assert-DockerOnly ($runnerText.Contains('Invoke-TaskspaceDockerOracle')) 'primary runner does not use Docker oracle'
Assert-DockerOnly (-not $productionText.Contains('SandboxMode')) 'configurable nested sandbox compatibility remains reachable'
Assert-DockerOnly (-not $productionText.Contains('Invoke-TaskspaceProbeProcess')) 'host Agent probe remains reachable'
Assert-DockerOnly (-not $productionText.Contains('Invoke-TaskspaceValidationCommand')) 'host validator remains reachable'
Assert-DockerOnly (-not $productionText.Contains('Invoke-TaskspaceHiddenOracle')) 'host oracle remains reachable'
Assert-DockerOnly ($workspaceText.Contains('--dangerously-bypass-approvals-and-sandbox')) 'Whale bypass is not fixed for the Docker boundary'
Assert-DockerOnly (-not $workspaceText.Contains('--full-auto')) 'nested full-auto sandbox remains in Whale argv construction'
Assert-DockerOnly (-not (Test-Path -LiteralPath (Join-Path $benchmarkRoot 'lib/oracle-runner.ps1'))) 'legacy host oracle runner still exists'
Assert-DockerOnly (-not (Test-Path -LiteralPath (Join-Path $benchmarkRoot 'test-oracle-runner-harness.ps1'))) 'legacy host oracle tests still exist'

if ($failures.Count -gt 0) {
    $failures | ForEach-Object { Write-Host " - $_" }
    exit 1
}
Write-Host 'Docker-only call graph test passed'
