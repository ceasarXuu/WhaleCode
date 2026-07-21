function ConvertTo-R7StrictCanonicalToken {
    param([Newtonsoft.Json.Linq.JToken]$Token)
    if ($Token -is [Newtonsoft.Json.Linq.JObject]) {
        $result = [Newtonsoft.Json.Linq.JObject]::new()
        $names = [string[]]@($Token.Properties() | ForEach-Object Name)
        [System.Array]::Sort($names, [System.StringComparer]::Ordinal)
        foreach ($name in $names) {
            $result.Add($name, (ConvertTo-R7StrictCanonicalToken $Token.Property($name).Value))
        }
        Write-Output -NoEnumerate $result
        return
    }
    if ($Token -is [Newtonsoft.Json.Linq.JArray]) {
        $result = [Newtonsoft.Json.Linq.JArray]::new()
        foreach ($item in $Token.Children()) { $result.Add((ConvertTo-R7StrictCanonicalToken $item)) }
        Write-Output -NoEnumerate $result
        return
    }
    Write-Output -NoEnumerate $Token.DeepClone()
}

function Assert-R7StrictUnicodeScalarString {
    param([string]$Value, [string]$Location)
    for ($index = 0; $index -lt $Value.Length; $index++) {
        $code = [int][char]$Value[$index]
        if ($code -ge 0xD800 -and $code -le 0xDBFF) {
            if ($index + 1 -ge $Value.Length) { throw "R7_STRICT_JSON_ISOLATED_SURROGATE location=$Location" }
            $next = [int][char]$Value[$index + 1]
            if ($next -lt 0xDC00 -or $next -gt 0xDFFF) { throw "R7_STRICT_JSON_ISOLATED_SURROGATE location=$Location" }
            $index++
        } elseif ($code -ge 0xDC00 -and $code -le 0xDFFF) {
            throw "R7_STRICT_JSON_ISOLATED_SURROGATE location=$Location"
        }
    }
}

function Assert-R7StrictEscapedSurrogates {
    param([string]$Text)
    $inString = $false
    for ($index = 0; $index -lt $Text.Length; $index++) {
        $char = $Text[$index]
        if (-not $inString) {
            if ($char -eq '"') { $inString = $true }
            continue
        }
        if ($char -eq '"') { $inString = $false; continue }
        if ($char -ne '\') { continue }
        if ($index + 1 -ge $Text.Length) { return }
        $escape = $Text[$index + 1]
        if ($escape -ne 'u') { $index++; continue }
        if ($index + 5 -ge $Text.Length) { return }
        $hex = $Text.Substring($index + 2, 4)
        $code = [Convert]::ToInt32($hex, 16)
        if ($code -ge 0xD800 -and $code -le 0xDBFF) {
            if ($index + 11 -ge $Text.Length -or $Text[$index + 6] -ne '\' -or $Text[$index + 7] -ne 'u') {
                throw "R7_STRICT_JSON_ISOLATED_SURROGATE escape=$hex"
            }
            $lowHex = $Text.Substring($index + 8, 4)
            $low = [Convert]::ToInt32($lowHex, 16)
            if ($low -lt 0xDC00 -or $low -gt 0xDFFF) { throw "R7_STRICT_JSON_ISOLATED_SURROGATE escape=$hex$lowHex" }
            $index += 11
        } elseif ($code -ge 0xDC00 -and $code -le 0xDFFF) {
            throw "R7_STRICT_JSON_ISOLATED_SURROGATE escape=$hex"
        } else {
            $index += 5
        }
    }
}

function Assert-R7StrictIJsonToken {
    param([Newtonsoft.Json.Linq.JToken]$Token, [string]$Location = '$')
    if ($Token -is [Newtonsoft.Json.Linq.JObject]) {
        foreach ($property in $Token.Properties()) {
            Assert-R7StrictUnicodeScalarString $property.Name "$Location.<name>"
            Assert-R7StrictIJsonToken $property.Value "$Location.$($property.Name)"
        }
        return
    }
    if ($Token -is [Newtonsoft.Json.Linq.JArray]) {
        $index = 0
        foreach ($item in $Token.Children()) { Assert-R7StrictIJsonToken $item "$Location[$index]"; $index++ }
        return
    }
    if ($Token.Type -eq [Newtonsoft.Json.Linq.JTokenType]::String) {
        Assert-R7StrictUnicodeScalarString ([string]$Token.Value) $Location
        return
    }
    if ($Token.Type -eq [Newtonsoft.Json.Linq.JTokenType]::Integer) {
        $integer = [System.Numerics.BigInteger]::Parse($Token.ToString([Newtonsoft.Json.Formatting]::None), [System.Globalization.CultureInfo]::InvariantCulture)
        $limit = [System.Numerics.BigInteger]::new(9007199254740991L)
        if ($integer -gt $limit -or $integer -lt -$limit) { throw "R7_STRICT_JSON_UNSAFE_INTEGER location=$Location value=$integer" }
        return
    }
    if ($Token.Type -eq [Newtonsoft.Json.Linq.JTokenType]::Float) {
        $number = [double]$Token.Value
        if ([double]::IsNaN($number) -or [double]::IsInfinity($number)) { throw "R7_STRICT_JSON_NONFINITE_NUMBER location=$Location" }
    }
}

function Read-R7StrictJsonInProcess {
    param([string]$Path, [string]$SchemaPath = "", [switch]$EmitCanonical)
    $resolvedPath = (Resolve-Path -LiteralPath $Path).Path
    $bytes = [System.IO.File]::ReadAllBytes($resolvedPath)
    $utf8 = [System.Text.UTF8Encoding]::new($false, $true)
    try { $text = $utf8.GetString($bytes) } catch { throw "R7_STRICT_JSON_INVALID_UTF8 path=$resolvedPath detail=$($_.Exception.Message)" }
    try {
        $options = [System.Text.Json.JsonDocumentOptions]::new()
        $options.AllowTrailingCommas = $false
        $options.CommentHandling = [System.Text.Json.JsonCommentHandling]::Disallow
        $document = [System.Text.Json.JsonDocument]::Parse($text, $options)
        $document.Dispose()
    } catch { throw "R7_STRICT_JSON_SYNTAX path=$resolvedPath detail=$($_.Exception.Message)" }
    Assert-R7StrictEscapedSurrogates $text
    $settings = [Newtonsoft.Json.Linq.JsonLoadSettings]::new()
    $settings.CommentHandling = [Newtonsoft.Json.Linq.CommentHandling]::Ignore
    $settings.DuplicatePropertyNameHandling = [Newtonsoft.Json.Linq.DuplicatePropertyNameHandling]::Error
    try { $token = [Newtonsoft.Json.Linq.JToken]::Parse($text, $settings) } catch { throw "R7_STRICT_JSON_DUPLICATE_OR_INVALID path=$resolvedPath detail=$($_.Exception.Message)" }
    Assert-R7StrictIJsonToken $token
    if (-not [string]::IsNullOrWhiteSpace($SchemaPath)) {
        $resolvedSchema = (Resolve-Path -LiteralPath $SchemaPath).Path
        if (-not ($text | Test-Json -SchemaFile $resolvedSchema -ErrorAction Stop)) { throw "R7_STRICT_JSON_SCHEMA path=$resolvedPath schema=$resolvedSchema" }
    }
    $canonical = (ConvertTo-R7StrictCanonicalToken $token).ToString([Newtonsoft.Json.Formatting]::None)
    if ($EmitCanonical) { return $canonical }
    $canonical | ConvertFrom-Json -Depth 100
}
