$script:R7RepoRoot = if ([string]::IsNullOrWhiteSpace($env:R7_REPO_ROOT)) {
    (Resolve-Path (Join-Path $PSScriptRoot "../..")).Path
} else {
    (Resolve-Path -LiteralPath $env:R7_REPO_ROOT).Path
}
$script:R7BaselineAnchorPath = "benchmarks/taskspace/r7/continuous-action-ca0-baseline-v3.json"
$script:R7ToolchainAnchorPath = "benchmarks/taskspace/r7/continuous-action-v2-toolchain-anchor-v1.json"
$script:R7AuthorityPath = "benchmarks/taskspace/r7/five-layer-contract-authority-v1.json"
$script:R7ProductionPath = "third_party/codex-cli/codex-rs/core/src/context/prompts/taskspace_contract_manifest_v1.json"
$script:R7CandidateRoot = "benchmarks/taskspace/r7/candidates"
$script:R7AnchorSchemaPath = "benchmarks/taskspace/r7/immutable-anchor-v1.schema.json"
$script:R7ArtifactNames = [ordered]@{
    l4_schema = "l4-schema.json"
    transition_schema = "transition-schema.json"
    typed_outcome = "typed-outcome.json"
    carrier_protocol_oracle = "carrier-protocol-oracle.json"
    entry_closure = "entry-closure.json"
    capability_matrix = "capability-matrix.json"
    rollback_manifest = "rollback-manifest.json"
    continuous_action_evaluation = "continuous-action-evaluation.json"
}
. (Join-Path $PSScriptRoot "r7-v2-history.ps1")
. (Join-Path $PSScriptRoot "r7-v2-promotion.ps1")
. (Join-Path $PSScriptRoot "r7-v2-git-transaction.ps1")
. (Join-Path $PSScriptRoot "lib/r7-strict-json.ps1")

function Get-R7Sha256Bytes {
    param([byte[]]$Bytes)
    [System.BitConverter]::ToString(
        [System.Security.Cryptography.SHA256]::Create().ComputeHash($Bytes)
    ).Replace("-", "").ToLowerInvariant()
}

function Get-R7Sha256File {
    param([string]$Path)
    Get-R7Sha256Bytes ([System.IO.File]::ReadAllBytes($Path))
}

function Get-R7Sha256Text {
    param([string]$Text)
    Get-R7Sha256Bytes ([System.Text.UTF8Encoding]::new($false).GetBytes($Text))
}

function ConvertTo-R7CanonicalToken {
    param([Newtonsoft.Json.Linq.JToken]$Token)
    if ($Token -is [Newtonsoft.Json.Linq.JObject]) {
        $result = [Newtonsoft.Json.Linq.JObject]::new()
        $names = [string[]]@($Token.Properties() | ForEach-Object { $_.Name })
        [System.Array]::Sort($names, [System.StringComparer]::Ordinal)
        foreach ($name in $names) {
            $child = ConvertTo-R7CanonicalToken $Token.Property($name).Value
            $result.Add($name, $child)
        }
        Write-Output -NoEnumerate $result
        return
    }
    if ($Token -is [Newtonsoft.Json.Linq.JArray]) {
        $result = [Newtonsoft.Json.Linq.JArray]::new()
        foreach ($item in $Token.Children()) {
            $result.Add((ConvertTo-R7CanonicalToken $item))
        }
        Write-Output -NoEnumerate $result
        return
    }
    Write-Output -NoEnumerate $Token.DeepClone()
}

function ConvertTo-R7CanonicalJson {
    param($Value)
    $raw = $Value | ConvertTo-Json -Depth 100 -Compress
    $token = [Newtonsoft.Json.Linq.JToken]::Parse($raw)
    $canonical = ConvertTo-R7CanonicalToken $token
    $canonical.ToString([Newtonsoft.Json.Formatting]::None)
}

function Write-R7JsonFile {
    param([string]$Path, $Value)
    $json = $Value | ConvertTo-Json -Depth 100
    [System.IO.File]::WriteAllText($Path, "$json`n", [System.Text.UTF8Encoding]::new($false))
}

function Invoke-R7Git {
    param([string[]]$Arguments, [switch]$AllowFailure)
    $output = & git -C $script:R7RepoRoot @Arguments 2>&1
    if ($LASTEXITCODE -ne 0 -and -not $AllowFailure) {
        throw "R7_GIT_FAILED args=$($Arguments -join ' ') detail=$($output -join "`n")"
    }
    @($output)
}

function Get-R7GitLine {
    param([string[]]$Arguments)
    $lines = @(Invoke-R7Git $Arguments)
    if ($lines.Count -ne 1) { throw "R7_GIT_EXPECTED_ONE_LINE args=$($Arguments -join ' ') count=$($lines.Count)" }
    ([string]$lines[0]).Trim()
}

function Get-R7GitBlobBytes {
    param([string]$Commit, [string]$Path)
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = "git"
    foreach ($argument in @("-C", $script:R7RepoRoot, "show", "${Commit}:$Path")) {
        $startInfo.ArgumentList.Add($argument)
    }
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void]$process.Start()
    $errorTask = $process.StandardError.ReadToEndAsync()
    $stream = [System.IO.MemoryStream]::new()
    $process.StandardOutput.BaseStream.CopyTo($stream)
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        throw "R7_GIT_BLOB_MISSING commit=$Commit path=$Path detail=$($errorTask.Result)"
    }
    $stream.ToArray()
}

function Get-R7GitBlobText {
    param([string]$Commit, [string]$Path)
    [System.Text.UTF8Encoding]::new($false, $true).GetString((Get-R7GitBlobBytes $Commit $Path))
}

function Get-R7GitBlobSha256 {
    param([string]$Commit, [string]$Path)
    Get-R7Sha256Bytes (Get-R7GitBlobBytes $Commit $Path)
}

function Get-R7GitDirectoryManifestSha256 {
    param([string]$Commit, [string]$Root)
    $entries = [System.Collections.Generic.List[object]]::new()
    foreach ($line in @(Invoke-R7Git @("ls-tree", "-r", $Commit, "--", $Root))) {
        $match = [regex]::Match([string]$line, '^(?<mode>[0-9]{6}) (?<type>[^ ]+) [0-9a-f]{40}\t(?<path>.+)$')
        if (-not $match.Success -or $match.Groups['mode'].Value -cne "100644" -or $match.Groups['type'].Value -cne "blob") {
            throw "R7_DIRECTORY_ENTRY_INVALID root=$Root line=$line"
        }
        $path = $match.Groups['path'].Value
        $entries.Add([pscustomobject][ordered]@{
            path = $path.Substring($Root.TrimEnd('/').Length + 1)
            git_mode = "100644"
            sha256 = Get-R7GitBlobSha256 $Commit $path
        })
    }
    if ($entries.Count -eq 0) { throw "R7_DIRECTORY_MANIFEST_EMPTY root=$Root" }
    Get-R7JsonValueHash @($entries.ToArray() | Sort-Object path)
}

function Get-R7WorktreeDirectoryManifestSha256 {
    param([string]$Root)
    $paths = @(Invoke-R7Git @("ls-files", "--cached", "--others", "--exclude-standard", "--", $Root) | Sort-Object -Unique)
    if ($paths.Count -eq 0) { throw "R7_DIRECTORY_MANIFEST_EMPTY root=$Root" }
    $entries = foreach ($path in $paths) {
        $full = Join-Path $script:R7RepoRoot $path
        if (-not (Test-Path -LiteralPath $full -PathType Leaf) -or (Get-Item -LiteralPath $full -Force).LinkType) {
            throw "R7_DIRECTORY_ENTRY_INVALID root=$Root path=$path"
        }
        [pscustomobject][ordered]@{
            path = $path.Substring($Root.TrimEnd('/').Length + 1)
            git_mode = "100644"
            sha256 = Get-R7Sha256File $full
        }
    }
    Get-R7JsonValueHash @($entries)
}

function Read-R7StrictJson {
    param([string]$Path, [string]$SchemaPath = "")
    if ([string]::IsNullOrWhiteSpace($env:R7_STRICT_PARSER_PATH)) {
        return Read-R7StrictJsonInProcess $Path $SchemaPath
    }
    $parser = $env:R7_STRICT_PARSER_PATH
    $arguments = @("-NoLogo", "-NoProfile", "-File", $parser, "-Path", $Path, "-EmitCanonical")
    if (-not [string]::IsNullOrWhiteSpace($SchemaPath)) { $arguments += @("-SchemaPath", $SchemaPath) }
    $canonical = & pwsh @arguments
    if ($LASTEXITCODE -ne 0) { throw "R7_STRICT_JSON_CHILD_FAILED path=$Path" }
    ($canonical -join "`n") | ConvertFrom-Json -Depth 100
}

function Get-R7CandidateId {
    param($Identity)
    Get-R7Sha256Text ("r7-continuous-action-candidate-v2`n$(ConvertTo-R7CanonicalJson $Identity)`n")
}

function Get-R7CandidatePath {
    param([string]$CandidateId)
    if ($CandidateId -notmatch '^[0-9a-f]{64}$') { throw "R7_CANDIDATE_ID_INVALID" }
    $relative = "$script:R7CandidateRoot/$CandidateId"
    $full = [System.IO.Path]::GetFullPath((Join-Path $script:R7RepoRoot $relative))
    $root = [System.IO.Path]::GetFullPath((Join-Path $script:R7RepoRoot "$script:R7CandidateRoot/"))
    if (-not $full.StartsWith($root, [System.StringComparison]::Ordinal)) { throw "R7_CANDIDATE_PATH_ESCAPE" }
    [pscustomobject]@{relative = $relative; full = $full}
}

function Get-R7JsonValueHash {
    param($Value)
    Get-R7Sha256Text (ConvertTo-R7CanonicalJson $Value)
}

function New-R7PatchOperation {
    param([string]$Op, [string]$Path, $OldValue, $Value)
    $oldHash = if ($Op -eq "add") {
        Get-R7Sha256Text "r7-json-pointer-absent-v1"
    } else {
        Get-R7JsonValueHash $OldValue
    }
    $operation = [ordered]@{op = $Op; path = $Path; old_value_sha256 = $oldHash}
    if ($Op -ne "remove") {
        $operation.value = $Value
        $operation.new_value_sha256 = Get-R7JsonValueHash $Value
    }
    [pscustomobject]$operation
}

function Get-R7PointerSegments {
    param([string]$Path)
    if (-not $Path.StartsWith("/", [System.StringComparison]::Ordinal)) { throw "R7_JSON_POINTER_INVALID path=$Path" }
    @($Path.Substring(1).Split('/') | ForEach-Object { $_.Replace("~1", "/").Replace("~0", "~") })
}

function Get-R7PointerParent {
    param($Document, [string]$Path)
    $segments = @(Get-R7PointerSegments $Path)
    if ($segments.Count -eq 0) { throw "R7_JSON_POINTER_ROOT_FORBIDDEN" }
    $cursor = $Document
    for ($index = 0; $index -lt ($segments.Count - 1); $index++) {
        $segment = $segments[$index]
        if ($cursor -is [System.Collections.IList]) {
            $cursor = $cursor[[int]$segment]
        } else {
            $property = $cursor.psobject.Properties[$segment]
            if ($null -eq $property) { throw "R7_JSON_POINTER_PARENT_MISSING path=$Path segment=$segment" }
            $cursor = $property.Value
        }
    }
    [pscustomobject]@{parent = $cursor; leaf = $segments[-1]}
}

function Invoke-R7JsonPatch {
    param($Document, [object[]]$Operations)
    $clone = (ConvertTo-R7CanonicalJson $Document) | ConvertFrom-Json -Depth 100
    foreach ($operation in $Operations) {
        $location = Get-R7PointerParent $clone ([string]$operation.path)
        $parent = $location.parent
        $leaf = [string]$location.leaf
        if ($parent -is [System.Collections.IList]) {
            $arrayIndex = [int]$leaf
            $exists = $arrayIndex -ge 0 -and $arrayIndex -lt $parent.Count
            $oldValue = if ($exists) { $parent[$arrayIndex] } else { $null }
        } else {
            $property = $parent.psobject.Properties[$leaf]
            $exists = $null -ne $property
            $oldValue = if ($exists) { $property.Value } else { $null }
        }
        $expectedOld = if ([string]$operation.op -eq "add") { Get-R7Sha256Text "r7-json-pointer-absent-v1" } else { Get-R7JsonValueHash $oldValue }
        if ($expectedOld -cne [string]$operation.old_value_sha256) { throw "R7_JSON_PATCH_OLD_VALUE_DRIFT path=$($operation.path)" }
        switch ([string]$operation.op) {
            "add" {
                if ($exists) { throw "R7_JSON_PATCH_ADD_EXISTS path=$($operation.path)" }
                if ($parent -is [System.Collections.IList]) { throw "R7_JSON_PATCH_ARRAY_ADD_FORBIDDEN path=$($operation.path)" }
                if ((Get-R7JsonValueHash $operation.value) -cne [string]$operation.new_value_sha256) { throw "R7_JSON_PATCH_NEW_VALUE_DRIFT path=$($operation.path)" }
                $parent | Add-Member -NotePropertyName $leaf -NotePropertyValue $operation.value
            }
            "replace" {
                if (-not $exists) { throw "R7_JSON_PATCH_REPLACE_MISSING path=$($operation.path)" }
                if ((Get-R7JsonValueHash $operation.value) -cne [string]$operation.new_value_sha256) { throw "R7_JSON_PATCH_NEW_VALUE_DRIFT path=$($operation.path)" }
                if ($parent -is [System.Collections.IList]) { $parent[$arrayIndex] = $operation.value } else { $parent.$leaf = $operation.value }
            }
            "remove" {
                if (-not $exists) { throw "R7_JSON_PATCH_REMOVE_MISSING path=$($operation.path)" }
                if ($parent -is [System.Collections.IList]) { $parent.RemoveAt($arrayIndex) } else { $parent.psobject.Properties.Remove($leaf) }
            }
            default { throw "R7_JSON_PATCH_OP_INVALID op=$($operation.op)" }
        }
    }
    $clone
}

function Assert-R7CleanWorktree {
    $status = Invoke-R7Git @("status", "--porcelain")
    if ($status.Count -ne 0) { throw "R7_WORKTREE_NOT_CLEAN`n$($status -join "`n")" }
}
