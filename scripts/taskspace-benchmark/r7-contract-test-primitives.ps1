function Assert-True {
    param([bool]$Condition, [string]$Message)
    if (-not $Condition) { throw $Message }
}

function Assert-Equal {
    param($Actual, $Expected, [string]$Message)
    if ($Actual -cne $Expected) { throw "$Message. expected=$Expected actual=$Actual" }
}

function Get-Sha256 {
    param([string]$Path)
    (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Get-TextSha256 {
    param([string]$Text)
    Get-BytesSha256 ([System.Text.Encoding]::UTF8.GetBytes($Text))
}

function Get-BytesSha256 {
    param([byte[]]$Bytes)
    [System.BitConverter]::ToString(
        [System.Security.Cryptography.SHA256]::Create().ComputeHash($Bytes)
    ).Replace("-", "").ToLowerInvariant()
}

function Get-GitBlobBytes {
    param([string]$Commit, [string]$Path)
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = "git"
    foreach ($argument in @("-C", $repoRoot, "show", "${Commit}:$Path")) { $startInfo.ArgumentList.Add($argument) }
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo
    [void]$process.Start()
    $errorTask = $process.StandardError.ReadToEndAsync()
    $buffer = [System.IO.MemoryStream]::new()
    $process.StandardOutput.BaseStream.CopyTo($buffer)
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw "Unable to read frozen blob ${Commit}:$Path $($errorTask.Result)" }
    $buffer.ToArray()
}

function Get-GitBlobText {
    param([string]$Commit, [string]$Path)
    [System.Text.UTF8Encoding]::new($false, $true).GetString((Get-GitBlobBytes $Commit $Path))
}

function Get-GitBlobSha256 {
    param([string]$Commit, [string]$Path)
    Get-BytesSha256 (Get-GitBlobBytes $Commit $Path)
}

function ConvertTo-CanonicalValue {
    param($Value)
    if ($null -eq $Value) { return $null }
    if ($Value -is [System.Collections.IDictionary]) {
        $ordered = [ordered]@{}
        foreach ($key in @($Value.Keys | ForEach-Object { [string]$_ } | Sort-Object)) {
            $ordered[$key] = ConvertTo-CanonicalValue $Value[$key]
        }
        return [pscustomobject]$ordered
    }
    if ($Value -is [pscustomobject]) {
        $ordered = [ordered]@{}
        foreach ($property in @($Value.psobject.Properties | Sort-Object Name)) {
            $ordered[$property.Name] = ConvertTo-CanonicalValue $property.Value
        }
        return [pscustomobject]$ordered
    }
    if ($Value -is [System.Collections.IEnumerable] -and $Value -isnot [string]) {
        $items = @()
        foreach ($item in $Value) { $items += ,(ConvertTo-CanonicalValue $item) }
        return ,$items
    }
    $Value
}

function ConvertTo-CanonicalJson {
    param($Value)
    ConvertTo-CanonicalValue $Value | ConvertTo-Json -Depth 100 -Compress
}

function Assert-Throws {
    param([scriptblock]$Action, [string]$Message)
    $threw = $false
    try { & $Action } catch { $threw = $true }
    Assert-True $threw $Message
}

function Get-RustEnumVariants {
    param([string]$Path, [string]$EnumName)
    $source = Get-Content -Raw -Encoding UTF8 -LiteralPath $Path
    $marker = "pub enum $EnumName"
    $markerIndex = $source.IndexOf($marker, [System.StringComparison]::Ordinal)
    if ($markerIndex -lt 0) { throw "Rust enum not found: $EnumName in $Path" }
    $braceStart = $source.IndexOf("{", $markerIndex)
    $depth = 0
    $braceEnd = -1
    for ($index = $braceStart; $index -lt $source.Length; $index++) {
        if ($source[$index] -eq '{') { $depth++ }
        if ($source[$index] -eq '}') {
            $depth--
            if ($depth -eq 0) { $braceEnd = $index; break }
        }
    }
    if ($braceEnd -lt 0) { throw "Rust enum is not balanced: $EnumName in $Path" }
    $body = $source.Substring($braceStart + 1, $braceEnd - $braceStart - 1)
    @([regex]::Matches($body, '(?m)^    ([A-Z][A-Za-z0-9_]*)\b') | ForEach-Object { $_.Groups[1].Value })
}

function Get-CandidateContentId {
    param([object]$Candidate)
    $lines = @(
        "r7-continuous-action-candidate-id-v1",
        "active_contract=$([string]$Candidate.active_authority.contract_id)",
        "active_path=$([string]$Candidate.active_authority.path)",
        "active_commit=$([string]$Candidate.active_authority.git_commit)",
        "active_sha256=$([string]$Candidate.active_authority.sha256)",
        "production_contract=$([string]$Candidate.active_production_manifest.contract_id)",
        "production_path=$([string]$Candidate.active_production_manifest.path)",
        "production_commit=$([string]$Candidate.active_production_manifest.git_commit)",
        "production_sha256=$([string]$Candidate.active_production_manifest.sha256)",
        "activation_through=$([string]$Candidate.activation_targets.activation_through)",
        "blocking_repair=$([string]$Candidate.activation_targets.blocking_repair.id)|$([string]$Candidate.activation_targets.blocking_repair.implementation_status)",
        "runtime_status=L4:$([string]$Candidate.activation_targets.production_runtime_status.L4)|L5:$([string]$Candidate.activation_targets.production_runtime_status.L5)"
    )
    foreach ($layer in @("L4", "L5")) {
        foreach ($target in @($Candidate.activation_targets.$layer | Sort-Object artifact_role)) {
            $lines += "target=$layer|$([string]$target.artifact_role)|$([string]$target.authority_layer)|$([string]$target.implementation_status)|$([string]$target.sha256)|$([string]$target.activation_phase)"
        }
    }
    foreach ($artifact in @($Candidate.artifact_hashes.psobject.Properties | Sort-Object Name)) {
        $lines += "$([string]$artifact.Name)=$([string]$artifact.Value.sha256)"
    }
    Get-TextSha256 (($lines -join "`n") + "`n")
}
