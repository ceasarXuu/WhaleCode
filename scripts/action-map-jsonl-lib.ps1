function New-JsonLineReadStats([string]$PathValue) {
    return [ordered]@{
        path = $PathValue
        exists = (Test-Path -LiteralPath $PathValue)
        totalLines = 0
        parsedLines = 0
        skippedBlankLines = 0
        parseErrorCount = 0
        parseErrors = New-Object System.Collections.Generic.List[object]
    }
}

function Read-JsonLines {
    param(
        [string]$PathValue,
        [object]$Stats = $null
    )

    $items = New-Object System.Collections.Generic.List[object]
    if (-not (Test-Path -LiteralPath $PathValue)) {
        return $items
    }

    $lineNumber = 0
    foreach ($line in Get-Content -LiteralPath $PathValue -Encoding UTF8) {
        $lineNumber++
        if ($Stats) { $Stats.totalLines = [int]$Stats.totalLines + 1 }
        if ([string]::IsNullOrWhiteSpace($line)) {
            if ($Stats) { $Stats.skippedBlankLines = [int]$Stats.skippedBlankLines + 1 }
            continue
        }
        try {
            $items.Add(($line | ConvertFrom-Json))
            if ($Stats) { $Stats.parsedLines = [int]$Stats.parsedLines + 1 }
        }
        catch {
            if ($Stats) {
                $Stats.parseErrorCount = [int]$Stats.parseErrorCount + 1
                $Stats.parseErrors.Add([ordered]@{
                    line = $lineNumber
                    message = $_.Exception.Message
                })
            }
        }
    }
    return $items
}
