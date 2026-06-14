param(
    [Parameter(Mandatory = $true)][string]$SuiteRoot,
    [string]$OutputRoot = ""
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "lib\runtime-reconstruction.ps1")

$result = Write-TaskspaceRuntimeReconstruction -SuiteRoot $SuiteRoot -OutputRoot $OutputRoot
Write-Host "RuntimeReconstruction: $($result.json_path)"
Write-Host "RuntimeReconstructionReport: $($result.markdown_path)"
