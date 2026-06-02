function Get-TaskspaceExternalSensitiveSourceFiles {
    param([Parameter(Mandatory = $true)]$Manifest)
    if ($null -eq $Manifest.ExternalBenchmark) { return @() }
    $files = New-Object System.Collections.Generic.List[string]
    $metadata = $Manifest.ExternalBenchmark.adapter_metadata
    if ($null -ne $metadata -and $metadata.PSObject.Properties.Name -contains "sensitive_source_files") {
        foreach ($path in @($metadata.sensitive_source_files)) {
            if (-not [string]::IsNullOrWhiteSpace([string]$path)) { $files.Add([string]$path) }
        }
    }
    $validatorRel = if ($Manifest.ExternalBenchmark.PSObject.Properties.Name -contains "validator_source_dir") {
        [string]$Manifest.ExternalBenchmark.validator_source_dir
    } else { "" }
    if (-not [string]::IsNullOrWhiteSpace($validatorRel)) {
        $validatorPath = Join-Path $Manifest.ScenarioRoot $validatorRel
        if (Test-Path -LiteralPath $validatorPath) {
            foreach ($file in Get-ChildItem -LiteralPath $validatorPath -Recurse -File -Force) {
                $files.Add($file.FullName)
            }
        }
    }
    @($files.ToArray() | Sort-Object -Unique)
}

function Get-TaskspaceTextSha256 {
    param([AllowEmptyString()][string]$Text = "")
    $bytes = [System.Text.Encoding]::UTF8.GetBytes($Text)
    $sha = [System.Security.Cryptography.SHA256]::Create()
    ([System.BitConverter]::ToString($sha.ComputeHash($bytes)) -replace "-", "").ToLowerInvariant()
}

function Invoke-TaskspaceGuardReadProbe {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Kind
    )
    $available = $true
    $exitCode = 1
    $output = ""
    try {
        if ($Kind -eq "current_powershell") {
            Get-Content -Raw -Encoding UTF8 -LiteralPath $Path -ErrorAction Stop | Out-Null
            $exitCode = 0
        } elseif ($Kind -eq "powershell_child") {
            $output = (& powershell -NoProfile -Command "Get-Content -Raw -LiteralPath '$($Path.Replace("'", "''"))'" 2>&1) -join "`n"
            $exitCode = $LASTEXITCODE
        } elseif ($Kind -eq "cmd_child") {
            $output = (& cmd /c type "$Path" 2>&1) -join "`n"
            $exitCode = $LASTEXITCODE
        } else {
            $available = $false
        }
    } catch {
        $output = [string]$_.Exception.Message
        $exitCode = 1
    }
    [pscustomobject]@{
        kind = $Kind
        available = $available
        exit_code = $exitCode
        read_denied = ($available -and $exitCode -ne 0)
        output_sha256 = Get-TaskspaceTextSha256 $output
        output_length = $output.Length
        denied_text_seen = ($output -match "Access is denied|Permission denied|拒绝访问|denied")
    }
}

function Protect-TaskspaceExternalSensitiveSource {
    param(
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)][string]$PairDir
    )
    $proofPath = Join-Path $PairDir "external-source-guard-proof.json"
    if ($null -eq $Manifest.ExternalBenchmark) {
        return [pscustomobject]@{ active = $false; proof_path = ""; files = @() }
    }
    $identity = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
    $files = @(Get-TaskspaceExternalSensitiveSourceFiles $Manifest | Where-Object { Test-Path -LiteralPath $_ })
    $rows = New-Object System.Collections.Generic.List[object]
    foreach ($file in $files) {
        $fileInfo = Get-Item -LiteralPath $file
        $preHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $file).Hash.ToLowerInvariant()
        $denyOutput = & icacls $file /deny "$($identity):(R)" 2>&1
        $denyExit = $LASTEXITCODE
        $probes = @(
            (Invoke-TaskspaceGuardReadProbe -Path $file -Kind "current_powershell"),
            (Invoke-TaskspaceGuardReadProbe -Path $file -Kind "powershell_child"),
            (Invoke-TaskspaceGuardReadProbe -Path $file -Kind "cmd_child")
        )
        $readDenied = @($probes | Where-Object { $_.available -and -not $_.read_denied }).Count -eq 0
        $rows.Add([pscustomobject]@{
            path = $file
            file_sha256_before_protect = $preHash
            file_size_before_protect = [int64]$fileInfo.Length
            deny_exit_code = $denyExit
            read_denied_after_protect = $readDenied
            probes_after_protect = @($probes)
            deny_output = (($denyOutput | ForEach-Object { [string]$_ }) -join "`n")
        })
    }
    $proof = [pscustomobject]@{
        active = $true
        proof_path = $proofPath
        identity = $identity
        process_id = $PID
        protected_at = (Get-Date).ToUniversalTime().ToString("o")
        released_at = ""
        files = @($rows.ToArray())
        protected_file_count = $files.Count
        all_reads_denied_after_protect = ($files.Count -gt 0 -and @($rows | Where-Object { -not $_.read_denied_after_protect -or $_.deny_exit_code -ne 0 }).Count -eq 0)
        required_probe_kinds = @("current_powershell", "powershell_child", "cmd_child")
        all_denies_removed_after_release = $false
        all_reads_restored_after_release = $false
    }
    Write-TaskspaceJson $proof $proofPath
    $proof
}

function Unprotect-TaskspaceExternalSensitiveSource {
    param($Guard)
    if ($null -eq $Guard -or -not [bool]$Guard.active) { return $Guard }
    $releaseRows = New-Object System.Collections.Generic.List[object]
    foreach ($row in @($Guard.files)) {
        $path = [string]$row.path
        if ([string]::IsNullOrWhiteSpace($path) -or -not (Test-Path -LiteralPath $path)) { continue }
        $removeOutput = & icacls $path /remove:d ([string]$Guard.identity) 2>&1
        $removeExit = $LASTEXITCODE
        $readRestored = $false
        try {
            Get-Content -Raw -Encoding UTF8 -LiteralPath $path -ErrorAction Stop | Out-Null
            $readRestored = $true
        } catch {}
        $releaseRows.Add([pscustomobject]@{
            path = $path
            file_sha256_after_release = if ($readRestored) { (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant() } else { "" }
            remove_exit_code = $removeExit
            read_restored_after_release = $readRestored
            remove_output = (($removeOutput | ForEach-Object { [string]$_ }) -join "`n")
        })
    }
    $Guard | Add-Member -NotePropertyName released_at -NotePropertyValue ((Get-Date).ToUniversalTime().ToString("o")) -Force
    $Guard | Add-Member -NotePropertyName release_files -NotePropertyValue @($releaseRows.ToArray()) -Force
    $Guard | Add-Member -NotePropertyName all_denies_removed_after_release -NotePropertyValue (@($releaseRows | Where-Object { $_.remove_exit_code -ne 0 }).Count -eq 0) -Force
    $Guard | Add-Member -NotePropertyName all_reads_restored_after_release -NotePropertyValue (@($releaseRows | Where-Object { -not $_.read_restored_after_release }).Count -eq 0) -Force
    Write-TaskspaceJson $Guard ([string]$Guard.proof_path)
    $Guard
}
