function New-TerminalBenchUvCache {
    param([Parameter(Mandatory = $true)][string]$OutputRoot)
    $resolvedOutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)
    $cache = Join-Path $resolvedOutputRoot "_adapter-generated\uv-cache"
    New-Item -ItemType Directory -Force -Path (Join-Path $cache "bin") | Out-Null
    $installerUrl = "https://astral.sh/uv/0.7.13/install.sh"
    $archiveUrl = "https://github.com/astral-sh/uv/releases/download/0.7.13/uv-x86_64-unknown-linux-gnu.tar.gz"
    $installer = Join-Path $cache "install.sh"
    $archive = Join-Path $cache "uv-x86_64-unknown-linux-gnu.tar.gz"
    $seedRoot = if ($env:TASKSPACE_TBENCH_UV_CACHE_SOURCE) { [System.IO.Path]::GetFullPath($env:TASKSPACE_TBENCH_UV_CACHE_SOURCE) } else { "" }
    if (-not [string]::IsNullOrWhiteSpace($seedRoot) -and (Test-Path -LiteralPath $seedRoot)) {
        foreach ($name in @("install.sh", "uv-x86_64-unknown-linux-gnu.tar.gz")) {
            $seedFile = Join-Path $seedRoot $name
            if (Test-Path -LiteralPath $seedFile) {
                Copy-Item -LiteralPath $seedFile -Destination (Join-Path $cache $name) -Force
            }
        }
    }
    $enabled = $true
    foreach ($item in @(@($installerUrl, $installer, 60), @($archiveUrl, $archive, 180))) {
        $downloadOk = $true
        if (-not (Test-Path -LiteralPath $item[1])) {
            & curl.exe -sS -L --max-time $item[2] -o $item[1] $item[0] 2>$null | Out-Null
            $downloadOk = ($LASTEXITCODE -eq 0)
        }
        if (-not $downloadOk -or -not (Test-Path -LiteralPath $item[1])) { $enabled = $false }
    }
    $wrapper = Join-Path $cache "bin\curl"
    $wrapperContent = @'
#!/bin/sh
set -eu
out=""
prev=""
url=""
for arg in "$@"; do
  if [ "$prev" = "-o" ] || [ "$prev" = "--output" ]; then out="$arg"; fi
  case "$arg" in
    http*) url="$arg" ;;
    --output=*) out="${arg#--output=}" ;;
    -o*) if [ "$arg" != "-o" ]; then out="${arg#-o}"; fi ;;
  esac
  prev="$arg"
done
case "$url" in
  *astral.sh/uv/install.sh*) src=/tbench-uv-cache/install.sh ;;
  *astral.sh/uv/0.7.13/install.sh*) src=/tbench-uv-cache/install.sh ;;
  *github.com/astral-sh/uv/releases/download/0.7.13/*x86_64-unknown-linux-gnu*) src=/tbench-uv-cache/uv-x86_64-unknown-linux-gnu.tar.gz ;;
  *) exec /usr/bin/curl "$@" ;;
esac
if [ -n "$out" ]; then cp "$src" "$out"; else cat "$src"; fi
'@
    $wrapperContent = $wrapperContent -replace "`r`n", "`n"
    [System.IO.File]::WriteAllText($wrapper, $wrapperContent, [System.Text.Encoding]::ASCII)
    $aptWrapper = Join-Path $cache "bin\apt-get"
    $aptContent = @'
#!/bin/sh
set -eu
state=/tmp/tbench-apt-update-skipped
cmd="${1:-}"
if [ "$cmd" = "update" ]; then touch "$state"; exit 0; fi
if [ "$cmd" = "install" ]; then
  needs_real=""
  shift
  for arg in "$@"; do
    case "$arg" in -*) ;; curl) ;; *) needs_real=1 ;; esac
  done
  if [ -z "$needs_real" ] && [ -x /usr/bin/curl ]; then rm -f "$state"; exit 0; fi
  if [ -f "$state" ]; then rm -f "$state"; /usr/bin/apt-get update; fi
  exec /usr/bin/apt-get install "$@"
fi
if [ -f "$state" ]; then rm -f "$state"; /usr/bin/apt-get update; fi
exec /usr/bin/apt-get "$@"
'@
    $aptContent = $aptContent -replace "`r`n", "`n"
    [System.IO.File]::WriteAllText($aptWrapper, $aptContent, [System.Text.Encoding]::ASCII)
    [pscustomobject]@{
        enabled = $enabled
        root = $cache
        installer_url = $installerUrl
        installer_alias_url = "https://astral.sh/uv/install.sh"
        archive_url = $archiveUrl
        installer_path = $installer
        archive_path = $archive
        apt_get_curl_short_circuit = $true
        installer_sha256 = if (Test-Path $installer) { (Get-FileHash -Algorithm SHA256 -LiteralPath $installer).Hash.ToLowerInvariant() } else { "" }
        archive_sha256 = if (Test-Path $archive) { (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant() } else { "" }
        installer_size_bytes = if (Test-Path $installer) { [int64](Get-Item -LiteralPath $installer).Length } else { 0 }
        archive_size_bytes = if (Test-Path $archive) { [int64](Get-Item -LiteralPath $archive).Length } else { 0 }
    }
}
