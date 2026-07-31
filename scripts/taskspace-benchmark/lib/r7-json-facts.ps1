function Assert-R7UniqueJsonProperties {
    param(
        [System.Text.Json.JsonElement]$Element,
        [string]$Path = "$"
    )
    if ($Element.ValueKind -eq [System.Text.Json.JsonValueKind]::Object) {
        $names = [Collections.Generic.HashSet[string]]::new(
            [StringComparer]::OrdinalIgnoreCase
        )
        foreach ($property in $Element.EnumerateObject()) {
            if (-not $names.Add([string]$property.Name)) {
                throw "Duplicate JSON property: $Path.$($property.Name)"
            }
            Assert-R7UniqueJsonProperties $property.Value "$Path.$($property.Name)"
        }
    } elseif ($Element.ValueKind -eq [System.Text.Json.JsonValueKind]::Array) {
        $index = 0
        foreach ($item in $Element.EnumerateArray()) {
            Assert-R7UniqueJsonProperties $item "$Path[$index]"
            $index++
        }
    }
}

function ConvertFrom-R7StrictJsonObject {
    param([Parameter(Mandatory = $true)][string]$Text)
    $document = [System.Text.Json.JsonDocument]::Parse($Text)
    try {
        Assert-R7UniqueJsonProperties $document.RootElement
        if ($document.RootElement.ValueKind -ne
            [System.Text.Json.JsonValueKind]::Object) {
            throw "JSON root must be an object"
        }
    } finally {
        $document.Dispose()
    }
    $payload = $Text | ConvertFrom-Json -Depth 100 -NoEnumerate
    if ($payload -is [System.Array] -or $payload -isnot [pscustomobject]) {
        throw "JSON root must be an object"
    }
    $payload
}

function Get-R7Sha256Hex {
    param([Parameter(Mandatory = $true)][string]$Text)
    $bytes = [Text.Encoding]::UTF8.GetBytes($Text)
    $hash = [Security.Cryptography.SHA256]::HashData($bytes)
    ([Convert]::ToHexString($hash)).ToLowerInvariant()
}
