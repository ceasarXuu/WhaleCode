$ErrorActionPreference = 'Stop'
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
. (Join-Path $repoRoot 'scripts/action-map-real-user-e2e-lib.ps1')
. (Join-Path $PSScriptRoot 'lib/container-contract.ps1')
. (Join-Path $PSScriptRoot 'lib/container-runtime.ps1')
. (Join-Path $PSScriptRoot 'lib/provider-boundary.ps1')
. (Join-Path $PSScriptRoot 'lib/container-benchmark-runner.ps1')

$contract = Read-TaskspaceContainerContract $repoRoot
$image = Resolve-TaskspaceContainerImage $repoRoot $contract
$root = New-Dir (Join-Path $repoRoot ("target/provider-boundary-selftest/{0}" -f ([guid]::NewGuid().ToString('N'))))
$side = [pscustomobject]@{
    Name = 'left'
    LogicalMode = 'standard'
    RepoDir = (New-Dir (Join-Path $root 'workspace'))
    ArtifactDir = (New-Dir (Join-Path $root 'artifacts'))
}
$boundary = $null
$mockId = ''
try {
    $boundary = Start-TaskspaceProviderBoundary 'provider-boundary-selftest' 'offline' 'pair-001' $side $image 'boundary-secret' 1 'http://mock-provider:8090'
    $mockScript = @'
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args): pass
    def do_POST(self):
        length = int(self.headers.get("Content-Length", "0"))
        self.rfile.read(length)
        body = json.dumps({"authorization": self.headers.get("Authorization", "")}).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)
ThreadingHTTPServer(("0.0.0.0", 8090), Handler).serve_forever()
'@
    $mock = Invoke-TaskspaceDocker @(
        'create', '--network', $boundary.egress_network, '--network-alias', 'mock-provider',
        '--label', 'whalecode.run_id=provider-boundary-selftest',
        [string]$image.image_ref, 'python', '-c', $mockScript
    )
    if ($mock.exit_code -ne 0) { throw "Mock provider create failed: $($mock.stderr)" }
    $mockId = $mock.stdout.Trim()
    $started = Invoke-TaskspaceDocker @('start', $mockId)
    if ($started.exit_code -ne 0) { throw "Mock provider start failed: $($started.stderr)" }
    Start-Sleep -Milliseconds 300

    $probeScript = @'
import json, urllib.error, urllib.request
request = urllib.request.Request("http://provider-proxy:8080/responses", data=b"{}", method="POST")
first = json.loads(urllib.request.urlopen(request, timeout=3).read())
assert first["authorization"] == "Bearer boundary-secret", first
try:
    urllib.request.urlopen(request, timeout=3)
except urllib.error.HTTPError as error:
    assert error.code == 429, error.code
else:
    raise AssertionError("second provider request exceeded no hard limit")
try:
    urllib.request.urlopen("http://mock-provider:8090", timeout=1)
except Exception:
    pass
else:
    raise AssertionError("agent network reached the provider egress network directly")
print("provider boundary probe passed")
'@
    $probe = Invoke-TaskspaceDocker @('run', '--rm', '--network', $boundary.internal_network, [string]$image.image_ref, 'python', '-c', $probeScript)
    if ($probe.exit_code -ne 0) { throw "Provider boundary probe failed: $($probe.stderr)" }
} finally {
    if ($mockId) { [void](Invoke-TaskspaceDocker @('rm', '--force', $mockId)) }
    $result = Stop-TaskspaceProviderBoundary $boundary $side.ArtifactDir
}
if ($null -eq $result -or $result.status -ne 'removed') { throw 'Provider boundary cleanup failed' }

$fakeWhale = Join-Path $root 'fake-whale'
Write-Text $fakeWhale @'
#!/usr/bin/env bash
set -euo pipefail
test ! -e /run/secrets/deepseek_api_key
test "${DEEPSEEK_API_KEY:-}" = provider-boundary-managed
echo provider-boundary-agent-ok
'@
if (-not $IsWindows) { & chmod 755 $fakeWhale }
Write-Text (Join-Path $side.ArtifactDir 'user-prompt.txt') "offline`n"
$agentResult = Invoke-TaskspaceDockerAgent `
    'provider-boundary-agent-selftest' 'offline' 'pair-001' $side $image $contract $fakeWhale `
    @('--dangerously-bypass-approvals-and-sandbox') @{} 'agent-hidden-secret' 1 20
if ($agentResult.exit_code -ne 0) { throw 'Boundary-managed agent fixture failed' }
$agentInspect = @(Get-Content -Raw -Encoding UTF8 -LiteralPath (Join-Path $side.ArtifactDir 'container-inspect-agent.json') | ConvertFrom-Json)[0]
$secretMounts = @($agentInspect.Mounts | Where-Object { [string]$_.Destination -eq '/run/secrets/deepseek_api_key' })
if ($secretMounts.Count -ne 0) { throw 'Agent container received the provider secret mount' }
$networkNames = @($agentInspect.NetworkSettings.Networks.PSObject.Properties.Name)
if ($networkNames.Count -ne 1 -or $networkNames[0] -notmatch '-internal$') {
    throw "Agent container was not restricted to one internal network: $($networkNames -join ',')"
}
Write-Host 'provider boundary tests passed'
