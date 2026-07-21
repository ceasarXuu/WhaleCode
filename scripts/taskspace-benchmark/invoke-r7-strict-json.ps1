param(
    [Parameter(Mandatory = $true)][string]$Path,
    [string]$SchemaPath = "",
    [switch]$EmitCanonical
)

$ErrorActionPreference = "Stop"
. (Join-Path $PSScriptRoot "lib/r7-strict-json.ps1")

if ($EmitCanonical) {
    Write-Output (Read-R7StrictJsonInProcess $Path $SchemaPath -EmitCanonical)
    exit 0
}

$bytes = [System.IO.File]::ReadAllBytes((Resolve-Path -LiteralPath $Path).Path)
$canonical = Read-R7StrictJsonInProcess $Path $SchemaPath -EmitCanonical
$utf8 = [System.Text.UTF8Encoding]::new($false)
function Get-Hash([byte[]]$Value) {
    [System.BitConverter]::ToString([System.Security.Cryptography.SHA256]::Create().ComputeHash($Value)).Replace("-", "").ToLowerInvariant()
}
[pscustomobject][ordered]@{
    valid = $true
    path = (Resolve-Path -LiteralPath $Path).Path
    sha256 = Get-Hash $bytes
    canonical_sha256 = Get-Hash $utf8.GetBytes($canonical)
} | ConvertTo-Json -Compress
