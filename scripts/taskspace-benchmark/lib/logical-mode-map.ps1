function Read-TaskspaceLogicalModeMap {
    param([Parameter(Mandatory = $true)][string]$Path)

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return [pscustomobject]@{
            status = "missing"
            valid = $false
            map = $null
            invalid_fields = @("file")
        }
    }

    $invalid = [System.Collections.Generic.List[string]]::new()
    $document = $null
    try {
        $raw = [System.IO.File]::ReadAllText($Path, [System.Text.Encoding]::UTF8)
        $document = [System.Text.Json.JsonDocument]::Parse($raw)
        $root = $document.RootElement
        if ($root.ValueKind -ne [System.Text.Json.JsonValueKind]::Object) {
            $invalid.Add("root")
        } else {
            $properties = @{}
            foreach ($property in $root.EnumerateObject()) {
                if ($properties.ContainsKey($property.Name)) {
                    $invalid.Add("duplicate_property:$($property.Name)")
                } else {
                    $properties[$property.Name] = $property.Value.Clone()
                }
            }

            foreach ($name in @("repeat", "left", "right")) {
                if (-not $properties.ContainsKey($name)) { $invalid.Add($name) }
            }
            if ($properties.ContainsKey("repeat")) {
                [int64]$repeat = 0
                if ($properties.repeat.ValueKind -ne [System.Text.Json.JsonValueKind]::Number -or
                    -not $properties.repeat.TryGetInt64([ref]$repeat) -or $repeat -le 0) {
                    $invalid.Add("repeat")
                }
            }
            foreach ($side in @("left", "right")) {
                if (-not $properties.ContainsKey($side)) { continue }
                if ($properties[$side].ValueKind -ne [System.Text.Json.JsonValueKind]::String -or
                    $properties[$side].GetString() -notin @("standard", "taskspace")) {
                    $invalid.Add($side)
                }
            }
            if ($properties.ContainsKey("left") -and $properties.ContainsKey("right") -and
                $properties.left.ValueKind -eq [System.Text.Json.JsonValueKind]::String -and
                $properties.right.ValueKind -eq [System.Text.Json.JsonValueKind]::String -and
                $properties.left.GetString() -eq $properties.right.GetString()) {
                $invalid.Add("side_identity")
            }
        }
    } catch {
        $invalid.Add("json")
    } finally {
        if ($null -ne $document) { $document.Dispose() }
    }

    $fields = @($invalid.ToArray() | Sort-Object -Unique)
    if ($fields.Count -gt 0) {
        return [pscustomobject]@{
            status = "invalid"
            valid = $false
            map = $null
            invalid_fields = $fields
        }
    }

    [pscustomobject]@{
        status = "measured"
        valid = $true
        map = [pscustomobject]@{
            repeat = [int64]$properties.repeat.GetInt64()
            left = [string]$properties.left.GetString()
            right = [string]$properties.right.GetString()
        }
        invalid_fields = @()
    }
}
