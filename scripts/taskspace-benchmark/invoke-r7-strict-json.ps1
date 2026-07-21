param(
    [Parameter(Mandatory = $true)]
    [string]$Path,
    [string]$SchemaPath = "",
    [switch]$EmitCanonical
)

$ErrorActionPreference = "Stop"

function ConvertTo-CanonicalToken {
    param([Newtonsoft.Json.Linq.JToken]$Token)
    if ($Token -is [Newtonsoft.Json.Linq.JObject]) {
        $result = [Newtonsoft.Json.Linq.JObject]::new()
        foreach ($property in @($Token.Properties() | Sort-Object Name)) {
            [void]$result.Add([Newtonsoft.Json.Linq.JProperty]::new(
                $property.Name,
                (ConvertTo-CanonicalToken $property.Value)
            ))
        }
        return ,$result
    }
    if ($Token -is [Newtonsoft.Json.Linq.JArray]) {
        $result = [Newtonsoft.Json.Linq.JArray]::new()
        foreach ($item in $Token.Children()) {
            [void]$result.Add((ConvertTo-CanonicalToken $item))
        }
        return ,$result
    }
    return ,($Token.DeepClone())
}

$resolvedPath = (Resolve-Path -LiteralPath $Path).Path
$bytes = [System.IO.File]::ReadAllBytes($resolvedPath)
$utf8 = [System.Text.UTF8Encoding]::new($false, $true)
try {
    $text = $utf8.GetString($bytes)
} catch {
    throw "R7_STRICT_JSON_INVALID_UTF8 path=$resolvedPath detail=$($_.Exception.Message)"
}

try {
    $documentOptions = [System.Text.Json.JsonDocumentOptions]::new()
    $documentOptions.AllowTrailingCommas = $false
    $documentOptions.CommentHandling = [System.Text.Json.JsonCommentHandling]::Disallow
    $document = [System.Text.Json.JsonDocument]::Parse($text, $documentOptions)
    $document.Dispose()
} catch {
    throw "R7_STRICT_JSON_SYNTAX path=$resolvedPath detail=$($_.Exception.Message)"
}

$loadSettings = [Newtonsoft.Json.Linq.JsonLoadSettings]::new()
$loadSettings.CommentHandling = [Newtonsoft.Json.Linq.CommentHandling]::Ignore
$loadSettings.DuplicatePropertyNameHandling = [Newtonsoft.Json.Linq.DuplicatePropertyNameHandling]::Error
try {
    $token = [Newtonsoft.Json.Linq.JToken]::Parse($text, $loadSettings)
} catch {
    throw "R7_STRICT_JSON_DUPLICATE_OR_INVALID path=$resolvedPath detail=$($_.Exception.Message)"
}

if (-not [string]::IsNullOrWhiteSpace($SchemaPath)) {
    $resolvedSchema = (Resolve-Path -LiteralPath $SchemaPath).Path
    if (-not ($text | Test-Json -SchemaFile $resolvedSchema -ErrorAction Stop)) {
        throw "R7_STRICT_JSON_SCHEMA path=$resolvedPath schema=$resolvedSchema"
    }
}

$canonical = (ConvertTo-CanonicalToken $token).ToString([Newtonsoft.Json.Formatting]::None)
if ($EmitCanonical) {
    Write-Output $canonical
    exit 0
}

$sha256 = [System.BitConverter]::ToString(
    [System.Security.Cryptography.SHA256]::Create().ComputeHash($bytes)
).Replace("-", "").ToLowerInvariant()
[pscustomobject][ordered]@{
    valid = $true
    path = $resolvedPath
    sha256 = $sha256
    canonical_sha256 = [System.BitConverter]::ToString(
        [System.Security.Cryptography.SHA256]::Create().ComputeHash($utf8.GetBytes($canonical))
    ).Replace("-", "").ToLowerInvariant()
} | ConvertTo-Json -Compress
