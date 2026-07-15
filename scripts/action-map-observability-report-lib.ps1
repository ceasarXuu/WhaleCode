. (Join-Path $PSScriptRoot "action-map-object-lib.ps1")

function ConvertTo-HtmlScriptJson([string]$Json) {
    return $Json.Replace("&", "\u0026").
        Replace("<", "\u003C").
        Replace(">", "\u003E").
        Replace([string][char]0x2028, "\u2028").
        Replace([string][char]0x2029, "\u2029")
}

function Join-ReportValues($Value) {
    return (@(Get-ObjectArray $Value) | ForEach-Object { [string]$_ }) -join ", "
}

function Format-MarkdownCell($Value) {
    $text = [string]$Value
    return $text.Replace("|", "\|").Replace("`r`n", "<br>").Replace("`n", "<br>").Replace("`r", "<br>")
}

function Join-MarkdownValues($Value) {
    return Format-MarkdownCell (Join-ReportValues $Value)
}

function Add-MarkdownRows {
    param(
        [System.Collections.Generic.List[string]]$Markdown,
        [object]$Rows
    )
    foreach ($row in @(Get-ObjectArray $Rows)) {
        $Markdown.Add($row)
    }
}

function Get-ReportPropertyPairs($Value) {
    $pairs = New-Object System.Collections.Generic.List[object]
    if ($null -eq $Value) {
        return $pairs
    }
    if ($Value -is [System.Collections.IDictionary]) {
        foreach ($key in $Value.Keys) {
            $pairs.Add([ordered]@{ Name = [string]$key; Value = $Value[$key] })
        }
        return $pairs
    }
    foreach ($property in $Value.PSObject.Properties) {
        $pairs.Add([ordered]@{ Name = [string]$property.Name; Value = $property.Value })
    }
    return $pairs
}

function Write-ActionMapObservabilityReport {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Reduced,
        [Parameter(Mandatory = $true)]
        [string]$OutputDir
    )

    $jsonPath = Join-Path $OutputDir "action-map-observability.json"
    $markdownPath = Join-Path $OutputDir "action-map-observability.md"
    $htmlPath = Join-Path $OutputDir "action-map-observability.html"
    $json = $Reduced | ConvertTo-Json -Depth 30
    $json | Set-Content -LiteralPath $jsonPath -Encoding UTF8

    $summary = $Reduced.summary
    $cognitiveAudit = $Reduced.cognitiveAudit
    $md = New-Object System.Collections.Generic.List[string]
    $md.Add("# Action Map Observability")
    $md.Add("")
    $md.Add("## Source")
    $md.Add("")
    $md.Add("- rollout path: " + $Reduced.source.rolloutPath)
    $md.Add("- jsonl path: " + $Reduced.source.jsonlPath)
    $md.Add("- artifact root: " + $Reduced.source.artifactRoot)
    $md.Add("- rollout parse errors: $($Reduced.source.rolloutReadStats.parseErrorCount)")
    $md.Add("- jsonl parse errors: $($Reduced.source.jsonlReadStats.parseErrorCount)")
    if ($Reduced.source.PSObject.Properties.Name -contains "replay") {
        $md.Add("- canonical replay: $($Reduced.source.replay.availability)")
        $md.Add("- replay error code: $($Reduced.source.replay.error_code)")
        $md.Add("- final snapshot SHA256: $($Reduced.source.replay.final_snapshot_sha256)")
        $md.Add("- active checkpoint/delta: $($Reduced.source.replay.active_checkpoint_id) / $($Reduced.source.replay.active_chain_last_delta_sequence)")
    }
    $md.Add("")
    $md.Add("## Summary")
    $md.Add("")
    $md.Add("- maps: $($summary.maps)")
    $md.Add("- nodes: $($summary.nodes)")
    $md.Add("- edges: $($summary.edges)")
    $md.Add("- agents: $($summary.agents)")
    $md.Add("- collab tool calls: $($summary.toolCalls)")
    $md.Add("- map runtime events: $($summary.mapRuntimeEvents)")
    $md.Add("- output contracts: $($summary.outputContracts)")
    $md.Add("- fact sources: $($summary.factSources)")
    $md.Add("- accepted results: $($summary.acceptedResults)")
    $md.Add("- questioned/invalid results: $($summary.questionedOrInvalidResults)")
    $md.Add("- final artifacts: $($summary.finalArtifacts)")
    $md.Add("- cognitive structural gate: $($summary.cognitiveStructuralGatePassed)")
    $md.Add("- cognitive hard gate: $($summary.cognitiveAuditHardGatePassed)")
    $md.Add("- full MVP hard gate implemented: $($cognitiveAudit.fullMvpHardGateImplemented)")
    $md.Add("")
    $md.Add("## Cognitive Audit")
    $md.Add("")
    $md.Add("- audit schema: $($cognitiveAudit.auditSchemaVersion)")
    $md.Add("- audit scope: $($cognitiveAudit.auditScope)")
    $md.Add("- promotion_not_in_mvp: $($cognitiveAudit.promotionNotInMvp)")
    if (@($cognitiveAudit.hardGateFailures).Count -eq 0) {
        $md.Add("- hard gate failures: none")
    }
    else {
        $md.Add("- hard gate failures: $(@($cognitiveAudit.hardGateFailures) -join ', ')")
    }
    $md.Add("- unsupported MVP gates: $(@($cognitiveAudit.unsupportedMvpGateIds) -join ', ')")
    $md.Add("")
    $md.Add("### Gate Records")
    $md.Add("")
    $md.Add("| Gate | Pass | Expected | Observed | Subject IDs |")
    $md.Add("|---|---|---|---|---|")
    foreach ($gate in @(Get-ObjectArray $cognitiveAudit.gateRecords)) {
        $md.Add("| $(Format-MarkdownCell $gate.gateId) | $(Format-MarkdownCell $gate.pass) | $(Format-MarkdownCell $gate.expected) | $(Format-MarkdownCell $gate.observed) | $(Join-MarkdownValues $gate.subjectIds) |")
    }
    $md.Add("")
    $md.Add("### Metrics")
    $md.Add("")
    $md.Add("| Metric | Value |")
    $md.Add("|---|---|")
    foreach ($metric in @(Get-ReportPropertyPairs $cognitiveAudit.metrics)) {
        $md.Add("| $(Format-MarkdownCell $metric.Name) | $(Format-MarkdownCell $metric.Value) |")
    }
    $md.Add("")
    $md.Add("## Tasks")
    $md.Add("")
    $md.Add("| Task | Status | Title | Objective | Output Contracts | Fact Sources | Facts |")
    $md.Add("|---|---|---|---|---|---|---|")
    foreach ($task in $Reduced.tasks) {
        $cognitive = $task.cognitiveState
        $md.Add("| $(Format-MarkdownCell $task.id) | $(Format-MarkdownCell $task.status) | $(Format-MarkdownCell $task.title) | $(Format-MarkdownCell $task.objective) | $(@($cognitive.outputContracts).Count) | $(@($cognitive.factSources).Count) | $(@($cognitive.facts).Count) |")
    }
    $md.Add("")
    $md.Add("## Nodes")
    $md.Add("")
    $md.Add("| Node | Kind | Title | Status | Agents | Results | Accepted | Questioned/Invalid | Blocked Actions | Active Barriers |")
    $md.Add("|---|---|---|---|---|---|---|---|---|---|")
    foreach ($node in $Reduced.nodes) {
        $blockedCount = 0
        if ($null -ne $node["blockedActions"]) { $blockedCount = [int]$node["blockedActions"].Count }
        $barrierCount = @($node.maintenanceBarriers | Where-Object { $_.state -eq "active" }).Count
        $acceptedCount = @($node.results | Where-Object { [string]$_.validity -eq "accepted" }).Count
        $questionedInvalidCount = @($node.results | Where-Object { [string]$_.validity -in @("questioned", "invalid") }).Count
        $md.Add("| $(Format-MarkdownCell $node.id) | $(Format-MarkdownCell $node.kind) | $(Format-MarkdownCell $node.title) | $(Format-MarkdownCell $node.status) | $($node.agentThreads.Count) | $($node.results.Count) | $acceptedCount | $questionedInvalidCount | $blockedCount | $barrierCount |")
    }
    $md.Add("")
    $md.Add("## Final Artifacts")
    $md.Add("")
    $md.Add("| Artifact | Task | Path | Hash | Results | Contracts | Claims | Evidence Refs | Validators | Fact Sources | Sentinels |")
    $md.Add("|---|---|---|---|---|---|---|---|---|---|---|")
    foreach ($artifact in @(Get-ObjectArray $Reduced.finalArtifacts)) {
        $hash = [string]$artifact.artifactHash
        if ($hash.Length -gt 16) { $hash = $hash.Substring(0, 16) }
        $md.Add("| $(Format-MarkdownCell $artifact.finalArtifactId) | $(Format-MarkdownCell $artifact.taskId) | $(Format-MarkdownCell $artifact.finalArtifactPath) | $(Format-MarkdownCell $hash) | $(Format-MarkdownCell (Join-ReportValues $artifact.resultIds)) | $(Format-MarkdownCell (Join-ReportValues $artifact.outputContractIds)) | $(Format-MarkdownCell (Join-ReportValues $artifact.claimIds)) | $(Format-MarkdownCell (Join-ReportValues $artifact.evidenceRefIds)) | $(Format-MarkdownCell (Join-ReportValues $artifact.validatorRefs)) | $(Format-MarkdownCell (Join-ReportValues $artifact.factSourceIds)) | $(Format-MarkdownCell (Join-ReportValues $artifact.sentinelIds)) |")
    }
    $md.Add("")
    $md.Add("## Result Evidence")
    $md.Add("")
    $md.Add("| Result | Node | Validity | Claims | Evidence Refs | Validators | Reason |")
    $md.Add("|---|---|---|---|---|---|---|")
    foreach ($node in $Reduced.nodes) {
        foreach ($result in @(Get-ObjectArray $node.results)) {
            $ep = Get-ObjectField $result "evidencePackage"
            $claimIds = @(Get-ObjectArray (Get-ObjectField $ep "claims") | ForEach-Object { [string](Get-ObjectField $_ "id") })
            $evidenceRefs = @(Get-ObjectArray (Get-ObjectField $ep "evidenceRefs") | ForEach-Object { ($_ | ConvertTo-Json -Compress -Depth 10) })
            $md.Add("| $(Format-MarkdownCell $result.resultId) | $(Format-MarkdownCell $node.id) | $(Format-MarkdownCell $result.validity) | $(Format-MarkdownCell ($claimIds -join ', ')) | $(Format-MarkdownCell ($evidenceRefs -join '<br>')) | $(Join-MarkdownValues (Get-ObjectField $ep 'validatorRefs')) | $(Format-MarkdownCell (Get-ObjectField $ep 'validityReason')) |")
        }
    }
    $md.Add("")
    $md.Add("## Sentinel Warnings")
    $md.Add("")
    $md.Add("| Sentinel | Type | Severity | Status | Task | Map | Node | Result | Trace Events | Reason | Clear Action | Clearance Guidance | Cleared By | Clear Events |")
    $md.Add("|---|---|---|---|---|---|---|---|---|---|---|---|---|---|")
    foreach ($warning in @(Get-ObjectArray $Reduced.sentinelWarnings)) {
        $md.Add("| $(Format-MarkdownCell $warning.id) | $(Format-MarkdownCell $warning.sentinelType) | $(Format-MarkdownCell $warning.severity) | $(Format-MarkdownCell $warning.status) | $(Format-MarkdownCell $warning.taskId) | $(Format-MarkdownCell $warning.mapId) | $(Format-MarkdownCell $warning.nodeId) | $(Format-MarkdownCell $warning.resultId) | $(Join-MarkdownValues $warning.traceEventIds) | $(Format-MarkdownCell $warning.reason) | $(Format-MarkdownCell $warning.clearAction) | $(Format-MarkdownCell $warning.clearanceAction) | $(Format-MarkdownCell $warning.clearedBy) | $(Join-MarkdownValues $warning.clearEventIds) |")
    }
    $md.Add("")
    $md.Add("## Edges")
    $md.Add("")
    $md.Add("| From | To |")
    $md.Add("|---|---|")
    foreach ($edge in $Reduced.edges) {
        $md.Add("| $(Format-MarkdownCell $edge.from) | $(Format-MarkdownCell $edge.to) |")
    }
    $md.Add("")
    $md.Add("## Timeline")
    $md.Add("")
    foreach ($event in $Reduced.timeline) {
        $md.Add(("- `{0}` **{1}** {2}" -f $event["at"], $event["kind"], $event["summary"]))
    }
    $md.Add("")
    $md.Add("## Known Missing / Future Work")
    $md.Add("")
    $md.Add("- This report validates the MVP final-artifact why-chain by mechanically joining output contracts, results, claims, evidence refs, validators/fact sources, sentinel warnings, and artifact hashes.")
    $md.Add('- Final artifacts are currently derived from artifact output contracts and result `changedArtifacts` / `artifactRef` fields; a dedicated final-artifact runtime event remains future work.')
    $md.Add("- Browser interaction coverage for `/task-show` remains separate from this static export report.")
    $md | Set-Content -LiteralPath $markdownPath -Encoding UTF8

    $encodedJson = ConvertTo-HtmlScriptJson $json
    $htmlTemplate = @'
<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Action Map Observability</title>
<style>
body { margin: 0; font-family: Segoe UI, Arial, sans-serif; color: #1f2937; background: #f6f7f9; }
header { padding: 24px 32px; background: #111827; color: white; }
main { padding: 24px 32px; display: grid; gap: 20px; }
.stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 12px; }
.stat, section { background: white; border: 1px solid #d8dde6; border-radius: 8px; padding: 16px; }
.stat strong { display: block; font-size: 28px; }
.graph { display: flex; flex-wrap: wrap; gap: 10px; align-items: stretch; }
.node { min-width: 190px; border: 1px solid #c7cedb; border-radius: 8px; padding: 12px; background: #fbfcfe; }
.node.completed { border-color: #2f855a; background: #eefaf3; }
.node.running { border-color: #b7791f; background: #fff8e8; }
.node.ready { border-color: #2b6cb0; background: #eef6ff; }
.node .id { font-weight: 700; }
.node .title { color: #4b5563; margin-top: 4px; }
.node .meta { margin-top: 8px; font-size: 12px; color: #6b7280; }
table { width: 100%; border-collapse: collapse; }
th, td { text-align: left; padding: 8px 10px; border-bottom: 1px solid #e5e7eb; vertical-align: top; }
th { background: #f3f4f6; }
.timeline { display: grid; gap: 8px; }
.event { border-left: 4px solid #6b7280; padding: 8px 12px; background: #fafafa; }
.event.result { border-color: #2f855a; }
.event.lease { border-color: #805ad5; }
.event.node { border-color: #2b6cb0; }
.event.tool { border-color: #dd6b20; }
.gate.pass { color: #2f855a; }
.gate.fail { color: #b00020; }
code { background: #eef1f5; padding: 2px 4px; border-radius: 4px; }
</style>
</head>
<body>
<header>
  <h1>Action Map Observability</h1>
  <div id="source"></div>
</header>
<main>
  <div class="stats" id="stats"></div>
  <section>
    <h2>Cognitive Audit</h2>
    <div id="audit"></div>
  </section>
  <section>
    <h2>Tasks</h2>
    <div id="tasks"></div>
  </section>
  <section>
    <h2>Map / Node Graph</h2>
    <div class="graph" id="graph"></div>
  </section>
  <section>
    <h2>Edges</h2>
    <div id="edges"></div>
  </section>
  <section>
    <h2>Agents</h2>
    <div id="agents"></div>
  </section>
  <section>
    <h2>Collaboration Tool Calls</h2>
    <div id="tools"></div>
  </section>
  <section>
    <h2>Sentinel Warnings</h2>
    <div id="sentinels"></div>
  </section>
  <section>
    <h2>Final Artifacts</h2>
    <div id="finalArtifacts"></div>
  </section>
  <section>
    <h2>Result Evidence</h2>
    <div id="resultEvidence"></div>
  </section>
  <section>
    <h2>Timeline</h2>
    <div class="timeline" id="timeline"></div>
  </section>
</main>
<script type="application/json" id="trace-data">__TRACE_DATA__</script>
<script>
const data = JSON.parse(document.getElementById('trace-data').textContent);
const esc = (v) => String(v ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
document.getElementById('source').innerHTML = `<code>${esc(data.source.rolloutPath)}</code><br><span>artifact root: ${esc(data.source.artifactRoot || '')}</span>`;
document.getElementById('stats').innerHTML = Object.entries(data.summary)
  .map(([k, v]) => `<div class="stat"><strong>${esc(v)}</strong>${esc(k)}</div>`).join('');
const audit = data.cognitiveAudit || {};
document.getElementById('audit').innerHTML = `
  <p class="gate ${audit.structuralGatePassed ? 'pass' : 'fail'}">structural gate: ${audit.structuralGatePassed ? 'PASS' : 'FAIL'}</p>
  <p>schema: <code>${esc(audit.auditSchemaVersion || '')}</code> | scope: <code>${esc(audit.auditScope || '')}</code> | full MVP hard gate implemented: ${esc(audit.fullMvpHardGateImplemented)}</p>
  <p>promotion_not_in_mvp: ${esc(audit.promotionNotInMvp)}</p>
  <p>hard gate failures: ${esc((audit.hardGateFailures || []).join(', ') || 'none')}</p>
  <p>unsupported MVP gates: ${esc((audit.unsupportedMvpGateIds || []).join(', ') || 'none')}</p>
  ${table(['Gate','Pass','Expected','Observed','Subject IDs'], (audit.gateRecords || []).map(g => [g.gateId, g.pass, g.expected, g.observed, (g.subjectIds || []).join(', ')]))}
  ${table(['Metric','Value'], Object.entries(audit.metrics || {}))}
`;
document.getElementById('tasks').innerHTML = table(['Task', 'Status', 'Title', 'Objective', 'Contracts', 'Sources', 'Facts'],
  (data.tasks || []).map(t => {
    const c = t.cognitiveState || {};
    return [t.id, t.status, t.title, t.objective, (c.outputContracts || []).length, (c.factSources || []).length, (c.facts || []).length];
  }));
document.getElementById('graph').innerHTML = data.nodes.map(n => `
  <div class="node ${esc(n.status)}">
    <div class="id">${esc(n.id)}</div>
    <div class="title">${esc(n.title)}</div>
    <div class="meta">kind: ${esc(n.kind)}</div>
    <div class="meta">status: ${esc(n.status)}</div>
    <div class="meta">agents: ${esc((n.agentThreads || []).join(', ') || '-')}</div>
    <div class="meta">results: ${esc((n.results || []).length)}</div>
    <div class="meta">accepted: ${esc((n.results || []).filter(r => r.validity === 'accepted').length)}</div>
    <div class="meta">questioned/invalid: ${esc((n.results || []).filter(r => r.validity === 'questioned' || r.validity === 'invalid').length)}</div>
    <div class="meta">blocked: ${esc((n.blockedActions || []).length)}</div>
    <div class="meta">active barriers: ${esc((n.maintenanceBarriers || []).filter(b => b.state === 'active').length)}</div>
    <div class="meta">leases: ${esc((n.leases || []).map(l => `${l.leaseId}:${l.state}`).join(', ') || '-')}</div>
  </div>`).join('');
document.getElementById('edges').innerHTML = table(['From', 'To'], data.edges.map(e => [e.from, e.to]));
document.getElementById('agents').innerHTML = table(['Thread', 'Path', 'Node', 'Lease'],
  data.agents.map(a => [a.threadId, a.path, a.nodeId, a.leaseId]));
document.getElementById('tools').innerHTML = table(['Tool', 'Status', 'Sender', 'Receivers', 'Prompt Preview', 'Output Preview'],
  data.toolCalls.map(t => [t.tool, t.status, t.senderThreadId, (t.receiverThreadIds || []).join(', '), t.promptPreview, t.outputPreview]));
const resultRows = [];
(data.nodes || []).forEach(n => (n.results || []).forEach(r => {
  const ep = r.evidencePackage || {};
  resultRows.push([r.resultId, n.id, r.validity, (ep.claims || []).map(c => c.id).join(', '), JSON.stringify(ep.evidenceRefs || []), (ep.validatorRefs || []).join(', '), ep.validityReason || '']);
}));
document.getElementById('resultEvidence').innerHTML = table(['Result','Node','Validity','Claims','Evidence Refs','Validators','Reason'], resultRows);
document.getElementById('sentinels').innerHTML = table(['Sentinel','Type','Severity','Status','Task','Map','Node','Result','Trace Events','Reason','Clear Action','Clearance Guidance','Cleared By','Clear Events'], (data.sentinelWarnings || []).map(w => [w.id, w.sentinelType, w.severity, w.status, w.taskId, w.mapId, w.nodeId, w.resultId, (w.traceEventIds || []).join(', '), w.reason, w.clearAction, w.clearanceAction, w.clearedBy, (w.clearEventIds || []).join(', ')]));
document.getElementById('finalArtifacts').innerHTML = table(['Artifact','Task','Path','Hash','Results','Contracts','Claims','Evidence','Validators','Sources','Sentinels'],
  (data.finalArtifacts || []).map(a => [a.finalArtifactId, a.taskId, a.finalArtifactPath, String(a.artifactHash || '').slice(0, 16), (a.resultIds || []).join(', '), (a.outputContractIds || []).join(', '), (a.claimIds || []).join(', '), (a.evidenceRefIds || []).join(', '), (a.validatorRefs || []).join(', '), (a.factSourceIds || []).join(', '), (a.sentinelIds || []).join(', ')]));
document.getElementById('timeline').innerHTML = data.timeline.map(e => {
  const cls = e.kind.includes('result') ? 'result' : e.kind.includes('lease') ? 'lease' : e.kind.includes('node') ? 'node' : e.kind.startsWith('tool:') ? 'tool' : '';
  return `<div class="event ${cls}"><code>${esc(e.at || '-')}</code> <strong>${esc(e.kind)}</strong><br>${esc(e.summary)}</div>`;
}).join('');
function table(headers, rows) {
  return `<table><thead><tr>${headers.map(h => `<th>${esc(h)}</th>`).join('')}</tr></thead><tbody>${rows.map(r => `<tr>${r.map(c => `<td>${esc(c)}</td>`).join('')}</tr>`).join('')}</tbody></table>`;
}
</script>
</body>
</html>
'@
    $html = $htmlTemplate.Replace("__TRACE_DATA__", $encodedJson)
    $html | Set-Content -LiteralPath $htmlPath -Encoding UTF8

    return [ordered]@{
        Json = $jsonPath
        Markdown = $markdownPath
        Html = $htmlPath
    }
}
