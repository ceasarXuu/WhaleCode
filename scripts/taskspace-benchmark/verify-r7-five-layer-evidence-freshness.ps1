param(
    [Parameter(Mandatory = $true)][string]$WhaleBin,
    [Parameter(Mandatory = $true)][string]$ResultPath,
    [Parameter(Mandatory = $true)][string[]]$RunRoots,
    [string]$OutputPath = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
. (Join-Path $PSScriptRoot "lib/r7-five-layer-evidence-freshness.ps1")

$RunRoots = @($RunRoots | ForEach-Object { ([string]$_) -split "," } | ForEach-Object { ([string]$_).Trim() } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
$result = Test-R7FiveLayerEvidenceFreshness -RepoRoot $repoRoot -WhaleBin $WhaleBin -ResultPath $ResultPath -RunRoots $RunRoots
if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $OutputPath = Join-Path $repoRoot "target/r7-five-layer/evidence-freshness.json"
} elseif (-not [System.IO.Path]::IsPathRooted($OutputPath)) {
    $OutputPath = Join-Path $repoRoot $OutputPath
}
$outputDir = Split-Path -Parent $OutputPath
if (-not [string]::IsNullOrWhiteSpace($outputDir)) {
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
}
$result | ConvertTo-Json -Depth 100 | Set-Content -LiteralPath $OutputPath -Encoding UTF8
Write-Output "R7FiveLayerEvidenceFreshness: $($result.status)"
Write-Output "EvidenceFreshnessReport: $OutputPath"
if ([string]$result.status -ne "pass") {
    foreach ($finding in @($result.findings)) {
        Write-Output "[$($finding.stable_code)] $($finding.message) $($finding.path)"
    }
    exit 1
}
