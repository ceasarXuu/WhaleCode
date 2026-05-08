param(
    [Parameter(Mandatory = $true)]
    [string]$RolloutPath,
    [Parameter(Mandatory = $true)]
    [string]$JsonlPath,
    [Parameter(Mandatory = $true)]
    [string]$OutputDir
)

$ErrorActionPreference = "Stop"

function Read-JsonLines([string]$PathValue) {
    $items = New-Object System.Collections.Generic.List[object]
    if (-not (Test-Path $PathValue)) {
        return $items
    }

    foreach ($line in Get-Content -LiteralPath $PathValue -Encoding UTF8) {
        if ([string]::IsNullOrWhiteSpace($line)) {
            continue
        }
        try {
            $items.Add(($line | ConvertFrom-Json))
        }
        catch {
        }
    }
    return $items
}

function Add-TimelineEvent {
    param(
        [System.Collections.Generic.List[object]]$Timeline,
        [string]$At,
        [string]$Kind,
        [string]$Summary,
        [object]$Details
    )

    $Timeline.Add([ordered]@{
        at = $At
        kind = $Kind
        summary = $Summary
        details = $Details
    })
}

function Ensure-Node {
    param(
        [hashtable]$Nodes,
        [string]$NodeId,
        [string]$Title = ""
    )

    if ([string]::IsNullOrWhiteSpace($NodeId)) {
        return $null
    }
    if (-not $Nodes.ContainsKey($NodeId)) {
        $Nodes[$NodeId] = [ordered]@{
            id = $NodeId
            title = $Title
            status = "unknown"
            leases = New-Object System.Collections.Generic.List[object]
            results = New-Object System.Collections.Generic.List[object]
            agentThreads = New-Object System.Collections.Generic.List[string]
            events = New-Object System.Collections.Generic.List[object]
        }
    }
    elseif ($Title -and -not $Nodes[$NodeId].title) {
        $Nodes[$NodeId].title = $Title
    }
    return $Nodes[$NodeId]
}

function Escape-Html([string]$Text) {
    return [System.Net.WebUtility]::HtmlEncode($Text)
}

$output = New-Item -ItemType Directory -Force -Path $OutputDir
$rolloutItems = Read-JsonLines $RolloutPath
$jsonlItems = Read-JsonLines $JsonlPath

$timeline = New-Object System.Collections.Generic.List[object]
$maps = New-Object System.Collections.Generic.List[object]
$nodes = @{}
$agents = @{}
$toolCalls = New-Object System.Collections.Generic.List[object]

foreach ($item in $rolloutItems) {
    $payload = $item.payload
    if (-not $payload -or $payload.type -notin @(
            "mode_changed",
            "map_created",
            "node_status_changed",
            "lease_created",
            "lease_attached",
            "node_result_recorded",
            "lease_released",
            "timeout_summary_requested"
        )) {
        continue
    }

    $kind = [string]$payload.type
    $at = [string]$item.timestamp
    switch ($kind) {
        "mode_changed" {
            Add-TimelineEvent $timeline $at $kind "mode changed: $($payload.previousMode) -> $($payload.currentMode)" $payload
        }
        "map_created" {
            $maps.Add([ordered]@{
                id = [string]$payload.mapId
                title = [string]$payload.title
                ownerSessionId = [string]$payload.ownerSessionId
                createdFrom = $payload.createdFrom
            })
            Add-TimelineEvent $timeline $at $kind "map created: $($payload.mapId) $($payload.title)" $payload
        }
        "node_status_changed" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId) ([string]$payload.nodeTitle)
            if ($node) {
                $node.status = [string]$payload.currentStatus
                $node.events.Add([ordered]@{
                    at = $at
                    kind = $kind
                    from = [string]$payload.previousStatus
                    to = [string]$payload.currentStatus
                })
            }
            Add-TimelineEvent $timeline $at $kind "node status: $($payload.nodeId) $($payload.previousStatus) -> $($payload.currentStatus)" $payload
        }
        "lease_created" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node) {
                $node.leases.Add([ordered]@{
                    at = $at
                    leaseId = [string]$payload.leaseId
                    state = "created"
                })
            }
            Add-TimelineEvent $timeline $at $kind "lease created: $($payload.leaseId) on $($payload.nodeId)" $payload
        }
        "lease_attached" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node -and $payload.agentThreadId) {
                $agentId = [string]$payload.agentThreadId
                if (-not $node.agentThreads.Contains($agentId)) {
                    $node.agentThreads.Add($agentId)
                }
                $agents[$agentId] = [ordered]@{
                    threadId = $agentId
                    path = [string]$payload.agentPath
                    nodeId = [string]$payload.nodeId
                    leaseId = [string]$payload.leaseId
                }
            }
            Add-TimelineEvent $timeline $at $kind "agent attached: $($payload.agentPath) -> $($payload.nodeId)" $payload
        }
        "node_result_recorded" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node) {
                $node.results.Add([ordered]@{
                    at = $at
                    resultId = [string]$payload.resultId
                    leaseId = [string]$payload.leaseId
                    sourceThreadId = [string]$payload.sourceThreadId
                    kind = [string]$payload.kind
                })
            }
            Add-TimelineEvent $timeline $at $kind "node result recorded: $($payload.nodeId) / $($payload.resultId)" $payload
        }
        "lease_released" {
            Add-TimelineEvent $timeline $at $kind "lease released: $($payload.leaseId), reason=$($payload.reason)" $payload
        }
        "timeout_summary_requested" {
            Add-TimelineEvent $timeline $at $kind "timeout summary requested: $($payload.agentPath)" $payload
        }
    }
}

foreach ($item in $jsonlItems) {
    $eventItem = $item.item
    if (-not $eventItem -or $eventItem.type -ne "collab_tool_call") {
        continue
    }
    $tool = [string]$eventItem.tool
    $status = [string]$eventItem.status
    $receivers = @()
    if ($eventItem.receiver_thread_ids) {
        $receivers = @($eventItem.receiver_thread_ids | ForEach-Object { [string]$_ })
    }
    $toolCall = [ordered]@{
        type = [string]$item.type
        id = [string]$eventItem.id
        tool = $tool
        status = $status
        senderThreadId = [string]$eventItem.sender_thread_id
        receiverThreadIds = $receivers
        promptPreview = if ($eventItem.prompt) { ([string]$eventItem.prompt).Substring(0, [Math]::Min(600, ([string]$eventItem.prompt).Length)) } else { "" }
    }
    $toolCalls.Add($toolCall)
    Add-TimelineEvent $timeline "" "tool:$tool" "tool call: $tool ($status)" $toolCall
}

$nodeList = @($nodes.Values | Sort-Object id)
$agentList = @($agents.Values | Sort-Object threadId)
$summary = [ordered]@{
    maps = $maps.Count
    nodes = $nodeList.Count
    agents = $agentList.Count
    toolCalls = $toolCalls.Count
    mapRuntimeEvents = @($timeline | Where-Object { $_.kind -notlike "tool:*" }).Count
}

$reduced = [ordered]@{
    generatedAt = (Get-Date).ToString("o")
    source = [ordered]@{
        rolloutPath = (Resolve-Path -LiteralPath $RolloutPath).Path
        jsonlPath = (Resolve-Path -LiteralPath $JsonlPath).Path
    }
    summary = $summary
    maps = @($maps.ToArray())
    nodes = $nodeList
    agents = $agentList
    toolCalls = @($toolCalls.ToArray())
    timeline = @($timeline.ToArray())
}

$jsonPath = Join-Path $output.FullName "action-map-observability.json"
$markdownPath = Join-Path $output.FullName "action-map-observability.md"
$htmlPath = Join-Path $output.FullName "action-map-observability.html"
$json = $reduced | ConvertTo-Json -Depth 30
$json | Set-Content -LiteralPath $jsonPath -Encoding UTF8

$md = New-Object System.Collections.Generic.List[string]
$md.Add("# Action Map Observability")
$md.Add("")
$md.Add("- maps: $($summary.maps)")
$md.Add("- nodes: $($summary.nodes)")
$md.Add("- agents: $($summary.agents)")
$md.Add("- collab tool calls: $($summary.toolCalls)")
$md.Add("- map runtime events: $($summary.mapRuntimeEvents)")
$md.Add("")
$md.Add("## Nodes")
$md.Add("")
$md.Add("| Node | Title | Status | Agents | Results |")
$md.Add("|---|---|---|---|---|")
foreach ($node in $nodeList) {
    $md.Add("| $($node.id) | $($node.title) | $($node.status) | $($node.agentThreads.Count) | $($node.results.Count) |")
}
$md.Add("")
$md.Add("## Timeline")
$md.Add("")
foreach ($event in $reduced.timeline) {
    $md.Add(("- `{0}` **{1}** {2}" -f $event["at"], $event["kind"], $event["summary"]))
}
$md | Set-Content -LiteralPath $markdownPath -Encoding UTF8

$encodedJson = Escape-Html $json
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
    <h2>Map / Node Graph</h2>
    <div class="graph" id="graph"></div>
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
    <h2>Timeline</h2>
    <div class="timeline" id="timeline"></div>
  </section>
</main>
<script type="application/json" id="trace-data">__TRACE_DATA__</script>
<script>
const data = JSON.parse(document.getElementById('trace-data').textContent);
const esc = (v) => String(v ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
document.getElementById('source').innerHTML = `<code>${esc(data.source.rolloutPath)}</code>`;
document.getElementById('stats').innerHTML = Object.entries(data.summary)
  .map(([k, v]) => `<div class="stat"><strong>${esc(v)}</strong>${esc(k)}</div>`).join('');
document.getElementById('graph').innerHTML = data.nodes.map(n => `
  <div class="node ${esc(n.status)}">
    <div class="id">${esc(n.id)}</div>
    <div class="title">${esc(n.title)}</div>
    <div class="meta">status: ${esc(n.status)}</div>
    <div class="meta">agents: ${esc((n.agentThreads || []).join(', ') || '-')}</div>
    <div class="meta">results: ${esc((n.results || []).length)}</div>
  </div>`).join('');
document.getElementById('agents').innerHTML = table(['Thread', 'Path', 'Node', 'Lease'],
  data.agents.map(a => [a.threadId, a.path, a.nodeId, a.leaseId]));
document.getElementById('tools').innerHTML = table(['Tool', 'Status', 'Sender', 'Receivers', 'Prompt Preview'],
  data.toolCalls.map(t => [t.tool, t.status, t.senderThreadId, (t.receiverThreadIds || []).join(', '), t.promptPreview]));
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

Write-Host "ObservabilityJson: $jsonPath"
Write-Host "ObservabilityMarkdown: $markdownPath"
Write-Host "ObservabilityHtml: $htmlPath"
