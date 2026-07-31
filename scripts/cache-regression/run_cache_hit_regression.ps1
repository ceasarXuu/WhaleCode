param(
    [Parameter(Mandatory = $true)][string]$AuthorizationReference,
    [string]$WhaleBin = "$HOME/.whale/bin/whale",
    [string]$RunRoot = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
$args = @(
    (Join-Path $PSScriptRoot "run_cache_hit_regression.py"),
    "--repo-root", $repoRoot,
    "--authorization-reference", $AuthorizationReference,
    "--whale-bin", $WhaleBin
)
if (-not [string]::IsNullOrWhiteSpace($RunRoot)) {
    $args += @("--run-root", $RunRoot)
}
& python3 @args
exit $LASTEXITCODE
