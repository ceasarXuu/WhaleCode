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
    $deepSignal = $hiddenStrategy -match '(?i)(ambiguity|repeated-failure|long-horizon|multi-module|cross-module|deep)'
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
        [void]$reasons.Add("format_sensitive_validator_visible")
    } elseif ($fileScope -eq "small" -and $hasValidator -and $maxSpawn -eq 0) {
        $recommendedMode = "thin"
        $confidence = "high"
        [void]$reasons.Add("small_scope_validator_visible_no_spawn_expected")
    } elseif ($level -eq "L3" -or $deepSignal) {
        $recommendedMode = "deep"
        [void]$reasons.Add("deep_or_ambiguous_task_shape")
    } elseif ($maxSpawn -gt 0) {
        $recommendedMode = "subagent_assisted"
        [void]$reasons.Add("independent_subagent_budget_present")
    } elseif ($fileScope -eq "large") {
        $recommendedMode = "default_compact"
        [void]$reasons.Add("large_scope_without_deep_signal")
    } else {
        [void]$reasons.Add("fallback_default_compact")
    }
    if ($largeOutput) { [void]$reasons.Add("large_output_ref_policy_required") }
    $escalationRules = @(
        "validator_failure_seen",
        "missing_expected_artifact",
        "ambiguity_increased",
        "cross_module_dependency_seen"
    )
    [pscustomobject]@{
        schema_version = "TaskShapeRouterV1"
        status = "report_only"
        scenario_id = [string]$Manifest.Id
        recommended_mode = $recommendedMode
        confidence = $confidence
        reason = (@($reasons) -join ";")
        trigger_reasons = @($reasons)
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
            node_budget = if ($recommendedMode -eq "deep") { [Math]::Max($maxNodes, 16) } else { $maxNodes }
            state_commit_budget = if ($recommendedMode -eq "thin") { 4 } elseif ($recommendedMode -eq "deep") { 12 } else { 8 }
            large_output_policy = if ($largeOutput) { "ref-only" } else { "standard" }
            must_read_validator_first = [bool]($recommendedMode -eq "verification_first")
            escalation_allowed = [bool]($recommendedMode -in @("thin", "verification_first", "default_compact", "subagent_assisted"))
        }
        escalation_rules = @($escalationRules)
        stay_thin_policy = [pscustomobject]@{
            enabled = [bool]($recommendedMode -eq "thin")
            condition = "clear_patch_path_and_validator_visible"
            disallow_default_subagent_spawn = [bool]($recommendedMode -eq "thin")
        }
    }
}

function New-TaskspaceRoutingPrompt {
    param([Parameter(Mandatory = $true)]$RoutingDecision)
    $null = $RoutingDecision
    ""
}
