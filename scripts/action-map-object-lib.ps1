function Get-ObjectPropertyNames($Value) {
    if ($null -eq $Value) {
        return @()
    }
    return @($Value.PSObject.Properties.Name)
}

function Get-ObjectField {
    param(
        [object]$Value,
        [string]$Name
    )

    if ($null -eq $Value) {
        return $null
    }
    if ($Value -is [System.Collections.IDictionary]) {
        return $Value[$Name]
    }
    $property = $Value.PSObject.Properties[$Name]
    if ($null -eq $property) {
        return $null
    }
    return $property.Value
}

function Get-ObjectArray($Value) {
    if ($null -eq $Value) {
        return @()
    }
    if ($Value -is [System.Array]) {
        return @($Value)
    }
    if ($Value -is [System.Collections.IEnumerable] -and -not ($Value -is [string])) {
        $items = New-Object System.Collections.Generic.List[object]
        foreach ($item in $Value) {
            $items.Add($item)
        }
        return @($items.ToArray())
    }
    return @($Value)
}
