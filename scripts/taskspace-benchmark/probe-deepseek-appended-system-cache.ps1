param(
    [string]$Model = "deepseek-v4-flash",
    [string]$Endpoint = "https://api.deepseek.com/chat/completions",
    [int]$PersistenceDelayMs = 5000,
    [string]$OutputPath = ""
)

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path

function Import-LocalApiKey {
    if (-not [string]::IsNullOrWhiteSpace($env:DEEPSEEK_API_KEY)) { return }
    $envPath = Join-Path $repoRoot ".env.local"
    if (-not (Test-Path -LiteralPath $envPath -PathType Leaf)) {
        throw "DEEPSEEK_API_KEY is unavailable and .env.local does not exist."
    }
    foreach ($line in Get-Content -LiteralPath $envPath -Encoding UTF8) {
        $trimmed = $line.Trim()
        if ($trimmed.StartsWith("DEEPSEEK_API_KEY=")) {
            $value = $trimmed.Substring("DEEPSEEK_API_KEY=".Length).Trim().Trim('"').Trim("'")
            if (-not [string]::IsNullOrWhiteSpace($value)) {
                $env:DEEPSEEK_API_KEY = $value
                return
            }
        }
    }
    throw "DEEPSEEK_API_KEY is unavailable."
}

function New-StablePrefix([string]$ProbeId, [string]$Arm) {
    $lines = [System.Collections.Generic.List[string]]::new()
    for ($index = 1; $index -le 260; $index++) {
        $lines.Add("$ProbeId $Arm stable cache line $($index.ToString('000')); this content must remain byte-identical across requests in this arm.")
    }
    $lines -join "`n"
}

function Get-Usage($Response) {
    $usage = $Response.usage
    $inputTokens = [int64]$usage.prompt_tokens
    $hitTokens = if ($usage.PSObject.Properties.Name -contains "prompt_cache_hit_tokens") {
        [int64]$usage.prompt_cache_hit_tokens
    } elseif ($usage.prompt_tokens_details -and
        $usage.prompt_tokens_details.PSObject.Properties.Name -contains "cached_tokens") {
        [int64]$usage.prompt_tokens_details.cached_tokens
    } else {
        0
    }
    $missTokens = if ($usage.PSObject.Properties.Name -contains "prompt_cache_miss_tokens") {
        [int64]$usage.prompt_cache_miss_tokens
    } else {
        [Math]::Max(0, $inputTokens - $hitTokens)
    }
    [ordered]@{
        input_tokens = $inputTokens
        cache_hit_tokens = $hitTokens
        cache_miss_tokens = $missTokens
        hit_rate = if (($hitTokens + $missTokens) -gt 0) {
            [Math]::Round($hitTokens / ($hitTokens + $missTokens), 6)
        } else {
            0.0
        }
    }
}

function Invoke-ProbeRequest([object[]]$Messages) {
    $body = [ordered]@{
        model = $Model
        messages = $Messages
        temperature = 0
        max_tokens = 1
        stream = $false
    }
    $response = Invoke-RestMethod `
        -Method Post `
        -Uri $Endpoint `
        -Headers @{ Authorization = "Bearer $env:DEEPSEEK_API_KEY" } `
        -ContentType "application/json" `
        -Body ($body | ConvertTo-Json -Depth 20 -Compress)
    Get-Usage $response
}

function Invoke-Arm([string]$ProbeId, [string]$Arm, [string]$SnapshotRole) {
    $stablePrefix = New-StablePrefix $ProbeId $Arm
    $base = @(
        [ordered]@{ role = "system"; content = "Cache probe $ProbeId. Reply with OK." },
        [ordered]@{ role = "user"; content = "$stablePrefix`nBASE_END" }
    )
    $extension = @(
        $base[0],
        $base[1],
        [ordered]@{ role = "assistant"; content = "OK" },
        [ordered]@{ role = $SnapshotRole; content = "REVISION_SNAPSHOT_2 for $ProbeId $Arm" },
        [ordered]@{ role = "user"; content = "EXTENSION_END; reply OK." }
    )

    $baseUsage = Invoke-ProbeRequest $base
    Start-Sleep -Milliseconds $PersistenceDelayMs
    $extensionUsage = Invoke-ProbeRequest $extension
    Start-Sleep -Milliseconds $PersistenceDelayMs
    $replayUsage = Invoke-ProbeRequest $extension
    [ordered]@{
        arm = $Arm
        snapshot_role = $SnapshotRole
        base = $baseUsage
        first_extension = $extensionUsage
        identical_extension_replay = $replayUsage
    }
}

Import-LocalApiKey
$probeId = "r7-cache-$([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds())"
$natural = Invoke-Arm $probeId "natural-user-append" "user"
$system = Invoke-Arm $probeId "interleaved-system-append" "system"
$result = [ordered]@{
    schema_version = 1
    event_name = "deepseek.appended_system_cache_probe"
    probe_id = $probeId
    generated_at = (Get-Date).ToString("o")
    model = $Model
    endpoint = $Endpoint
    persistence_delay_ms = $PersistenceDelayMs
    natural = $natural
    system = $system
}

if ([string]::IsNullOrWhiteSpace($OutputPath)) {
    $outputDir = Join-Path $repoRoot "target/r7-phase-c-cache-probe/$probeId"
    New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
    $OutputPath = Join-Path $outputDir "result.json"
} else {
    $parent = Split-Path -Parent $OutputPath
    if ($parent) { New-Item -ItemType Directory -Path $parent -Force | Out-Null }
}
$result | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $OutputPath -Encoding UTF8

[pscustomobject]@{
    probe_id = $probeId
    natural_extension_hit_rate = $natural.first_extension.hit_rate
    system_extension_hit_rate = $system.first_extension.hit_rate
    natural_replay_hit_rate = $natural.identical_extension_replay.hit_rate
    system_replay_hit_rate = $system.identical_extension_replay.hit_rate
    output_path = (Resolve-Path -LiteralPath $OutputPath).Path
} | Format-List
