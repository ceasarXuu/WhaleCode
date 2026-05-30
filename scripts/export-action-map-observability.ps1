param(
    [Parameter(Mandatory = $true)]
    [string]$RolloutPath,
    [Parameter(Mandatory = $true)]
    [string]$JsonlPath,
    [Parameter(Mandatory = $true)]
    [string]$OutputDir
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "action-map-observability-lib.ps1")

$output = New-Item -ItemType Directory -Force -Path $OutputDir
$rolloutItems = Read-JsonLines $RolloutPath
$jsonlItems = Read-JsonLines $JsonlPath

$timeline = New-Object System.Collections.Generic.List[object]
$maps = New-Object System.Collections.Generic.List[object]
$mapById = @{}
$nodes = @{}
$agents = @{}
$edges = New-Object System.Collections.Generic.List[object]
$edgeKeys = @{}
$toolCalls = New-Object System.Collections.Generic.List[object]
$toolCallById = @{}
$collabToolNames = @("spawn_agent", "wait_agent", "close_agent", "resume_agent")

foreach ($item in $rolloutItems) {
    $payload = $item.payload
    if (-not $payload -or $payload.type -notin @(
            "mode_changed",
            "map_created",
            "node_status_changed",
            "lease_created",
            "lease_attached",
            "node_result_recorded",
            "tool_action_blocked",
            "lease_released",
            "timeout_summary_requested",
            "maintenance_barrier_raised",
            "maintenance_barrier_cleared",
            "snapshot_updated"
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
            [void](Ensure-Map $maps $mapById ([string]$payload.mapId) ([string]$payload.title) ([string]$payload.ownerSessionId) $payload.createdFrom)
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
                Add-Or-Update-Lease $node $at ([string]$payload.leaseId) "created"
            }
            Add-TimelineEvent $timeline $at $kind "lease created: $($payload.leaseId) on $($payload.nodeId)" $payload
        }
        "lease_attached" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node -and $payload.agentThreadId) {
                $agentId = [string]$payload.agentThreadId
                Add-Or-Update-Lease $node $at ([string]$payload.leaseId) "attached" "" $agentId
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
                Add-Or-Update-NodeResult $node $at ([string]$payload.resultId) ([string]$payload.leaseId) ([string]$payload.sourceThreadId) ([string]$payload.kind) ([string]$payload.actionClass)
            }
            $actionClassSuffix = if ($payload.actionClass) { " action=$($payload.actionClass)" } else { "" }
            Add-TimelineEvent $timeline $at $kind "node result recorded: $($payload.nodeId) / $($payload.resultId)$actionClassSuffix" $payload
        }
        "tool_action_blocked" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId) "" ([string]$payload.nodeKind)
            if ($node) {
                $node.blockedActions.Add([ordered]@{
                    at = $at
                    toolName = [string]$payload.toolName
                    actionClass = [string]$payload.actionClass
                    reason = [string]$payload.reason
                })
            }
            Add-TimelineEvent $timeline $at $kind "tool action blocked: $($payload.nodeId) $($payload.actionClass) via $($payload.toolName)" $payload
        }
        "lease_released" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node) {
                Add-Or-Update-Lease $node $at ([string]$payload.leaseId) "released" ([string]$payload.reason)
            }
            Add-TimelineEvent $timeline $at $kind "lease released: $($payload.leaseId), reason=$($payload.reason)" $payload
        }
        "timeout_summary_requested" {
            Add-TimelineEvent $timeline $at $kind "timeout summary requested: $($payload.agentPath)" $payload
        }
        "maintenance_barrier_raised" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node) {
                Add-Or-Update-MaintenanceBarrier $node $at ([string]$payload.mapId) ([string]$payload.reason) ([int]$payload.resultCount) ([int]$payload.budget) "active"
                $node.events.Add([ordered]@{
                    at = $at
                    kind = $kind
                    reason = [string]$payload.reason
                    resultCount = [int]$payload.resultCount
                    budget = [int]$payload.budget
                })
            }
            Add-TimelineEvent $timeline $at $kind "maintenance barrier raised: $($payload.nodeId) $($payload.resultCount)/$($payload.budget)" $payload
        }
        "maintenance_barrier_cleared" {
            $node = Ensure-Node $nodes ([string]$payload.nodeId)
            if ($node) {
                Add-Or-Update-MaintenanceBarrier $node $at ([string]$payload.mapId) ([string]$payload.reason) -1 -1 "cleared"
                $node.events.Add([ordered]@{
                    at = $at
                    kind = $kind
                    reason = [string]$payload.reason
                })
            }
            Add-TimelineEvent $timeline $at $kind "maintenance barrier cleared: $($payload.nodeId), reason=$($payload.reason)" $payload
        }
        "snapshot_updated" {
            $snapshotMapCount = 0
            $snapshotNodeCount = 0
            foreach ($snapshotMap in @($payload.snapshot.maps)) {
                $snapshotMapCount++
                [void](Ensure-Map $maps $mapById ([string]$snapshotMap.id) ([string]$snapshotMap.title) ([string]$snapshotMap.ownerSessionId) $snapshotMap.createdFrom)
                foreach ($snapshotEdge in @($snapshotMap.edges)) {
                    $from = [string]$snapshotEdge.from
                    $to = [string]$snapshotEdge.to
                    $mapId = [string]$snapshotMap.id
                    $edgeKey = "$mapId|$from|$to"
                    if ($from -and $to -and -not $edgeKeys.ContainsKey($edgeKey)) {
                        $edgeKeys[$edgeKey] = $true
                        $edges.Add([ordered]@{
                            mapId = $mapId
                            from = $from
                            to = $to
                        })
                    }
                }
                foreach ($snapshotNode in @($snapshotMap.nodes)) {
                    $snapshotNodeCount++
                    $node = Ensure-Node $nodes ([string]$snapshotNode.id) ([string]$snapshotNode.title) ([string]$snapshotNode.kind)
                    if ($node) {
                        if ($snapshotNode.status) { $node.status = [string]$snapshotNode.status }
                    }
                }
                foreach ($snapshotResult in @($snapshotMap.results)) {
                    $node = Ensure-Node $nodes ([string]$snapshotResult.nodeId)
                    Add-Or-Update-NodeResult $node $at ([string]$snapshotResult.id) ([string]$snapshotResult.assignmentId) ([string]$snapshotResult.sourceThreadId) ([string]$snapshotResult.kind) ([string]$snapshotResult.actionClass) ([string]$snapshotResult.body)
                }
            }
            foreach ($snapshotBarrier in @($payload.snapshot.maintenanceBarriers)) {
                $node = Ensure-Node $nodes ([string]$snapshotBarrier.nodeId)
                Add-Or-Update-MaintenanceBarrier $node $at ([string]$snapshotBarrier.mapId) ([string]$snapshotBarrier.reason) ([int]$snapshotBarrier.resultCount) ([int]$snapshotBarrier.budget) "active"
            }
            Add-TimelineEvent $timeline $at $kind "snapshot updated: maps=$snapshotMapCount nodes=$snapshotNodeCount" $payload
        }
    }
}

foreach ($item in $rolloutItems) {
    if ($item.type -ne "response_item" -or -not $item.payload) {
        continue
    }
    $payload = $item.payload
    $at = [string]$item.timestamp
    if ($payload.type -eq "function_call") {
        $tool = [string]$payload.name
        if ($collabToolNames -notcontains $tool) {
            continue
        }
        $callId = [string]$payload.call_id
        $promptPreview = ""
        $receivers = @()
        try {
            $args = [string]$payload.arguments | ConvertFrom-Json
            if ($args.message) { $promptPreview = [string]$args.message }
            elseif ($args.prompt) { $promptPreview = [string]$args.prompt }
            if ($args.targets) { $receivers = @($args.targets | ForEach-Object { [string]$_ }) }
            elseif ($args.target) { $receivers = @([string]$args.target) }
        } catch {
            $promptPreview = [string]$payload.arguments
        }
        if ($promptPreview.Length -gt 600) {
            $promptPreview = $promptPreview.Substring(0, 600)
        }
        [void](Add-Or-Update-ToolCall $toolCalls $toolCallById $at $callId $tool "in_progress" "" $receivers $promptPreview "")
        Add-TimelineEvent $timeline $at "tool:$tool" "tool call: $tool (in_progress)" $payload
    }
    elseif ($payload.type -eq "function_call_output") {
        $callId = [string]$payload.call_id
        if (-not $toolCallById.ContainsKey($callId)) {
            continue
        }
        $existingTool = [string]$toolCallById[$callId].tool
        $toolOutput = [string]$payload.output
        $receivers = @()
        $status = "completed"
        $structuredSuccess = $false
        try {
            $parsedOutput = $toolOutput | ConvertFrom-Json
            if ($parsedOutput.agent_id) {
                $receivers = @([string]$parsedOutput.agent_id)
                $structuredSuccess = $true
            }
            elseif ($parsedOutput.task_name) {
                $receivers = @([string]$parsedOutput.task_name)
                $structuredSuccess = $true
            }
            elseif ($parsedOutput.status) {
                $receivers = @($parsedOutput.status.PSObject.Properties.Name | ForEach-Object { [string]$_ })
                $structuredSuccess = $true
            }
            if ($parsedOutput.timed_out -eq $true) {
                $status = "timed_out"
            }
        } catch {
            if ($toolOutput -match "(?i)\b(error|failed|not found|TaskSpace mode has multiple ready nodes|Call spawn_agent with|blocked this tool call)\b") {
                $status = "failed"
            }
        }
        if ($existingTool -eq "spawn_agent" -and -not $structuredSuccess) {
            $status = "failed"
        }
        $preview = $toolOutput
        if ($preview.Length -gt 600) {
            $preview = $preview.Substring(0, 600)
        }
        $updated = Add-Or-Update-ToolCall $toolCalls $toolCallById $at $callId "" $status "" $receivers "" $preview
        if ($updated) {
            Add-TimelineEvent $timeline $at "tool:$($updated.tool)" "tool call: $($updated.tool) ($status)" $payload
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
        at = ""
        id = [string]$eventItem.id
        tool = $tool
        status = $status
        senderThreadId = [string]$eventItem.sender_thread_id
        receiverThreadIds = $receivers
        promptPreview = if ($eventItem.prompt) { ([string]$eventItem.prompt).Substring(0, [Math]::Min(600, ([string]$eventItem.prompt).Length)) } else { "" }
        outputPreview = ""
    }
    if ($status -in @("completed", "in_progress") -and (Has-TimestampedToolCallWithStatus $toolCalls $tool $status)) {
        continue
    }
    if ($status -eq "in_progress" -and (Has-TimestampedToolCall $toolCalls $tool)) {
        continue
    }
    if (Has-TimestampedToolCallDuplicate $toolCalls $toolCall) {
        continue
    }
    $isNewToolCall = -not $toolCallById.ContainsKey([string]$eventItem.id)
    [void](Add-Or-Update-ToolCall $toolCalls $toolCallById "" ([string]$eventItem.id) $tool $status ([string]$eventItem.sender_thread_id) $receivers $toolCall.promptPreview "")
    if ($isNewToolCall) {
        Add-TimelineEvent $timeline "" "tool:$tool" "tool call: $tool ($status)" $toolCall
    }
}

$nodeList = @($nodes.Values | Sort-Object id)
$agentList = @($agents.Values | Sort-Object threadId)
$blockedToolActionCount = 0
foreach ($node in $nodeList) {
    $blockedActions = $node["blockedActions"]
    if ($null -ne $blockedActions) {
        $blockedToolActionCount += [int]$blockedActions.Count
    }
}
$summary = [ordered]@{
    maps = $maps.Count
    nodes = $nodeList.Count
    edges = $edges.Count
    agents = $agentList.Count
    toolCalls = $toolCalls.Count
    blockedToolActions = $blockedToolActionCount
    activeMaintenanceBarriers = @($nodeList | ForEach-Object {
            @($_.maintenanceBarriers | Where-Object { $_.state -eq "active" })
        }).Count
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
    edges = @($edges.ToArray())
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
$md.Add("- edges: $($summary.edges)")
$md.Add("- agents: $($summary.agents)")
$md.Add("- collab tool calls: $($summary.toolCalls)")
$md.Add("- map runtime events: $($summary.mapRuntimeEvents)")
$md.Add("")
$md.Add("## Nodes")
$md.Add("")
$md.Add("| Node | Kind | Title | Status | Agents | Results | Blocked Actions | Active Barriers |")
$md.Add("|---|---|---|---|---|---|---|---|")
foreach ($node in $nodeList) {
    $blockedCount = 0
    if ($null -ne $node["blockedActions"]) { $blockedCount = [int]$node["blockedActions"].Count }
    $barrierCount = @($node.maintenanceBarriers | Where-Object { $_.state -eq "active" }).Count
    $md.Add("| $($node.id) | $($node.kind) | $($node.title) | $($node.status) | $($node.agentThreads.Count) | $($node.results.Count) | $blockedCount | $barrierCount |")
}
$md.Add("")
$md.Add("## Edges")
$md.Add("")
$md.Add("| From | To |")
$md.Add("|---|---|")
foreach ($edge in $edges) {
    $md.Add("| $($edge.from) | $($edge.to) |")
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
    <div class="meta">kind: ${esc(n.kind)}</div>
    <div class="meta">status: ${esc(n.status)}</div>
    <div class="meta">agents: ${esc((n.agentThreads || []).join(', ') || '-')}</div>
    <div class="meta">results: ${esc((n.results || []).length)}</div>
    <div class="meta">blocked: ${esc((n.blockedActions || []).length)}</div>
    <div class="meta">active barriers: ${esc((n.maintenanceBarriers || []).filter(b => b.state === 'active').length)}</div>
    <div class="meta">leases: ${esc((n.leases || []).map(l => `${l.leaseId}:${l.state}`).join(', ') || '-')}</div>
  </div>`).join('');
document.getElementById('edges').innerHTML = table(['From', 'To'], data.edges.map(e => [e.from, e.to]));
document.getElementById('agents').innerHTML = table(['Thread', 'Path', 'Node', 'Lease'],
  data.agents.map(a => [a.threadId, a.path, a.nodeId, a.leaseId]));
document.getElementById('tools').innerHTML = table(['Tool', 'Status', 'Sender', 'Receivers', 'Prompt Preview', 'Output Preview'],
  data.toolCalls.map(t => [t.tool, t.status, t.senderThreadId, (t.receiverThreadIds || []).join(', '), t.promptPreview, t.outputPreview]));
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
