function Get-R7MatrixFinalStatusProvenance {
    param(
        [string]$MatrixStatusPath,
        [string]$ManifestPath,
        [string]$RepoCommit,
        [int]$RunCount,
        $EvaluationAuthority,
        [System.Collections.Generic.List[object]]$Findings
    )
    if ([string]::IsNullOrWhiteSpace($MatrixStatusPath)) {
        return [pscustomobject]@{ status = $null; fact = $null }
    }
    $matrixStatus = Read-R7ProvenanceJson `
        $MatrixStatusPath `
        $Findings `
        "matrix_final_status_missing" `
        "matrix_final_status_invalid"
    $matrixStatusFact = Get-R7ProvenanceFileFact `
        $MatrixStatusPath `
        $Findings `
        "matrix_final_status_missing"
    if ($matrixStatus) {
        $inputFacts = @(Get-R7ProvenanceProperty $matrixStatus "inputs" @())
        $outputFacts = @(Get-R7ProvenanceProperty $matrixStatus "outputs" @())
        $requiredNames = @("summary.csv", "aggregate.csv", "trace-analysis.json", "report.md")
        $actualNames = @($outputFacts | ForEach-Object { Split-Path -Leaf ([string]$_.path) })
        $manifestInputs = @($inputFacts | Where-Object { [string]$_.role -eq "run_manifest" })
        $contractInputs = @($inputFacts | Where-Object { [string]$_.role -eq "evaluation_contract" })
        if ([int]$matrixStatus.schema_version -ne 1 -or
            [string]$matrixStatus.status -ne "finalized" -or
            -not [bool]$matrixStatus.final_aggregate_ready -or
            [string]$matrixStatus.repo_commit -ne $RepoCommit -or
            [int]$matrixStatus.run_count -ne $RunCount -or
            $inputFacts.Count -ne 2 -or
            $manifestInputs.Count -ne 1 -or
            $contractInputs.Count -ne 1 -or
            -not [string]::Equals(
                [IO.Path]::GetFullPath([string]$manifestInputs[0].path),
                [IO.Path]::GetFullPath($ManifestPath),
                [StringComparison]::OrdinalIgnoreCase
            ) -or
            $null -eq $EvaluationAuthority -or
            -not [string]::Equals(
                [IO.Path]::GetFullPath([string]$contractInputs[0].path),
                [string]$EvaluationAuthority.contract_path,
                [StringComparison]::OrdinalIgnoreCase
            ) -or
            (Compare-Object $requiredNames $actualNames)) {
            Add-R7ProvenanceFinding `
                $Findings `
                "matrix_final_status_identity_mismatch" `
                $MatrixStatusPath
        }
        foreach ($inputFact in $inputFacts) {
            if (-not (Test-R7ProvenanceFileFact $inputFact)) {
                Add-R7ProvenanceFinding `
                    $Findings `
                    "matrix_final_input_hash_mismatch" `
                    ([string]$inputFact.path)
            }
        }
        foreach ($outputFact in $outputFacts) {
            if (-not (Test-R7ProvenanceFileFact $outputFact)) {
                Add-R7ProvenanceFinding `
                    $Findings `
                    "matrix_final_output_hash_mismatch" `
                    ([string]$outputFact.path)
            }
        }
    }
    [pscustomobject]@{ status = $matrixStatus; fact = $matrixStatusFact }
}
