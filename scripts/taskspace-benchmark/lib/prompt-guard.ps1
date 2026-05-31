function Invoke-TaskspacePromptGuard {
    param(
        [Parameter(Mandatory = $true)][string]$PromptText,
        [string[]]$AllowedContextTerms = @()
    )
    $hardPatterns = @(
        "(?i)\btaskspace\b",
        "(?i)action\s+map",
        "(?i)\bsubagent\b",
        "(?i)\bspawn_agent\b",
        "(?i)\btaskspace_control\b",
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
    $allowedSpanPatterns = @(
        "(?i)\bnode\.js\b",
        "(?i)\bsource\s+map\b",
        "(?i)\bparallel\s+tests?\b",
        "(?i)\bperformance\s+benchmark\b"
    ) + $AllowedContextTerms
    $allowedSpans = @()
    foreach ($allowedPattern in $allowedSpanPatterns) {
        foreach ($match in [regex]::Matches($PromptText, $allowedPattern)) {
            $allowedSpans += [pscustomobject]@{ Start = $match.Index; End = $match.Index + $match.Length; Text = [string]$match.Value }
        }
    }
    $hardHits = New-Object System.Collections.Generic.List[string]
    foreach ($pattern in $hardPatterns) {
        if ($PromptText -match $pattern) { $hardHits.Add($Matches[0]) }
    }
    $contextHits = New-Object System.Collections.Generic.List[string]
    $allowedHits = New-Object System.Collections.Generic.List[string]
    foreach ($pattern in $contextPatterns) {
        foreach ($match in [regex]::Matches($PromptText, $pattern)) {
            $text = [string]$match.Value
            $allowed = $false
            foreach ($span in @($allowedSpans)) {
                if ($match.Index -ge $span.Start -and ($match.Index + $match.Length) -le $span.End) {
                    $allowed = $true
                    break
                }
            }
            if ($allowed) { $allowedHits.Add($text) } else { $contextHits.Add($text) }
        }
    }
    [pscustomobject]@{
        invalid_prompt = $hardHits.Count -gt 0
        manual_review_required = ($hardHits.Count -eq 0 -and $contextHits.Count -gt 0)
        hard_hits = @($hardHits.ToArray())
        context_hits = @($contextHits.ToArray())
        allowed_context_hits = @($allowedHits.ToArray())
    }
}
