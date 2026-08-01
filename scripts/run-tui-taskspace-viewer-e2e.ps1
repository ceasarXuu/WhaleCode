param(
    [string]$WhaleBin = "$env:USERPROFILE\.whale\bin\whale.exe",
    [string]$Model = "deepseek-v4-flash",
    [int]$TimeoutSeconds = 150,
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot
$RunId = Get-Date -Format "yyyyMMdd-HHmmss-fff"
if ([string]::IsNullOrWhiteSpace($OutputDir)) {
    $RunRoot = Join-Path $RepoRoot "target\tui-taskspace-viewer-e2e\$RunId"
} else {
    $RunRoot = $OutputDir
}
$RepoDir = Join-Path $RunRoot "repo"
$ArtifactDir = Join-Path $RunRoot "artifacts"
$PtyRoot = Join-Path $RepoRoot "target\pty-tools"
$PtyModule = Join-Path $PtyRoot "node_modules\@homebridge\node-pty-prebuilt-multiarch"
$PlaywrightModule = Join-Path $PtyRoot "node_modules\playwright-core"

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

if (-not (Test-Path -LiteralPath $PlaywrightModule)) {
    Push-Location $PtyRoot
    try {
        if (-not (Test-Path -LiteralPath (Join-Path $PtyRoot "package.json"))) {
            npm init -y | Out-Null
        }
        npm install playwright-core@1.60.0 | Out-Null
    } finally {
        Pop-Location
    }
}

$BrowserExe = @(
    "C:\Program Files\Google\Chrome\Application\chrome.exe",
    "C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    "C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
if ([string]::IsNullOrWhiteSpace($BrowserExe)) {
    throw "Cannot find Chrome or Edge for real TaskSpace viewer interaction E2E."
}

$LogPath = Join-Path $ArtifactDir "tui-output.log"
$ReportPath = Join-Path $ArtifactDir "report.md"
$JsPath = Join-Path $ArtifactDir "run-tui-taskspace-viewer.js"
$Marker = "TASKSPACE_VIEWER_OK_$RunId"

$PtyModuleJson = $PtyModule.Replace('\', '/') | ConvertTo-Json -Compress
$PlaywrightModuleJson = $PlaywrightModule.Replace('\', '/') | ConvertTo-Json -Compress
$ArtifactDirJson = $ArtifactDir.Replace('\', '/') | ConvertTo-Json -Compress
$LogPathJson = $LogPath.Replace('\', '/') | ConvertTo-Json -Compress
$ReportPathJson = $ReportPath.Replace('\', '/') | ConvertTo-Json -Compress
$WhaleBinJson = $WhaleBin.Replace('\', '/') | ConvertTo-Json -Compress
$BrowserExeJson = $BrowserExe.Replace('\', '/') | ConvertTo-Json -Compress
$RepoDirJson = $RepoDir.Replace('\', '/') | ConvertTo-Json -Compress
$ModelJson = $Model | ConvertTo-Json -Compress
$MarkerJson = $Marker | ConvertTo-Json -Compress
$TimeoutMs = $TimeoutSeconds * 1000

$Js = @"
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const pty = require($PtyModuleJson);
const playwrightModule = $PlaywrightModuleJson;
const { chromium } = require(playwrightModule);
const playwrightPackage = require(path.join(playwrightModule, 'package.json'));
const artifactDir = $ArtifactDirJson;
const logPath = $LogPathJson;
const reportPath = $ReportPathJson;
const whale = $WhaleBinJson;
const browserExe = $BrowserExeJson;
const repo = $RepoDirJson;
const model = $ModelJson;
const marker = $MarkerJson;
const markerPrompt = 'Inspect this tiny repository as a normal coding request: identify the files present and say whether there is any test entrypoint. End the final line with this token assembled with no spaces: TASKSPACE_VIEWER_OK + "_" + ' + marker.replace('TASKSPACE_VIEWER_OK_', '') + '.';
const startedAt = Date.now();
const snapshotDir = path.join(artifactDir, 'snapshots');
fs.mkdirSync(snapshotDir, { recursive: true });

let out = '', viewerUrl = null, fetchError = '', browserError = '', browserVersion = '';
let pageOk = false, graphUiOk = false, stateUiOk = false, initialEmptySnapshotOk = false;
let snapshotOk = false, activeMapId = '', snapshotMode = '', mapCount = 0, nodeCount = 0, edgeCount = 0, resultCount = 0;
let entered = false, taskspaceSent = false, showSent = false, promptSent = false, finalShowSent = false, exited = false;
let browserRunning = false, browserInteractionStarted = false, browserInteractionOk = false, browserAttempts = 0;
let detailStateOk = false, selectionStateOk = false, graphZoomOk = false, graphPanOk = false, graphTransformOk = false;
let refreshDuringDetailOk = false, refreshDuringGraphOk = false, refreshDuringSelectionOk = false;
let browserSnapshotStatusOk = false, browserSnapshotActiveOk = false, faviconConsoleErrorCount = 0;
let graphTransformInitial = '', graphTransformAfterZoom = '', graphTransformAfterDrag = '', graphTransformAfterRefresh = '';
let browserRefreshCount = 0, browserSnapshotFirstMs = 0, browserSnapshotLastMs = 0, browserLastHash = '';
let pageErrors = [], consoleErrors = [], networkFailures = [], commandTimeline = [];

function write(data) {
  out += data;
  fs.writeFileSync(logPath, out, 'utf8');
  const matches = [...out.matchAll(/(?:Action Map|TaskSpace) viewer:\s+(http:\/\/127\.0\.0\.1:\d+\/)/g)];
  if (matches.length) viewerUrl = matches[matches.length - 1][1];
}
function hashText(text) { return crypto.createHash('sha256').update(text).digest('hex'); }
function stats(snapshot) {
  const maps = Array.isArray(snapshot?.maps) ? snapshot.maps : [];
  return {
    mode: snapshot?.mode || '',
    activeMapId: snapshot?.activeMapId || '',
    mapCount: maps.length,
    nodeCount: maps.reduce((n, m) => n + (Array.isArray(m.nodes) ? m.nodes.length : 0), 0),
    edgeCount: maps.reduce((n, m) => n + (Array.isArray(m.edges) ? m.edges.length : 0), 0),
    resultCount: maps.reduce((n, m) => n + (Array.isArray(m.results) ? m.results.length : 0), 0)
  };
}
function saveJson(name, value) {
  fs.writeFileSync(path.join(snapshotDir, name + '.json'), JSON.stringify(value, null, 2), 'utf8');
}
function applyStats(s) {
  snapshotMode = s.mode; activeMapId = s.activeMapId; mapCount = s.mapCount;
  nodeCount = s.nodeCount; edgeCount = s.edgeCount; resultCount = s.resultCount;
  snapshotOk = snapshotMode === 'experiment' && mapCount > 0 && nodeCount > 0 && resultCount > 0 && !!activeMapId;
}
function markerCount() { return (out.match(new RegExp(marker, 'g')) || []).length; }
function assistantMarkerObserved() { return !markerPrompt.includes(marker) && out.includes(marker); }
function recordCommand(name) { commandTimeline.push({ atMs: Date.now() - startedAt, command: name }); }
function cmd(text) { recordCommand(text); term.write(text + '\r'); setTimeout(() => term.write('\r'), 350); }

async function probeViewer() {
  if (!viewerUrl) return;
  try {
    const html = await (await fetch(viewerUrl, { cache: 'no-store' })).text();
    graphUiOk = html.includes("className='graph'") && html.includes('pointerdown') && html.includes('wheel to zoom');
    stateUiOk = html.includes("details[data-key]") && html.includes('saveUi()') && html.includes('restoreUi()');
    pageOk = html.includes("fetch('/snapshot.json'") && html.includes('setInterval(refresh,2000)') && graphUiOk && stateUiOk;
    const json = await (await fetch(viewerUrl + 'snapshot.json', { cache: 'no-store' })).json();
    const s = stats(json.snapshot);
    applyStats(s);
    if (json.ok === true && s.mode === 'experiment' && s.mapCount === 0 && !s.activeMapId) {
      initialEmptySnapshotOk = true;
      saveJson('node-initial-empty-snapshot', json);
    }
    if (snapshotOk) saveJson('node-active-snapshot', json);
    fetchError = json.error || '';
    if (snapshotOk && !browserRunning && !browserInteractionOk && browserAttempts < 3) {
      browserRunning = true; browserInteractionStarted = true; browserAttempts++;
      probeBrowserInteraction().catch(err => { browserError = String(err?.stack || err); }).finally(() => { browserRunning = false; });
    }
  } catch (err) { fetchError = String(err); }
}
function parseMatrix(value) {
  if (!value || value === 'none') return { scale: 1, x: 0, y: 0 };
  const nums = (value.match(/-?\d+(?:\.\d+)?/g) || []).map(Number);
  return nums.length >= 6 ? { scale: nums[0], x: nums[4], y: nums[5] } : { scale: 1, x: 0, y: 0 };
}
function waitForSnapshotAfter(events, count, label, timeout = 9000) {
  const start = Date.now();
  return new Promise((resolve, reject) => {
    const tick = () => {
      if (events.length > count) return resolve(events[events.length - 1]);
      if (Date.now() - start > timeout) return reject(new Error('timed out waiting for browser snapshot after ' + label));
      setTimeout(tick, 100);
    };
    tick();
  });
}
async function probeBrowserInteraction() {
  let browser = null;
  const events = [];
  try {
    browser = await chromium.launch({ executablePath: browserExe, headless: true, args: ['--disable-gpu', '--no-sandbox'] });
    browserVersion = await browser.version();
    const page = await browser.newPage({ viewport: { width: 1366, height: 900 } });
    page.on('console', msg => {
      if (msg.type() !== 'error') return;
      const url = msg.location()?.url || '';
      if (/favicon/i.test(url)) { faviconConsoleErrorCount++; return; }
      consoleErrors.push(url ? msg.text() + ' @ ' + url : msg.text());
    });
    page.on('pageerror', err => pageErrors.push(String(err)));
    page.on('requestfailed', req => networkFailures.push(req.url() + ' ' + (req.failure()?.errorText || 'failed')));
    page.on('response', async res => {
      if (!res.url().endsWith('/snapshot.json')) return;
      const text = await res.text();
      const json = JSON.parse(text);
      const event = { index: events.length + 1, status: res.status(), fetchedAtMs: json.fetchedAtMs || 0, hash: hashText(text), stats: stats(json.snapshot) };
      events.push(event); saveJson('browser-snapshot-' + event.index, json);
    });
    await page.goto(viewerUrl, { waitUntil: 'domcontentloaded' });
    await waitForSnapshotAfter(events, 0, 'page-load');
    await page.waitForSelector('.graph-world', { timeout: 12000 });
    await page.waitForSelector('details[data-key^="nodes:"]', { timeout: 12000 });

    const details = page.locator('details[data-key^="nodes:"]').first();
    await details.locator('summary').click();
    const detailOpenBefore = await details.evaluate(n => n.open);
    const detailCount = events.length;
    await waitForSnapshotAfter(events, detailCount, 'detail-open');
    detailStateOk = detailOpenBefore && await details.evaluate(n => n.open);
    refreshDuringDetailOk = events.length > detailCount;

    const graph = page.locator('.graph').first();
    const world = page.locator('.graph-world').first();
    graphTransformInitial = await world.evaluate(n => getComputedStyle(n).transform);
    await graph.locator('button', { hasText: '+' }).click();
    graphTransformAfterZoom = await world.evaluate(n => getComputedStyle(n).transform);
    const box = await graph.boundingBox();
    if (!box) throw new Error('graph bounding box unavailable');
    await page.mouse.move(box.x + 360, box.y + 230); await page.mouse.down();
    await page.mouse.move(box.x + 420, box.y + 270); await page.mouse.up();
    graphTransformAfterDrag = await world.evaluate(n => getComputedStyle(n).transform);
    const graphCount = events.length;
    await waitForSnapshotAfter(events, graphCount, 'graph-transform');
    graphTransformAfterRefresh = await world.evaluate(n => getComputedStyle(n).transform);
    const initial = parseMatrix(graphTransformInitial), zoomed = parseMatrix(graphTransformAfterZoom), dragged = parseMatrix(graphTransformAfterDrag);
    graphZoomOk = zoomed.scale > initial.scale;
    graphPanOk = Math.abs(dragged.x - zoomed.x) > 10 || Math.abs(dragged.y - zoomed.y) > 10;
    graphTransformOk = graphZoomOk && graphPanOk && graphTransformAfterRefresh === graphTransformAfterDrag;
    refreshDuringGraphOk = events.length > graphCount;

    await page.locator('#meta').evaluate(node => {
      const range = document.createRange(); range.selectNodeContents(node);
      const selection = window.getSelection(); selection.removeAllRanges(); selection.addRange(range);
    });
    const selectedBefore = await page.evaluate(() => String(window.getSelection()));
    const selectionCount = events.length;
    await waitForSnapshotAfter(events, selectionCount, 'active-selection');
    const selectedAfter = await page.evaluate(() => String(window.getSelection()));
    selectionStateOk = selectedBefore.includes('thread ') && selectedAfter === selectedBefore && selectedAfter.length > 10;
    refreshDuringSelectionOk = events.length > selectionCount;

    browserRefreshCount = events.length;
    browserSnapshotFirstMs = events[0]?.fetchedAtMs || 0;
    browserSnapshotLastMs = events[events.length - 1]?.fetchedAtMs || 0;
    browserLastHash = events[events.length - 1]?.hash || '';
    browserSnapshotStatusOk = events.length >= 4 && events.every(e => e.status === 200);
    browserSnapshotActiveOk = events.some(e => e.stats?.activeMapId && e.stats.nodeCount > 0 && e.stats.resultCount > 0);
    browserInteractionOk = detailStateOk && selectionStateOk && graphTransformOk && refreshDuringDetailOk && refreshDuringGraphOk && refreshDuringSelectionOk && browserSnapshotStatusOk && browserSnapshotActiveOk;
    saveJson('browser-summary', { browserRefreshCount, browserSnapshotFirstMs, browserSnapshotLastMs, browserLastHash, browserSnapshotStatusOk, browserSnapshotActiveOk, events, consoleErrors, faviconConsoleErrorCount, pageErrors, networkFailures });
  } finally { if (browser) await browser.close(); }
}

function report(overall, exitCode, failure) {
  const sawStub = out.includes('Not available in TUI yet');
  const lines = [
    '# TUI TaskSpace Viewer E2E', '',
    '- overall: ' + overall, '- exit_code: ' + exitCode,
    '- started_at_ms: ' + startedAt, '- finished_at_ms: ' + Date.now(),
    '- duration_ms: ' + (Date.now() - startedAt), '- viewer_url: ' + (viewerUrl || ''),
    '- node_version: ' + process.version, '- playwright_version: ' + playwrightPackage.version,
    '- browser_executable: ' + browserExe, '- browser_version: ' + browserVersion,
    '- page_ok: ' + pageOk, '- graph_ui_ok: ' + graphUiOk, '- state_ui_ok: ' + stateUiOk,
    '- browser_interaction_started: ' + browserInteractionStarted, '- browser_attempts: ' + browserAttempts,
    '- browser_interaction_ok: ' + browserInteractionOk, '- browser_refresh_count: ' + browserRefreshCount,
    '- browser_snapshot_first_ms: ' + browserSnapshotFirstMs, '- browser_snapshot_last_ms: ' + browserSnapshotLastMs,
    '- browser_snapshot_status_ok: ' + browserSnapshotStatusOk, '- browser_snapshot_active_ok: ' + browserSnapshotActiveOk,
    '- browser_last_hash: ' + browserLastHash, '- detail_state_ok: ' + detailStateOk,
    '- selection_state_ok: ' + selectionStateOk, '- graph_zoom_ok: ' + graphZoomOk,
    '- graph_pan_ok: ' + graphPanOk, '- graph_transform_ok: ' + graphTransformOk,
    '- refresh_during_detail_ok: ' + refreshDuringDetailOk, '- refresh_during_graph_ok: ' + refreshDuringGraphOk,
    '- refresh_during_selection_ok: ' + refreshDuringSelectionOk, '- graph_transform_initial: ' + graphTransformInitial,
    '- graph_transform_after_zoom: ' + graphTransformAfterZoom, '- graph_transform_after_drag: ' + graphTransformAfterDrag,
    '- graph_transform_after_refresh: ' + graphTransformAfterRefresh, '- snapshot_ok: ' + snapshotOk,
    '- snapshot_mode: ' + snapshotMode, '- snapshot_map_count: ' + mapCount, '- snapshot_node_count: ' + nodeCount,
    '- snapshot_edge_count: ' + edgeCount, '- snapshot_result_count: ' + resultCount, '- active_map_id: ' + activeMapId,
    '- initial_empty_snapshot_ok: ' + initialEmptySnapshotOk, '- user_prompt_contains_marker: ' + markerPrompt.includes(marker),
    '- assistant_marker_observed: ' + assistantMarkerObserved(), '- marker_count: ' + markerCount(),
    '- saw_stub_error: ' + sawStub, '- console_error_count: ' + consoleErrors.length,
    '- favicon_console_error_count: ' + faviconConsoleErrorCount,
    '- page_error_count: ' + pageErrors.length, '- network_failure_count: ' + networkFailures.length,
    '- command_timeline: ' + commandTimeline.map(c => c.command + '@' + c.atMs).join(', '),
    '- snapshot_dir: ' + snapshotDir, '- fetch_error: ' + fetchError, '- browser_error: ' + browserError,
    '- failure: ' + (failure || ''), '- log: ' + logPath, ''
  ];
  fs.writeFileSync(reportPath, lines.join('\n'), 'utf8');
}

const term = pty.spawn(whale, ['--no-alt-screen', '-C', repo, '-m', model, '--dangerously-bypass-approvals-and-sandbox'], {
  name: 'xterm-256color', cols: 160, rows: 48, cwd: repo,
  env: { ...process.env, TERM: 'xterm-256color', WHALE_ACTION_MAP_VIEWER_NO_BROWSER: '1' }
});
term.onData(write);

const deadline = Date.now() + $TimeoutMs;
const timer = setInterval(async () => {
  await probeViewer();
  const elapsed = Date.now() - startedAt;
  if (!entered && elapsed > 2500) { entered = true; term.write('\r'); }
  if (!taskspaceSent && elapsed > 8000) { taskspaceSent = true; cmd('/taskspace'); }
  if (viewerUrl && pageOk && initialEmptySnapshotOk && !promptSent && elapsed > 13000) { promptSent = true; cmd(markerPrompt); }
  if (snapshotOk && promptSent && !showSent) { showSent = true; cmd('/task-show'); }
  if (assistantMarkerObserved() && !finalShowSent) { finalShowSent = true; cmd('/task-show'); }
  const pass = viewerUrl && pageOk && browserInteractionOk && initialEmptySnapshotOk && snapshotOk && assistantMarkerObserved() && !out.includes('Not available in TUI yet');
  if (pass) {
    clearInterval(timer); report('PASS', 0, '');
    try { term.write('/quit\r\r'); } catch {}
    setTimeout(() => process.exit(0), 1500); return;
  }
  if (Date.now() > deadline) {
    clearInterval(timer); report('FAIL', 1, 'timeout');
    try { term.kill(); } catch {}
    setTimeout(() => process.exit(1), 500);
  }
}, 1000);
term.onExit(({ exitCode }) => {
  if (exited) return; exited = true; clearInterval(timer);
  const ok = viewerUrl && pageOk && browserInteractionOk && initialEmptySnapshotOk && snapshotOk && assistantMarkerObserved() && !out.includes('Not available in TUI yet');
  report(ok ? 'PASS' : 'FAIL', exitCode ?? 1, '');
  process.exit(ok ? 0 : 1);
});
"@

Set-Content -LiteralPath $JsPath -Encoding UTF8 -Value $Js

node $JsPath
$ExitCode = $LASTEXITCODE
if (Test-Path -LiteralPath $ReportPath) {
    Get-Content -LiteralPath $ReportPath -Encoding UTF8
}
if ($ExitCode -eq 0) {
    Write-Host "Overall: PASS"
} else {
    Write-Host "Overall: FAIL"
}
if ($ExitCode -ne 0) {
    throw "TUI task-show viewer E2E failed. Artifacts: $ArtifactDir"
}
