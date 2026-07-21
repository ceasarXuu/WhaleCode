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

function Assert-Throws {
    param([scriptblock]$Action, [string]$Message)
    $threw = $false
    try { & $Action } catch { $threw = $true }
    Assert-True $threw $Message
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
        "production_sha256=$([string]$Candidate.active_production_manifest.sha256)"
    )
    foreach ($layer in @("L4", "L5")) {
        foreach ($target in @($Candidate.activation_targets.$layer | Sort-Object artifact_role)) {
            $lines += "target=$layer|$([string]$target.artifact_role)|$([string]$target.sha256)|$([string]$target.activation_phase)"
        }
    }
    foreach ($artifact in @($Candidate.artifact_hashes.psobject.Properties | Sort-Object Name)) {
        $lines += "$([string]$artifact.Name)=$([string]$artifact.Value.sha256)"
    }
    Get-TextSha256 (($lines -join "`n") + "`n")
}
