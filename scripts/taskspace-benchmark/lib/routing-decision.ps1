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
        [void]$reasons.Add("format_sensitive_validator_visible")
    } elseif ($fileScope -eq "small" -and $hasValidator -and $maxSpawn -eq 0) {
        $recommendedMode = "thin"
        $confidence = "high"
        [void]$reasons.Add("small_scope_validator_visible_no_spawn_expected")
    } elseif ($maxSpawn -gt 0 -or $fileScope -eq "large") {
        $recommendedMode = "default_compact"
        [void]$reasons.Add("multi_step_or_spawn_budget_present")
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
            node_budget = $maxNodes
            state_commit_budget = if ($recommendedMode -eq "thin") { 4 } else { 8 }
            large_output_policy = if ($largeOutput) { "ref-only" } else { "standard" }
            must_read_validator_first = [bool]($recommendedMode -eq "verification_first")
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
    $mode = [string]$RoutingDecision.recommended_mode
    $constraints = $RoutingDecision.initial_constraints
    $nodeBudget = if ($constraints -and $constraints.PSObject.Properties.Name -contains "node_budget") { [int]$constraints.node_budget } else { 8 }
    $stateCommitBudget = if ($constraints -and $constraints.PSObject.Properties.Name -contains "state_commit_budget") { [int]$constraints.state_commit_budget } else { 8 }
    $lines = New-Object System.Collections.Generic.List[string]
    [void]$lines.Add("")
    [void]$lines.Add("TaskShapeRouterV1 active profile constraints:")
    [void]$lines.Add("- recommended_mode: $mode")
    [void]$lines.Add("- node_budget: $nodeBudget")
    [void]$lines.Add("- state_commit_budget: $stateCommitBudget")
    [void]$lines.Add("- Use the smallest evidence path that can satisfy the visible validator and hidden oracle.")
    if ($mode -eq "verification_first") {
        [void]$lines.Add("- Verification-first rule: read the validator/test contract before editing and identify the exact expected output format.")
        [void]$lines.Add("- Use at most three TaskSpace nodes unless validation fails: inspect contract, implement patch, validate.")
        [void]$lines.Add("- Batch obvious read-only inspection into one bounded command when practical: README, tests, source, and validator.")
        [void]$lines.Add("- After inspection identifies the expected format and target function, do not stop for a separate findings checkpoint; proceed to the implementation patch.")
        [void]$lines.Add("- Batch final validation into one bounded command when practical: unit tests and the visible validator.")
        [void]$lines.Add("- Do not run an extra direct CLI-output check after the visible validator passes. The visible validator is the final format check for this route.")
        [void]$lines.Add("- After validation passes, use one state_commit for result_validities, success_criteria, finished_nodes, decisions/adoptions that are ready, then answer directly.")
    } elseif ($mode -eq "thin") {
        [void]$lines.Add("- Thin rule: no subagents by default; stay on the main-agent path unless validation failure reveals a new independent track.")
        [void]$lines.Add("- Use at most three TaskSpace nodes for a clear single-file fix: inspect, implement, validate.")
        [void]$lines.Add("- After validation passes, avoid summary-only final_synthesis; answer directly.")
    } else {
        [void]$lines.Add("- Default compact rule: prefer state_commit checkpoints and avoid duplicate summary-only nodes.")
    }
    [void]$lines.Add("")
    ($lines.ToArray() -join [Environment]::NewLine)
}
