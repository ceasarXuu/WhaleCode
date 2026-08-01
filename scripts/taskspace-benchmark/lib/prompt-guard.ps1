function Invoke-TaskspacePromptGuard {
    param(
        [Parameter(Mandatory = $true)][string]$PromptText,
        [string[]]$AllowedContextTerms = @(),
        [object[]]$SourceSpans = @()
    )
    $hardPatterns = @(
        "(?i)\btaskspace\b",
        "(?i)/taskspace\b",
        "(?i)/task-show\b",
        "(?i)action\s+map",
        "(?i)\bspawn_agent\b",
        "(?i)\bspawn\s+subagents?\b",
        "(?i)\bspawn\s+agents?\b",
        "(?i)\bbind_node\b",
        "(?i)\bbind\s+nodes?\b",
        "(?i)\bnode_id\b",
        "(?i)\btask_id\b",
        "(?i)\btaskspace_control\b",
        "(?i)\blease_id\b"
    )
    $softOperationPatterns = @(
        "(?i)\bsubagent\b",
        "(?i)\bmulti-agent\b",
        "(?i)\bmultiple\s+agents?\b",
        "(?i)\bsplit\b.{0,40}\bagents?\b",
        "(?i)\bdelegate\b.{0,40}\bagents?\b"
    )
    $contextPatterns = @(
        "(?i)\bmap\b",
        "(?i)\bnode\b",
        "(?i)\bparallel\b",
        "(?i)\bconcurrent(?:ly)?\b",
        "(?i)\bsimultaneous(?:ly)?\b",
        "(?i)\bdelegation\b",
        "(?i)\bbenchmark\b"
    )
    if (@($SourceSpans).Count -eq 0) {
        $SourceSpans = @([pscustomobject]@{
            source_kind = "user_prompt"
            source_path = ""
            start = 0
            end = $PromptText.Length
        })
    }
    function Get-TaskspacePromptSpan {
        param([int]$Start, [int]$End, [object[]]$Spans)
        foreach ($span in @($Spans)) {
            $spanStart = if ($span.PSObject.Properties.Name -contains "start") { [int]$span.start } else { 0 }
            $spanEnd = if ($span.PSObject.Properties.Name -contains "end") { [int]$span.end } else { 0 }
            if ($Start -ge $spanStart -and $End -le $spanEnd) { return $span }
        }
        $null
    }
    function New-TaskspacePromptHit {
        param([string]$Pattern, [object]$Match, [string]$Class, [object[]]$Spans)
        $start = [int]$Match.Index
        $end = $start + [int]$Match.Length
        $sourceSpan = Get-TaskspacePromptSpan $start $end $Spans
        [pscustomobject]@{
            text = [string]$Match.Value
            pattern = $Pattern
            class = $Class
            start = $start
            end = $end
            source_kind = if ($sourceSpan -and $sourceSpan.PSObject.Properties.Name -contains "source_kind") { [string]$sourceSpan.source_kind } else { "" }
            source_path = if ($sourceSpan -and $sourceSpan.PSObject.Properties.Name -contains "source_path") { [string]$sourceSpan.source_path } else { "" }
            line_start = if ($sourceSpan -and $sourceSpan.PSObject.Properties.Name -contains "line_start") { [int]$sourceSpan.line_start } else { 0 }
            line_end = if ($sourceSpan -and $sourceSpan.PSObject.Properties.Name -contains "line_end") { [int]$sourceSpan.line_end } else { 0 }
            byte_start = if ($sourceSpan -and $sourceSpan.PSObject.Properties.Name -contains "byte_start") { [int]$sourceSpan.byte_start } else { $start }
            byte_end = if ($sourceSpan -and $sourceSpan.PSObject.Properties.Name -contains "byte_end") { [int]$sourceSpan.byte_end } else { $end }
            raw_sha256 = if ($sourceSpan -and $sourceSpan.PSObject.Properties.Name -contains "raw_sha256") { [string]$sourceSpan.raw_sha256 } else { "" }
            adapted_sha256 = if ($sourceSpan -and $sourceSpan.PSObject.Properties.Name -contains "adapted_sha256") { [string]$sourceSpan.adapted_sha256 } else { "" }
        }
    }
    $allowedSpanPatterns = @(
        "(?i)\bnode\.js\b",
        "(?i)\bsource\s+map\b",
        "(?i)\bparallel\s+tests?\b",
        "(?i)\bperformance\s+benchmark\b"
    ) + $AllowedContextTerms
    $allowedSpans = @()
    foreach ($allowedPattern in $allowedSpanPatterns) {
        foreach ($match in [regex]::Matches($PromptText, $allowedPattern)) {
            $start = [int]$match.Index
            $end = $start + [int]$match.Length
            $sourceSpan = Get-TaskspacePromptSpan $start $end $SourceSpans
            $allowSourceScoped = @($AllowedContextTerms).Count -gt 0 -and @($AllowedContextTerms | Where-Object { $allowedPattern -eq $_ }).Count -gt 0
            $sourceAllowed = (-not $allowSourceScoped) -or ($sourceSpan -and [string]$sourceSpan.source_kind -eq "upstream_task")
            if ($sourceAllowed) {
                $allowedSpans += [pscustomobject]@{
                    Start = $start
                    End = $end
                    Text = [string]$match.Value
                    Pattern = $allowedPattern
                    SourceKind = if ($sourceSpan) { [string]$sourceSpan.source_kind } else { "" }
                    SourcePath = if ($sourceSpan) { [string]$sourceSpan.source_path } else { "" }
                }
            }
        }
    }
    $hardHits = New-Object System.Collections.Generic.List[object]
    foreach ($pattern in $hardPatterns) {
        foreach ($match in [regex]::Matches($PromptText, $pattern)) {
            $hardHits.Add((New-TaskspacePromptHit $pattern $match "hard" $SourceSpans))
        }
    }
    $contextHits = New-Object System.Collections.Generic.List[object]
    $allowedHits = New-Object System.Collections.Generic.List[object]
    foreach ($pattern in @($softOperationPatterns + $contextPatterns)) {
        foreach ($match in [regex]::Matches($PromptText, $pattern)) {
            $text = [string]$match.Value
            $allowed = $false
            foreach ($span in @($allowedSpans)) {
                if ($match.Index -ge $span.Start -and ($match.Index + $match.Length) -le $span.End) {
                    $allowed = $true
                    break
                }
            }
            $hitClass = if ($softOperationPatterns -contains $pattern) { "soft_operation" } else { "context" }
            if ($allowed) {
                $allowedHits.Add((New-TaskspacePromptHit $pattern $match "allowed" $SourceSpans))
            } else {
                $contextHits.Add((New-TaskspacePromptHit $pattern $match $hitClass $SourceSpans))
            }
        }
    }
    [pscustomobject]@{
        invalid_prompt = $hardHits.Count -gt 0
        manual_review_required = ($hardHits.Count -eq 0 -and $contextHits.Count -gt 0)
        hard_hits = @($hardHits.ToArray() | ForEach-Object { $_.text })
        context_hits = @($contextHits.ToArray() | ForEach-Object { $_.text })
        allowed_context_hits = @($allowedHits.ToArray() | ForEach-Object { $_.text })
        hard_hit_details = @($hardHits.ToArray())
        context_hit_details = @($contextHits.ToArray())
        allowed_context_hit_details = @($allowedHits.ToArray())
        source_spans = @($SourceSpans)
    }
}
