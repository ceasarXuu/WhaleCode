$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/container-contract.ps1")

$contract = Read-TaskspaceContainerContract $repoRoot
$resources = @(Get-TaskspaceContainerResourceArgs $contract)
$logs = @(Get-TaskspaceContainerLogArgs $contract)
$matrix = @(Get-TaskspaceContainerPermissionMatrix $contract)

if ($resources -notcontains '--cpus' -or $resources -notcontains '--memory') {
    throw "Resource args are incomplete"
}
if ($logs -notcontains 'local' -or $logs -notcontains 'max-size=10m') {
    throw "Log args are incomplete"
}
if ($matrix.Count -ne 3) { throw "Permission matrix must contain three roles" }
$agent = @($matrix | Where-Object { $_.role -eq 'agent' })[0]
$oracle = @($matrix | Where-Object { $_.role -eq 'oracle' })[0]
if ($agent.oracle -ne 'none' -or $agent.provider_secret -ne 'ro') {
    throw "Agent mount policy is unsafe"
}
if ($oracle.workspace -ne 'ro' -or $oracle.oracle -ne 'ro') {
    throw "Oracle mount policy is unsafe"
}

$invalid = $contract | ConvertTo-Json -Depth 20 | ConvertFrom-Json
$invalid.base_image = 'ubuntu:24.04'
$failed = $false
try { Assert-TaskspaceContainerContract $invalid } catch { $failed = $true }
if (-not $failed) { throw "Unpinned image contract was accepted" }

Write-Host "container contract tests passed"
