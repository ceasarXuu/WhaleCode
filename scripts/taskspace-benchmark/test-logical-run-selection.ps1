$ErrorActionPreference = "Stop"

. (Join-Path $PSScriptRoot "lib\side-selection.ps1")

function Assert-Selected {
    param($Side, [string]$RunSide, [string]$RunLogicalMode, [bool]$Expected)
    $actual = Test-TaskspaceRunSelection $Side $RunSide $RunLogicalMode
    if ($actual -ne $Expected) {
        throw "selection mismatch: side=$($Side.Name), logical=$($Side.LogicalMode), run_side=$RunSide, run_logical_mode=$RunLogicalMode"
    }
}

$oddLeft = [pscustomobject]@{ Name = "left"; LogicalMode = "standard" }
$oddRight = [pscustomobject]@{ Name = "right"; LogicalMode = "taskspace" }
$evenLeft = [pscustomobject]@{ Name = "left"; LogicalMode = "taskspace" }
$evenRight = [pscustomobject]@{ Name = "right"; LogicalMode = "standard" }

Assert-Selected $oddLeft "both" "taskspace" $false
Assert-Selected $oddRight "both" "taskspace" $true
Assert-Selected $evenLeft "both" "taskspace" $true
Assert-Selected $evenRight "both" "taskspace" $false
Assert-Selected $oddRight "left" "taskspace" $false
Assert-Selected $evenLeft "left" "taskspace" $true
Assert-Selected $oddLeft "both" "both" $true
Assert-Selected $oddRight "both" "both" $true

Write-Host "Logical run selection contract passed."
