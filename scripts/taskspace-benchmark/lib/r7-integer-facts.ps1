function ConvertTo-R7NonnegativeInt64Fact {
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

function Get-R7RequiredNonnegativeInt64Fact {
    param($Object, [string]$Name, [string]$Context)
    if ($null -eq $Object -or
        -not ($Object.PSObject.Properties.Name -contains $Name)) {
        throw "$Context is missing required nonnegative Int64 fact $Name"
    }
    $value = ConvertTo-R7NonnegativeInt64Fact $Object.$Name
    if ($null -eq $value) {
        throw "$Context has invalid nonnegative Int64 fact $Name"
    }
    $value
}

function Get-R7OptionalNonnegativeInt64Fact {
    param($Object, [string]$Name, [string]$Context)
    if ($null -eq $Object -or
        -not ($Object.PSObject.Properties.Name -contains $Name) -or
        $null -eq $Object.$Name) {
        return $null
    }
    $value = ConvertTo-R7NonnegativeInt64Fact $Object.$Name
    if ($null -eq $value) {
        throw "$Context has invalid optional nonnegative Int64 fact $Name"
    }
    $value
}

function Get-R7ExactPropertyInt64Sum {
    param([object[]]$Rows, [string]$Name, [string]$Context)
    [bigint]$sum = 0
    foreach ($row in $Rows) {
        $sum += [bigint](Get-R7RequiredNonnegativeInt64Fact $row $Name $Context)
    }
    if ($sum -gt [int64]::MaxValue) {
        throw "$Context exact sum exceeds Int64 for $Name"
    }
    [int64]$sum
}
