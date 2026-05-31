function Invoke-TaskspaceValidationCommand {
    param(
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][object]$Validation,
        [Parameter(Mandatory = $true)][string]$StdoutPath,
        [Parameter(Mandatory = $true)][string]$StderrPath,
        [int]$TimeoutSeconds = 120
    )
    $args = @($Validation.args | ForEach-Object { [string]$_ })
    Invoke-RealProcess ([string]$Validation.command) $args $RepoDir $StdoutPath $StderrPath $TimeoutSeconds
}

function Test-TaskspaceOracleLeak {
    param(
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [Parameter(Mandatory = $true)][string]$HiddenOraclePath,
        [string]$ScenarioOraclePath = ""
    )
    $targets = @("private-oracle", "reviewer-only")
    $targets += $HiddenOraclePath
    if ($ScenarioOraclePath) { $targets += $ScenarioOraclePath }
    $visibleRepoHits = @(Get-ChildItem -LiteralPath $RepoDir -Recurse -Force -ErrorAction SilentlyContinue |
        Where-Object {
            $full = [string]$_.FullName
            @($targets | Where-Object { $full -like "*$_*" }).Count -gt 0
        })
    $textHits = New-Object System.Collections.Generic.List[string]
    foreach ($file in @(Get-ChildItem -LiteralPath $ArtifactDir -Recurse -File -ErrorAction SilentlyContinue)) {
        try {
            $text = Get-Content -Raw -Encoding UTF8 -LiteralPath $file.FullName
        } catch {
            continue
        }
        foreach ($target in $targets) {
            if (-not [string]::IsNullOrWhiteSpace($target) -and $text.Contains($target)) {
                $textHits.Add($file.FullName)
                break
            }
        }
    }
    [pscustomobject]@{
        leaked = ($visibleRepoHits.Count -gt 0 -or $textHits.Count -gt 0)
        repo_hits = @($visibleRepoHits | ForEach-Object { $_.FullName })
        artifact_hits = @($textHits.ToArray())
    }
}

function Invoke-TaskspaceHiddenOracle {
    param(
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$ArtifactDir,
        [Parameter(Mandatory = $true)][string]$HiddenOraclePath,
        [string]$ScenarioOraclePath = "",
        [switch]$BypassSandbox
    )
    $stdoutPath = Join-Path $ArtifactDir "hidden-oracle.stdout.log"
    $stderrPath = Join-Path $ArtifactDir "hidden-oracle.stderr.log"
    $exitCode = Invoke-RealProcess "python" @($HiddenOraclePath, $RepoDir) $RepoDir $stdoutPath $stderrPath 120
    $leak = Test-TaskspaceOracleLeak $RepoDir $ArtifactDir $HiddenOraclePath $ScenarioOraclePath
    $level = if ($leak.leaked) { "failed" } elseif ($BypassSandbox) { "soft_denylist" } else { "hard_sandbox" }
    [pscustomobject]@{
        exit_code = $exitCode
        stdout_path = $stdoutPath
        stderr_path = $stderrPath
        oracle_sha256 = (Get-TaskspaceFileSha256 $HiddenOraclePath)
        oracle_isolation_level = $level
        oracle_isolation_failure = ($level -eq "failed")
        leak = $leak
    }
}
