param(
    [string]$OutputDir = ""
)

$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "action-map-observability-lib.ps1")

if (-not $OutputDir) {
    $OutputDir = Join-Path $PSScriptRoot "..\target\test-reports\action-map-reparse-containment"
}

function Assert-Equal($Actual, $Expected, [string]$Message) {
    if ($Actual -ne $Expected) {
        throw "$Message. Expected '$Expected', got '$Actual'."
    }
}

function Assert-Contains($Items, [string]$Expected, [string]$Message) {
    if (@($Items) -notcontains $Expected) {
        throw "$Message. Missing '$Expected'."
    }
}

if ($env:OS -ne "Windows_NT") {
    Write-Host "reparse-containment: SKIP non-Windows host"
    Write-Host "Overall: SKIP"
    exit 0
}

$runRoot = Join-Path $OutputDir (Get-Date -Format "yyyyMMdd-HHmmss-fff")
$artifactRoot = Join-Path $runRoot "artifact-root"
$outsideRoot = Join-Path $runRoot "outside-root"
$linkDir = Join-Path $artifactRoot "linked"
[void](New-Item -ItemType Directory -Force -Path $artifactRoot, $outsideRoot)
"outside" | Set-Content -LiteralPath (Join-Path $outsideRoot "secret.txt") -Encoding UTF8

try {
    [void](New-Item -ItemType Junction -Path $linkDir -Target $outsideRoot -ErrorAction Stop)
} catch {
    throw "Failed to create Windows junction for reparse containment test: $($_.Exception.Message)"
}

$artifactRef = "linked/secret.txt"
$resultId = "result-reparse"
$evidencePackage = [pscustomobject]@{
    claims = @([pscustomobject]@{
            id = "claim-reparse"
            statement = "outside artifact was produced"
            evidenceRefs = @([pscustomobject]@{ resultId = $resultId; validatorRef = "validator" })
        })
    evidenceRefs = @([pscustomobject]@{ resultId = $resultId; validatorRef = "validator" })
    changedArtifacts = @($artifactRef)
    validatorRefs = @("validator")
    remainingUncertainty = @()
    validity = "accepted"
    validityReason = "validator passed"
}

$nodes = @{}
$node = Ensure-Node $nodes "node-reparse" "Reparse output" "implement_patch"
Add-Or-Update-NodeResult $node "2026-06-05T00:00:00Z" $resultId "lease-reparse" "thread-1" "result" "implement" "output" $evidencePackage "map-1" "task-reparse"

$tasks = New-Object System.Collections.Generic.List[object]
$taskById = @{}
$cognitiveState = [pscustomobject]@{
    outputContracts = @([pscustomobject]@{ id = "contract-reparse"; kind = "artifact"; artifactRef = $artifactRef; evidenceRefs = @([pscustomobject]@{ resultId = $resultId }) })
    factSources = @([pscustomobject]@{ id = "source-reparse"; provenance = "observed_from_environment"; description = "validator"; evidenceRefs = @([pscustomobject]@{ validatorRef = "validator" }) })
    facts = @([pscustomobject]@{ id = "fact-reparse"; statement = "validator passed"; evidenceRefs = @([pscustomobject]@{ factSourceId = "source-reparse" }) })
    assumptions = @()
    riskNotes = @()
    successCriteria = @()
}
[void](Ensure-Task $tasks $taskById "task-reparse" "Reparse containment" "Reject reparse escape" "active" "thread-1" "map-1" @("map-1") $cognitiveState)

$timeline = New-Object System.Collections.Generic.List[object]
Add-TimelineEvent $timeline "2026-06-05T00:01:00Z" "result_validity_changed" "result accepted" ([pscustomobject]@{ resultId = $resultId })

$audit = Get-CognitiveAuditSummary @($tasks.ToArray()) @($nodes.Values) @() @($timeline.ToArray()) $artifactRoot
Assert-Equal ([bool]$audit.hardGatePassed) $false "reparse-point artifact escape should fail final artifact audit"
Assert-Contains $audit.hardGateFailures "final_artifact_hash_missing" "reparse escape should be reported as missing hash"
Assert-Equal ([int]$audit.metrics.finalArtifactCount) 1 "audit should still record the logical final artifact"

$artifact = @($audit.finalArtifacts | Select-Object -First 1)[0]
Assert-Equal ([bool]$artifact.artifactFound) $false "reparse escape should not be treated as an in-root artifact"
Assert-Equal ([string]$artifact.resolvedPath) "" "reparse escape should not expose an accepted resolved path"
Assert-Equal ([string]$artifact.artifactHash) "" "reparse escape should not be hashed"

$deepTarget = Join-Path $runRoot "deep-target"
$deepOutside = Join-Path $runRoot "deep-outside"
[void](New-Item -ItemType Directory -Force -Path $deepTarget, $deepOutside)
$absoluteOutside = Join-Path $deepOutside "absolute.txt"
"outside absolute" | Set-Content -LiteralPath $absoluteOutside -Encoding UTF8
for ($i = 18; $i -ge 0; $i--) {
    $linkPath = Join-Path $runRoot "deep-link-$i"
    $targetPath = if ($i -eq 18) { $deepTarget } else { Join-Path $runRoot "deep-link-$($i + 1)" }
    [void](New-Item -ItemType Junction -Path $linkPath -Target $targetPath -ErrorAction Stop)
}

$deepResultId = "result-deep-root"
$deepEvidence = [pscustomobject]@{
    claims = @([pscustomobject]@{ id = "claim-deep-root"; statement = "absolute outside artifact"; evidenceRefs = @([pscustomobject]@{ resultId = $deepResultId; validatorRef = "validator" }) })
    evidenceRefs = @([pscustomobject]@{ resultId = $deepResultId; validatorRef = "validator" })
    changedArtifacts = @($absoluteOutside)
    validatorRefs = @("validator")
    remainingUncertainty = @()
    validity = "accepted"
    validityReason = "validator passed"
}
$deepNodes = @{}
$deepNode = Ensure-Node $deepNodes "node-deep-root" "Deep root" "implement_patch"
Add-Or-Update-NodeResult $deepNode "2026-06-05T00:02:00Z" $deepResultId "lease-deep-root" "thread-1" "result" "implement" "output" $deepEvidence "map-1" "task-deep-root"
$deepTasks = New-Object System.Collections.Generic.List[object]
$deepTaskById = @{}
$deepState = [pscustomobject]@{
    outputContracts = @([pscustomobject]@{ id = "contract-deep-root"; kind = "artifact"; artifactRef = $absoluteOutside; evidenceRefs = @([pscustomobject]@{ resultId = $deepResultId }) })
    factSources = @([pscustomobject]@{ id = "source-deep-root"; provenance = "observed_from_environment"; description = "validator"; evidenceRefs = @([pscustomobject]@{ validatorRef = "validator" }) })
    facts = @([pscustomobject]@{ id = "fact-deep-root"; statement = "validator passed"; evidenceRefs = @([pscustomobject]@{ factSourceId = "source-deep-root" }) })
    assumptions = @()
    riskNotes = @()
    successCriteria = @()
}
[void](Ensure-Task $deepTasks $deepTaskById "task-deep-root" "Deep root containment" "Reject unresolved artifact root" "active" "thread-1" "map-1" @("map-1") $deepState)
Add-TimelineEvent $timeline "2026-06-05T00:03:00Z" "result_validity_changed" "deep result accepted" ([pscustomobject]@{ resultId = $deepResultId })
$deepAudit = Get-CognitiveAuditSummary @($deepTasks.ToArray()) @($deepNodes.Values) @() @($timeline.ToArray()) (Join-Path $runRoot "deep-link-0")
Assert-Equal ([bool]$deepAudit.hardGatePassed) $false "unresolved reparse ArtifactRoot should fail closed"
Assert-Contains $deepAudit.hardGateFailures "final_artifact_hash_missing" "unresolved reparse root should be reported as missing hash"
$deepArtifact = @($deepAudit.finalArtifacts | Select-Object -First 1)[0]
Assert-Equal ([bool]$deepArtifact.artifactFound) $false "unresolved root should not accept absolute outside artifact"
Assert-Equal ([string]$deepArtifact.artifactHash) "" "unresolved root should not hash absolute outside artifact"

Write-Host "cognitive-audit-artifact-root-reparse-containment: PASS"
Write-Host "cognitive-audit-artifact-root-unresolved-fail-closed: PASS"
Write-Host "Overall: PASS"
