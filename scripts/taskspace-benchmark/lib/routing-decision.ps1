function Get-TaskspaceScenarioExpectedInt {
    param(
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)][string]$Name,
        [int]$DefaultValue = 0
    )
    if ($null -ne $Manifest.Expected -and $Manifest.Expected.PSObject.Properties.Name -contains $Name) {
        return [int]$Manifest.Expected.$Name
    }
    $DefaultValue
}

function Test-TaskspaceScenarioHasValidator {
    param([Parameter(Mandatory = $true)]$Manifest)
    $null -ne $Manifest.PublicValidation -and
        $Manifest.PublicValidation.PSObject.Properties.Name -contains "command" -and
        -not [string]::IsNullOrWhiteSpace([string]$Manifest.PublicValidation.command)
}

function New-TaskspaceRoutingDecision {
    param(
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)][string]$PromptText
    )
    $maxNodes = Get-TaskspaceScenarioExpectedInt $Manifest "max_taskspace_nodes" 8
    $maxSpawn = Get-TaskspaceScenarioExpectedInt $Manifest "max_taskspace_spawn_agent_calls" 0
    $hasValidator = Test-TaskspaceScenarioHasValidator $Manifest
    $hiddenStrategy = [string]$Manifest.HiddenOracleStrategy
    $level = [string]$Manifest.Level
    $formatSensitive = $hiddenStrategy -match '(?i)(format|parser|stack|call-stack)'
    $largeOutput = $hiddenStrategy -match '(?i)large-output'
    $fileScope = if ($level -eq "L1" -or $maxNodes -le 4) {
        "small"
    } elseif ($level -eq "L2" -or $maxNodes -le 12) {
        "medium"
    } else {
        "large"
    }
    $recommendedMode = "default_compact"
    $confidence = "medium"
    $reasons = New-Object System.Collections.Generic.List[string]
    if ($formatSensitive -and $hasValidator) {
        $recommendedMode = "verification_first"
        $reasons.Add("format_sensitive_validator_visible")
    } elseif ($fileScope -eq "small" -and $hasValidator -and $maxSpawn -eq 0) {
        $recommendedMode = "thin"
        $confidence = "high"
        $reasons.Add("small_scope_validator_visible_no_spawn_expected")
    } elseif ($maxSpawn -gt 0 -or $fileScope -eq "large") {
        $recommendedMode = "default_compact"
        $reasons.Add("multi_step_or_spawn_budget_present")
    } else {
        $reasons.Add("fallback_default_compact")
    }
    if ($largeOutput) { $reasons.Add("large_output_ref_policy_required") }
    [pscustomobject]@{
        schema_version = "TaskShapeRouterV1"
        status = "report_only"
        scenario_id = [string]$Manifest.Id
        recommended_mode = $recommendedMode
        confidence = $confidence
        reason = (@($reasons) -join ";")
        task_prompt_features = [pscustomobject]@{
            file_scope = $fileScope
            output_artifact_required = $true
            format_sensitive = [bool]$formatSensitive
            validator_visible = [bool]$hasValidator
            multi_source = [bool]($maxNodes -gt 4)
            code_patch_required = ($PromptText -match '(?i)(fix|repair|implementation|bug)')
            ambiguity = if ($fileScope -eq "small") { "low" } elseif ($fileScope -eq "medium") { "medium" } else { "high" }
        }
        observed_runtime_features = [pscustomobject]@{
            validator_failure_seen = $false
            large_output_seen = [bool]$largeOutput
            uncertainty_increased = $false
        }
        initial_constraints = [pscustomobject]@{
            subagent_allowed = [bool]($recommendedMode -ne "thin" -and $maxSpawn -gt 0)
            node_budget = $maxNodes
            state_commit_budget = if ($recommendedMode -eq "thin") { 4 } else { 8 }
            large_output_policy = if ($largeOutput) { "ref-only" } else { "standard" }
            must_read_validator_first = [bool]($recommendedMode -eq "verification_first")
        }
    }
}
