function Get-PerformanceNonnegativeInt64 {
    param($Value)
    if ($null -eq $Value) { return $null }
    if ($Value -isnot [byte] -and $Value -isnot [uint16] -and
        $Value -isnot [uint32] -and $Value -isnot [uint64] -and
        $Value -isnot [sbyte] -and $Value -isnot [int16] -and
        $Value -isnot [int32] -and $Value -isnot [int64]) {
        return $null
    }
    [bigint]$number = $Value
    if ($number -lt 0 -or $number -gt [int64]::MaxValue) { return $null }
    [int64]$number
}

function Get-PerformanceExactInt64Sum {
    param([object[]]$Values, [string]$FieldName = "value")
    [bigint]$sum = 0
    foreach ($value in $Values) {
        $exact = Get-PerformanceNonnegativeInt64 $value
        if ($null -eq $exact) {
            throw "Performance exact sum contains an invalid $FieldName"
        }
        $sum += [bigint]$exact
    }
    if ($sum -gt [int64]::MaxValue) {
        throw "Performance exact sum exceeds int64 for $FieldName"
    }
    [int64]$sum
}

function Get-PerformanceTokenIdentity {
    param(
        $Metrics,
        $CacheSummary,
        $ProviderRequestCount,
        [bool]$Skipped
    )
    $raw = [ordered]@{
        input_tokens = Get-PerformanceProperty $Metrics "input_tokens"
        cached_input_tokens = Get-PerformanceProperty $Metrics "cached_input_tokens"
        uncached_input_tokens = Get-PerformanceProperty $Metrics "uncached_input_tokens"
        output_tokens = Get-PerformanceProperty $Metrics "output_tokens"
        request_2_plus_count = Get-PerformanceProperty $CacheSummary "request_2_plus_count"
        request_2_plus_cached_input_tokens = Get-PerformanceProperty `
            $CacheSummary "request_2_plus_cached_input_tokens"
        request_2_plus_uncached_input_tokens = Get-PerformanceProperty `
            $CacheSummary "request_2_plus_uncached_input_tokens"
    }
    $values = [ordered]@{}
    $invalid = [Collections.Generic.List[string]]::new()
    foreach ($field in $raw.Keys) {
        $values[$field] = Get-PerformanceNonnegativeInt64 $raw[$field]
        if (-not $Skipped -and $null -eq $values[$field]) {
            $invalid.Add($field)
        }
    }
    $providerRequests = Get-PerformanceNonnegativeInt64 $ProviderRequestCount
    if (-not $Skipped -and $null -eq $providerRequests) {
        $invalid.Add("provider_request_count")
    }
    if (-not $Skipped -and $null -ne $values.input_tokens -and
        $values.input_tokens -le 0) {
        $invalid.Add("input_tokens_nonpositive")
    }
    if (-not $Skipped -and
        $null -ne $values.input_tokens -and
        $null -ne $values.cached_input_tokens -and
        $null -ne $values.uncached_input_tokens) {
        if ($values.cached_input_tokens -gt $values.input_tokens) {
            $invalid.Add("cached_input_tokens_exceed_input")
        } elseif ($values.uncached_input_tokens -ne
            ($values.input_tokens - $values.cached_input_tokens)) {
            $invalid.Add("uncached_input_tokens_mismatch")
        }
    }
    if (-not $Skipped -and
        $null -ne $providerRequests -and
        $null -ne $values.request_2_plus_count -and
        $values.request_2_plus_count -ne [Math]::Max(0, $providerRequests - 1)) {
        $invalid.Add("request_2_plus_count_mismatch")
    }
    if (-not $Skipped -and
        $null -ne $values.request_2_plus_cached_input_tokens -and
        $null -ne $values.cached_input_tokens -and
        $values.request_2_plus_cached_input_tokens -gt $values.cached_input_tokens) {
        $invalid.Add("request_2_plus_cached_input_tokens_exceed_total")
    }
    if (-not $Skipped -and
        $null -ne $values.request_2_plus_uncached_input_tokens -and
        $null -ne $values.uncached_input_tokens -and
        $values.request_2_plus_uncached_input_tokens -gt $values.uncached_input_tokens) {
        $invalid.Add("request_2_plus_uncached_input_tokens_exceed_total")
    }
    [pscustomobject]@{
        valid = $Skipped -or $invalid.Count -eq 0
        invalid_fields = @($invalid | Sort-Object -Unique)
        provider_request_count = $providerRequests
        input_tokens = $values.input_tokens
        cached_input_tokens = $values.cached_input_tokens
        uncached_input_tokens = $values.uncached_input_tokens
        output_tokens = $values.output_tokens
        request_2_plus_count = $values.request_2_plus_count
        request_2_plus_cached_input_tokens =
            $values.request_2_plus_cached_input_tokens
        request_2_plus_uncached_input_tokens =
            $values.request_2_plus_uncached_input_tokens
    }
}
