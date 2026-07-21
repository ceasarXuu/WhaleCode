$script:R7ContinuousActionArms = @("standard", "sibling_baseline", "fla3_5_candidate")
$script:R7ContinuousActionSamples = @("carrier_validation_dev", "single_file_fast_fix", "subscription_billing_repair")

function Add-R7Gate {
    param([System.Collections.Generic.List[object]]$Gates, [string]$Code, [bool]$Passed, [string]$Detail)
    $Gates.Add([pscustomobject][ordered]@{code = $Code; passed = $Passed; detail = $Detail})
}

function Get-R7ArtifactObject {
    param([string]$BaseDir, $Ref, [string]$Role)
    $path = [string]$Ref.path
    if ([System.IO.Path]::IsPathRooted($path) -or $path.Contains("..")) { throw "R7_ARTIFACT_PATH_INVALID role=$Role" }
    $full = Join-Path $BaseDir $path
    if (-not (Test-Path -LiteralPath $full -PathType Leaf)) { throw "R7_ARTIFACT_MISSING role=$Role path=$path" }
    if ((Get-R7Sha256File $full) -cne [string]$Ref.sha256) { throw "R7_HASH_DRIFT role=$Role path=$path" }
    Read-R7StrictJson $full
}

function Get-R7ContractDigest {
    param($Contract)
    $copy = ($Contract | ConvertTo-Json -Depth 100) | ConvertFrom-Json -Depth 100
    $copy.psobject.Properties.Remove("contract_digest")
    Get-R7Sha256Text ((ConvertTo-R7CanonicalJson $copy) + "`n")
}

function Assert-R7RunSetIdentity {
    param($RunSet, $Contract, [string]$ContractPath, [System.Collections.Generic.List[object]]$Gates)
    Add-R7Gate $Gates "R7_CONTRACT_FILE_HASH" ((Get-R7Sha256File $ContractPath) -ceq [string]$RunSet.evaluation_contract.sha256) "contract file hash"
    Add-R7Gate $Gates "R7_CONTRACT_DIGEST" ((Get-R7ContractDigest $Contract) -ceq [string]$Contract.contract_digest -and [string]$RunSet.evaluation_contract.contract_digest -ceq [string]$Contract.contract_digest) "contract digest"
    $identityFields = @($Contract.identity.directory_identity_fields)
    $contractIdentity = [ordered]@{}
    foreach ($sample in $script:R7ContinuousActionSamples) {
        $row = [ordered]@{}
        foreach ($field in $identityFields) { $row[$field] = $Contract.samples.$sample.$field }
        $contractIdentity[$sample] = $row
    }
    Add-R7Gate $Gates "R7_CONTRACT_DIRECTORY_IDENTITY" ((Get-R7JsonValueHash ([pscustomobject]$contractIdentity)) -ceq [string]$Contract.identity.contract_directory_identity_sha256) "sealed fixture identity"
    $fixtureBySample = @{}
    foreach ($fixture in @($RunSet.fixtures)) {
        $sample = [string]$fixture.sample
        if ($fixtureBySample.ContainsKey($sample)) { Add-R7Gate $Gates "R7_FIXTURE_DUPLICATE" $false $sample }
        $fixtureBySample[$sample] = $fixture
    }
    foreach ($sample in $script:R7ContinuousActionSamples) {
        if (-not $fixtureBySample.ContainsKey($sample)) { Add-R7Gate $Gates "R7_FIXTURE_MISSING" $false $sample; continue }
        $actual = $fixtureBySample[$sample]
        $expected = $Contract.samples.$sample
        $actualRow = [ordered]@{}
        $expectedRow = [ordered]@{}
        foreach ($field in $identityFields) { $actualRow[$field] = $actual.$field; $expectedRow[$field] = $expected.$field }
        Add-R7Gate $Gates "R7_FIXTURE_IDENTITY_$($sample.ToUpperInvariant())" ((ConvertTo-R7CanonicalJson ([pscustomobject]$actualRow)) -ceq (ConvertTo-R7CanonicalJson ([pscustomobject]$expectedRow))) $sample
    }
}

function Assert-R7ExpectedRunSet {
    param($RunSet, $Contract, [System.Collections.Generic.List[object]]$Gates)
    $expected = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    foreach ($sample in $script:R7ContinuousActionSamples) {
        for ($repeat = 1; $repeat -le [int]$Contract.samples.$sample.repeats; $repeat++) {
            foreach ($arm in $script:R7ContinuousActionArms) { [void]$expected.Add("$arm|$sample|$repeat") }
        }
    }
    $seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
    foreach ($run in @($RunSet.runs)) {
        $key = "$($run.arm)|$($run.sample)|$($run.repeat)"
        if (-not $expected.Contains($key)) { Add-R7Gate $Gates "R7_EXTRA_ARM_SAMPLE_REPEAT" $false $key }
        if (-not $seen.Add($key)) { Add-R7Gate $Gates "R7_DUPLICATE_ARM_SAMPLE_REPEAT" $false $key }
        if ([bool]$run.held_out) { Add-R7Gate $Gates "R7_HELD_OUT_REJECTED" $false $key }
    }
    foreach ($key in $expected) {
        if (-not $seen.Contains($key)) { Add-R7Gate $Gates "R7_MISSING_ARM_SAMPLE_REPEAT" $false $key }
    }
}

function Get-R7RunFacts {
    param([string]$Base, $Run)
    $a = $Run.artifacts
    $metrics = Get-R7ArtifactObject $Base $a.metrics "metrics"
    $requests = Get-R7ArtifactObject $Base $a.requests "requests"
    $tools = Get-R7ArtifactObject $Base $a.tools "tools"
    $map = Get-R7ArtifactObject $Base $a.map "map"
    $verdict = Get-R7ArtifactObject $Base $a.verdict "verdict"
    $cache = Get-R7ArtifactObject $Base $a.cache "cache"
    $requestEvents = @($requests.events)
    if ($requestEvents.Count -eq 0) { throw "R7_REQUEST_EVENTS_EMPTY" }
    $inputTokens = [double](($requestEvents | Measure-Object input_tokens -Sum).Sum)
    $outputTokens = [double](($requestEvents | Measure-Object output_tokens -Sum).Sum)
    if ($inputTokens -le 0) { throw "R7_INPUT_TOKENS_UNAVAILABLE" }
    $cacheEpochs = @($cache.epochs)
    if ($cacheEpochs.Count -eq 0 -or @($cacheEpochs | Where-Object { [string]$_.status -ne "available" }).Count -gt 0) { throw "R7_CACHE_UNAVAILABLE" }
    $cacheInput = [double](($cacheEpochs | Measure-Object input_tokens -Sum).Sum)
    $cached = [double](($cacheEpochs | Measure-Object cached_input_tokens -Sum).Sum)
    if ($cacheInput -ne $inputTokens -or $cached -lt 0 -or $cached -gt $cacheInput) { throw "R7_CACHE_EPOCH_DRIFT" }
    $mapEvents = @($map.events)
    $accepted = @($mapEvents | Where-Object { [string]$_.event_kind -eq "transition_accepted" -and -not [bool]$_.terminal })
    $carriers = @($accepted | Where-Object { -not [string]::IsNullOrWhiteSpace([string]$_.reserved_tool_call_id) })
    $standalone = @($accepted | Where-Object { [string]::IsNullOrWhiteSpace([string]$_.reserved_tool_call_id) })
    $toolEvents = @($tools.events)
    $started = @($toolEvents | Where-Object event_kind -eq "handler_handoff_started" | ForEach-Object carrier_id | Sort-Object -Unique)
    $accounted = @($toolEvents | Where-Object event_kind -eq "carrier_outcome" | ForEach-Object carrier_id | Sort-Object -Unique)
    $patch = @($toolEvents | Where-Object event_kind -eq "patch_input")
    $outputs = @($toolEvents | Where-Object event_kind -eq "carrier_output")
    $errors = @($verdict.error_codes)
    $wall = [double]$metrics.finished_at_ms - [double]$metrics.started_at_ms
    if ($wall -lt 0) { throw "R7_WALL_TIME_INVALID" }
    [pscustomobject][ordered]@{
        run_id = [string]$Run.run_id; arm = [string]$Run.arm; sample = [string]$Run.sample; repeat = [int]$Run.repeat
        correct = [bool]$verdict.correct; accepted = $accepted.Count; carrier = $carriers.Count
        carrier_started = $started.Count; carrier_accounted = $accounted.Count; standalone = $standalone.Count
        h003 = @($errors | Where-Object { $_ -eq "TASKSPACE_REQUIRED_SIBLING_MISSING" }).Count
        patch_inputs = $patch.Count; patch_exact = @($patch | Where-Object byte_exact).Count
        typed_outputs = $outputs.Count; typed_exact = @($outputs | Where-Object { [bool]$_.schema_exact -and [bool]$_.payload_exact }).Count
        request_count = $requestEvents.Count; input_tokens = $inputTokens; output_tokens = $outputTokens
        cache_hit_rate = $cached / $cacheInput; wall_time_ms = $wall
        provider_time_ms = [double](($requestEvents | Measure-Object duration_ms -Sum).Sum)
        tool_time_ms = [double](($toolEvents | Where-Object event_kind -eq "tool_finished" | Measure-Object duration_ms -Sum).Sum)
    }
}

function Get-R7Rate {
    param([double]$Numerator, [double]$Denominator)
    if ($Denominator -le 0) { return $null }
    [math]::Round($Numerator / $Denominator, 8)
}

function Get-R7ArmMetrics {
    param([object[]]$Facts)
    $count = $Facts.Count
    $sum = @{}
    foreach ($name in @("accepted", "carrier", "carrier_started", "carrier_accounted", "standalone", "h003", "patch_inputs", "patch_exact", "typed_outputs", "typed_exact", "request_count", "input_tokens", "output_tokens", "wall_time_ms", "provider_time_ms", "tool_time_ms")) {
        $sum[$name] = [double](($Facts | Measure-Object $name -Sum).Sum)
    }
    [pscustomobject][ordered]@{
        run_count = $count
        correctness_rate = Get-R7Rate (@($Facts | Where-Object correct).Count) $count
        transition_carrier_rate = Get-R7Rate $sum.carrier $sum.accepted
        carrier_execution_started_rate = Get-R7Rate $sum.carrier_started $sum.carrier
        carrier_conservation_rate = Get-R7Rate $sum.carrier_accounted $sum.carrier
        standalone_nonterminal_count = [int]$sum.standalone; h003_count = [int]$sum.h003
        patch_input_exact_rate = Get-R7Rate $sum.patch_exact $sum.patch_inputs
        typed_output_exact_rate = Get-R7Rate $sum.typed_exact $sum.typed_outputs
        request_count = [int]$sum.request_count; input_tokens = [int]$sum.input_tokens; output_tokens = [int]$sum.output_tokens
        cache_hit_rate = if ($count -eq 0) { $null } else { [math]::Round((($Facts | Measure-Object cache_hit_rate -Average).Average), 8) }
        wall_time_ms = [int]$sum.wall_time_ms; provider_time_ms = [int]$sum.provider_time_ms; tool_time_ms = [int]$sum.tool_time_ms
    }
}

function Get-R7PairedFacts {
    param([object[]]$Facts, $Contract, [System.Collections.Generic.List[object]]$Gates)
    $pairs = [System.Collections.Generic.List[object]]::new()
    foreach ($sample in $script:R7ContinuousActionSamples) {
        for ($repeat = 1; $repeat -le [int]$Contract.samples.$sample.repeats; $repeat++) {
            $candidate = @($Facts | Where-Object { $_.arm -eq "fla3_5_candidate" -and $_.sample -eq $sample -and $_.repeat -eq $repeat })
            $baseline = @($Facts | Where-Object { $_.arm -eq "sibling_baseline" -and $_.sample -eq $sample -and $_.repeat -eq $repeat })
            if ($candidate.Count -ne 1 -or $baseline.Count -ne 1) { continue }
            if ($baseline[0].request_count -le 0 -or $baseline[0].input_tokens -le 0 -or $baseline[0].output_tokens -le 0 -or $baseline[0].wall_time_ms -le 0) {
                Add-R7Gate $Gates "R7_PAIRED_BASELINE_ZERO" $false "$sample/$repeat"; continue
            }
            $pairs.Add([pscustomobject][ordered]@{
                pair_id = "$sample/$repeat"; sample = $sample; repeat = $repeat
                request_amplification = $candidate[0].request_count / $baseline[0].request_count
                input_token_amplification = $candidate[0].input_tokens / $baseline[0].input_tokens
                output_token_amplification = $candidate[0].output_tokens / $baseline[0].output_tokens
                wall_time_amplification = $candidate[0].wall_time_ms / $baseline[0].wall_time_ms
                cache_hit_rate_delta = $candidate[0].cache_hit_rate - $baseline[0].cache_hit_rate
            })
        }
    }
    $pairs.ToArray()
}

function Invoke-R7PairedBootstrap {
    param([object[]]$Pairs, $Contract)
    $resamples = [int]$Contract.bootstrap.resamples
    $random = [System.Random]::new([int]$Contract.seed + [int]$Contract.bootstrap.seed_offset)
    $definitions = @(
        [pscustomobject]@{name = "request_amplification"; threshold = [double]$Contract.thresholds.request_amplification_max; direction = "upper"},
        [pscustomobject]@{name = "input_token_amplification"; threshold = [double]$Contract.thresholds.input_token_amplification_max; direction = "upper"},
        [pscustomobject]@{name = "output_token_amplification"; threshold = [double]$Contract.thresholds.output_token_amplification_max; direction = "upper"},
        [pscustomobject]@{name = "wall_time_amplification"; threshold = [double]$Contract.thresholds.wall_time_amplification_max; direction = "upper"},
        [pscustomobject]@{name = "cache_hit_rate_delta"; threshold = [double]$Contract.thresholds.cache_hit_rate_delta_min; direction = "lower"}
    )
    $results = [System.Collections.Generic.List[object]]::new()
    foreach ($definition in $definitions) {
        $sources = @{}
        $estimateTotal = 0.0
        foreach ($sample in $script:R7ContinuousActionSamples) {
            [double[]]$values = @($Pairs | Where-Object sample -eq $sample | ForEach-Object { [double]$_."$($definition.name)" })
            if ($values.Count -eq 0) { throw "R7_BOOTSTRAP_SAMPLE_EMPTY sample=$sample" }
            $sources[$sample] = $values
            $sampleTotal = 0.0
            foreach ($value in $values) { $sampleTotal += $value }
            $estimateTotal += $sampleTotal / $values.Count
        }
        $distribution = [double[]]::new($resamples)
        for ($iteration = 0; $iteration -lt $resamples; $iteration++) {
            $strataTotal = 0.0
            foreach ($sample in $script:R7ContinuousActionSamples) {
                [double[]]$source = $sources[$sample]
                $sampleTotal = 0.0
                for ($pick = 0; $pick -lt $source.Count; $pick++) { $sampleTotal += $source[$random.Next(0, $source.Count)] }
                $strataTotal += $sampleTotal / $source.Count
            }
            $distribution[$iteration] = $strataTotal / $script:R7ContinuousActionSamples.Count
        }
        [System.Array]::Sort($distribution)
        $low = $distribution[[math]::Floor(0.025 * ($resamples - 1))]
        $high = $distribution[[math]::Ceiling(0.975 * ($resamples - 1))]
        $bad = 0
        foreach ($value in $distribution) {
            if (($definition.direction -eq "upper" -and $value -gt $definition.threshold) -or ($definition.direction -eq "lower" -and $value -lt $definition.threshold)) { $bad++ }
        }
        $passed = if ($definition.direction -eq "upper") { $high -le $definition.threshold } else { $low -ge $definition.threshold }
        $results.Add([pscustomobject][ordered]@{
            metric = $definition.name; estimate = $estimateTotal / $script:R7ContinuousActionSamples.Count
            ci_low = $low; ci_high = $high; threshold = $definition.threshold; direction = $definition.direction
            raw_p = ($bad + 1.0) / ($resamples + 1.0); decision = if ($passed) { "pass" } else { "fail" }
        })
    }
    [pscustomobject][ordered]@{method = "paired_percentile_sample_stratified"; confidence = 0.95; resamples = $resamples; seed = [int]$Contract.seed + [int]$Contract.bootstrap.seed_offset; metrics = $results.ToArray()}
}

function Get-R7HolmResult {
    param([object[]]$Metrics, $Contract)
    $ordered = @($Metrics | Sort-Object raw_p, metric)
    $previous = 0.0
    $rows = [System.Collections.Generic.List[object]]::new()
    for ($index = 0; $index -lt $ordered.Count; $index++) {
        $adjusted = [math]::Min(1.0, [math]::Max($previous, [double]$ordered[$index].raw_p * ($ordered.Count - $index)))
        $previous = $adjusted
        $rows.Add([pscustomobject][ordered]@{metric = $ordered[$index].metric; raw_p = $ordered[$index].raw_p; adjusted_p = $adjusted; reject = $adjusted -le [double]$Contract.correction.familywise_alpha})
    }
    [pscustomobject][ordered]@{method = "holm"; familywise_alpha = [double]$Contract.correction.familywise_alpha; metrics = $rows.ToArray()}
}

function Invoke-R7ContinuousActionEvaluation {
    param([string]$RunSetPath, [string]$ContractPath, [string]$RunArtifactRoot)
    $gates = [System.Collections.Generic.List[object]]::new()
    $contract = Read-R7StrictJson $ContractPath (Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/continuous-action-evaluation-v1.schema.json")
    $runSet = Read-R7StrictJson $RunSetPath (Join-Path $script:R7RepoRoot "benchmarks/taskspace/r7/continuous-action-raw-run-set-v1.schema.json")
    Assert-R7RunSetIdentity $runSet $contract $ContractPath $gates
    Assert-R7ExpectedRunSet $runSet $contract $gates
    if (@($gates | Where-Object { -not $_.passed }).Count -gt 0) {
        $emptyMetrics = [pscustomobject][ordered]@{}
        foreach ($arm in $script:R7ContinuousActionArms) { $emptyMetrics | Add-Member -NotePropertyName $arm -NotePropertyValue (Get-R7ArmMetrics @()) }
        $emptyBootstrap = [pscustomobject]@{method = "paired_percentile_sample_stratified"; confidence = 0.95; resamples = 0; seed = [int]$contract.seed + 1; metrics = @()}
        return [pscustomobject][ordered]@{
            schema_version = 1; artifact_role = "continuous_action_evaluation_result"; evaluation_id = "r7-fla3-5-continuous-action-v1"; decision = "fail"
            hard_gates = $gates.ToArray(); codes = @($gates | Where-Object { -not $_.passed } | ForEach-Object code | Sort-Object -Unique)
            run_count = @($runSet.runs).Count; pair_count = 0; metrics_by_arm = $emptyMetrics; paired_facts = @()
            bootstrap = $emptyBootstrap; holm = Get-R7HolmResult @() $contract
        }
    }
    $base = if ([string]::IsNullOrWhiteSpace($RunArtifactRoot)) { Split-Path -Parent $RunSetPath } else { $RunArtifactRoot }
    $facts = [System.Collections.Generic.List[object]]::new()
    foreach ($run in @($runSet.runs | Sort-Object arm, sample, repeat)) {
        try { $facts.Add((Get-R7RunFacts $base $run)) } catch { Add-R7Gate $gates ([string]$_.Exception.Message.Split(" ")[0]) $false ([string]$run.run_id); break }
    }
    $metrics = [pscustomobject][ordered]@{}
    foreach ($arm in $script:R7ContinuousActionArms) { $metrics | Add-Member -NotePropertyName $arm -NotePropertyValue (Get-R7ArmMetrics @($facts | Where-Object arm -eq $arm)) }
    foreach ($arm in $script:R7ContinuousActionArms) { Add-R7Gate $gates "R7_CORRECTNESS_$($arm.ToUpperInvariant())" ($metrics.$arm.correctness_rate -eq 1) "rate=$($metrics.$arm.correctness_rate)" }
    $candidate = $metrics.fla3_5_candidate
    Add-R7Gate $gates "R7_CANDIDATE_CARRIER" ($candidate.transition_carrier_rate -eq 1 -and $candidate.carrier_execution_started_rate -eq 1 -and $candidate.carrier_conservation_rate -eq 1) "carrier/start/conservation"
    Add-R7Gate $gates "R7_CANDIDATE_STANDALONE_H003" ($candidate.standalone_nonterminal_count -eq 0 -and $candidate.h003_count -eq 0) "standalone=$($candidate.standalone_nonterminal_count) h003=$($candidate.h003_count)"
    Add-R7Gate $gates "R7_CANDIDATE_EXACTNESS" ($candidate.patch_input_exact_rate -eq 1 -and $candidate.typed_output_exact_rate -eq 1) "patch=$($candidate.patch_input_exact_rate) typed=$($candidate.typed_output_exact_rate)"
    $standardWire = @($runSet.runs | Where-Object arm -eq "standard" | Where-Object { [string]$_.wire_sha256 -cne [string]$runSet.identity.standard_wire_sha256 })
    Add-R7Gate $gates "R7_STANDARD_WIRE_IDENTITY" ($standardWire.Count -eq 0) "drift_runs=$($standardWire.Count)"
    $pairs = @(Get-R7PairedFacts $facts.ToArray() $contract $gates)
    $preStatFailure = @($gates | Where-Object { -not $_.passed }).Count -gt 0
    $bootstrap = if ($pairs.Count -gt 0 -and -not $preStatFailure) { Invoke-R7PairedBootstrap $pairs $contract } else { [pscustomobject]@{method = "paired_percentile_sample_stratified"; confidence = 0.95; resamples = 0; seed = [int]$contract.seed + 1; metrics = @()} }
    foreach ($metric in @($bootstrap.metrics)) { Add-R7Gate $gates "R7_NONINFERIOR_$($metric.metric.ToUpperInvariant())" ([string]$metric.decision -eq "pass") "ci=[$($metric.ci_low),$($metric.ci_high)] threshold=$($metric.threshold)" }
    $holm = Get-R7HolmResult @($bootstrap.metrics) $contract
    $failed = @($gates | Where-Object { -not $_.passed })
    [pscustomobject][ordered]@{
        schema_version = 1; artifact_role = "continuous_action_evaluation_result"; evaluation_id = "r7-fla3-5-continuous-action-v1"
        decision = if ($failed.Count -eq 0) { "pass" } else { "fail" }
        hard_gates = $gates.ToArray(); codes = @($failed | ForEach-Object code | Sort-Object -Unique)
        run_count = @($runSet.runs).Count; pair_count = $pairs.Count; metrics_by_arm = $metrics; paired_facts = $pairs
        bootstrap = $bootstrap; holm = $holm
    }
}
