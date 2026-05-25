param(
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 150
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$RunId = Get-Date -Format "yyyyMMdd-HHmmss-fff"
$RunRoot = Join-Path $RepoRoot "target\tui-taskspace-viewer-e2e\$RunId"
$RepoDir = Join-Path $RunRoot "repo"
$ArtifactDir = Join-Path $RunRoot "artifacts"
$PtyRoot = Join-Path $RepoRoot "target\pty-tools"
$PtyModule = Join-Path $PtyRoot "node_modules\@homebridge\node-pty-prebuilt-multiarch"

New-Item -ItemType Directory -Force $RepoDir, $ArtifactDir, $PtyRoot | Out-Null
Set-Content -LiteralPath (Join-Path $RepoDir "README.md") -Encoding UTF8 -Value "# TUI task-show viewer E2E`n"

if (-not (Test-Path -LiteralPath $WhaleBin)) {
    throw "Cannot find installed Whale binary: $WhaleBin"
}

if (-not (Test-Path -LiteralPath $PtyModule)) {
    Push-Location $PtyRoot
    try {
        if (-not (Test-Path -LiteralPath (Join-Path $PtyRoot "package.json"))) {
            npm init -y | Out-Null
        }
        npm install @homebridge/node-pty-prebuilt-multiarch@0.13.1 | Out-Null
    } finally {
        Pop-Location
    }
}

$LogPath = Join-Path $ArtifactDir "tui-output.log"
$ReportPath = Join-Path $ArtifactDir "report.md"
$JsPath = Join-Path $ArtifactDir "run-tui-taskspace-viewer.js"
$Marker = "TASKSPACE_VIEWER_OK_$RunId"

$PtyModuleJson = $PtyModule.Replace('\', '/') | ConvertTo-Json -Compress
$LogPathJson = $LogPath.Replace('\', '/') | ConvertTo-Json -Compress
$ReportPathJson = $ReportPath.Replace('\', '/') | ConvertTo-Json -Compress
$WhaleBinJson = $WhaleBin.Replace('\', '/') | ConvertTo-Json -Compress
$RepoDirJson = $RepoDir.Replace('\', '/') | ConvertTo-Json -Compress
$ModelJson = $Model | ConvertTo-Json -Compress
$MarkerJson = $Marker | ConvertTo-Json -Compress
$TimeoutMs = $TimeoutSeconds * 1000

$Js = @"
const fs = require('fs');
const pty = require($PtyModuleJson);
const logPath = $LogPathJson;
const reportPath = $ReportPathJson;
const whale = $WhaleBinJson;
const repo = $RepoDirJson;
const model = $ModelJson;
const marker = $MarkerJson;
let out = '';
let viewerUrl = null;
let snapshotOk = false;
let snapshotMode = '';
let snapshotMapCount = 0;
let activeMapId = '';
let rebornCommandSent = false;
let initialEmptySnapshotOk = false;
let pageOk = false;
let graphUiOk = false;
let stateUiOk = false;
let fetchError = '';
let exited = false;

function write(data) {
  out += data;
  fs.writeFileSync(logPath, out, 'utf8');
  const matches = [...out.matchAll(/(?:Action Map|TaskSpace) viewer:\s+(http:\/\/127\.0\.0\.1:\d+\/)/g)];
  if (matches.length) viewerUrl = matches[matches.length - 1][1];
}

function markerCount() {
  return (out.match(new RegExp(marker, 'g')) || []).length;
}

async function probeViewer() {
  if (!viewerUrl) return;
  try {
    const html = await (await fetch(viewerUrl, { cache: 'no-store' })).text();
    graphUiOk = html.includes("className='graph'") &&
      html.includes("document.createElementNS('http://www.w3.org/2000/svg'") &&
      html.includes("marker-end','url(#arrow)'");
    stateUiOk = html.includes("details[data-key]") &&
      html.includes('saveUi()') &&
      html.includes('restoreUi()');
    pageOk = html.includes("fetch('/snapshot.json'") &&
      html.includes('setInterval(refresh,2000)') &&
      graphUiOk &&
      stateUiOk;
    const response = await fetch(viewerUrl + 'snapshot.json', { cache: 'no-store' });
    const json = await response.json();
    snapshotMapCount = json.snapshot && Array.isArray(json.snapshot.maps) ? json.snapshot.maps.length : 0;
    activeMapId = json.snapshot && json.snapshot.activeMapId ? json.snapshot.activeMapId : '';
    snapshotOk = json.ok === true &&
      json.snapshot &&
      json.snapshot.mode === 'experiment' &&
      snapshotMapCount > 0 &&
      !!activeMapId;
    snapshotMode = json.snapshot ? json.snapshot.mode : '';
    if (!rebornCommandSent &&
      json.ok === true &&
      json.snapshot &&
      json.snapshot.mode === 'experiment' &&
      snapshotMapCount === 0 &&
      !activeMapId) {
      initialEmptySnapshotOk = true;
    }
    fetchError = json.error || '';
  } catch (err) {
    fetchError = String(err);
  }
}

function report(overall, exitCode, failure) {
  const sawStub = out.includes('Not available in TUI yet');
  fs.writeFileSync(reportPath, [
    '# TUI TaskSpace Viewer E2E',
    '',
    '- overall: ' + overall,
    '- exit_code: ' + exitCode,
    '- viewer_url: ' + (viewerUrl || ''),
    '- page_ok: ' + pageOk,
    '- graph_ui_ok: ' + graphUiOk,
    '- state_ui_ok: ' + stateUiOk,
    '- snapshot_ok: ' + snapshotOk,
    '- snapshot_mode: ' + snapshotMode,
    '- snapshot_map_count: ' + snapshotMapCount,
    '- active_map_id: ' + activeMapId,
    '- initial_empty_snapshot_ok: ' + initialEmptySnapshotOk,
    '- saw_dialogue_marker: ' + (markerCount() > 0),
    '- marker_count: ' + markerCount(),
    '- saw_stub_error: ' + sawStub,
    '- fetch_error: ' + fetchError,
    '- failure: ' + (failure || ''),
    '- log: ' + logPath,
    ''
  ].join('\n'), 'utf8');
}

const term = pty.spawn(whale, ['--no-alt-screen', '-C', repo, '-m', model, '--dangerously-bypass-approvals-and-sandbox'], {
  name: 'xterm-256color',
  cols: 160,
  rows: 48,
  cwd: repo,
  env: { ...process.env, TERM: 'xterm-256color', WHALE_ACTION_MAP_VIEWER_NO_BROWSER: '1' }
});

term.onData(write);
function send(ms, text) { setTimeout(() => term.write(text), ms); }
function cmd(ms, text) { send(ms, text + '\r'); send(ms + 350, '\r'); }
function cmdWithHook(ms, text, hook) {
  setTimeout(() => { hook(); term.write(text + '\r'); }, ms);
  send(ms + 350, '\r');
}

send(2500, '\r');
cmd(8000, '/taskspace');
cmdWithHook(18000, '/task-reborn', () => { rebornCommandSent = true; });
cmd(22000, '/task-show');
cmd(28000, 'Reply with exactly ' + marker + ' and nothing else.');
cmd(90000, '/task-show');
cmd(120000, '/quit');

const deadline = Date.now() + $TimeoutMs;
const timer = setInterval(async () => {
  await probeViewer();
  const pass = viewerUrl && pageOk && initialEmptySnapshotOk && snapshotOk && markerCount() > 0 && !out.includes('Not available in TUI yet');
  if (pass) {
    clearInterval(timer);
    report('PASS', '', '');
    try { term.write('/quit\r\r'); } catch {}
    setTimeout(() => process.exit(0), 1500);
    return;
  }
  if (Date.now() > deadline) {
    clearInterval(timer);
    report('FAIL', '', 'timeout');
    try { term.kill(); } catch {}
    setTimeout(() => process.exit(1), 500);
  }
}, 1000);

term.onExit(({ exitCode }) => {
  if (exited) return;
  exited = true;
  clearInterval(timer);
  const overall = viewerUrl && pageOk && initialEmptySnapshotOk && snapshotOk && markerCount() > 0 && !out.includes('Not available in TUI yet') ? 'PASS' : 'FAIL';
  report(overall, exitCode, '');
  process.exit(overall === 'PASS' ? 0 : 1);
});
"@

Set-Content -LiteralPath $JsPath -Encoding UTF8 -Value $Js

node $JsPath
$ExitCode = $LASTEXITCODE
if (Test-Path -LiteralPath $ReportPath) {
    Get-Content -LiteralPath $ReportPath -Encoding UTF8
}
if ($ExitCode -ne 0) {
    throw "TUI task-show viewer E2E failed. Artifacts: $ArtifactDir"
}
