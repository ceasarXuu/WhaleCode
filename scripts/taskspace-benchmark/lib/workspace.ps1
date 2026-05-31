function New-TaskspaceBenchmarkRun {
    param(
        [Parameter(Mandatory = $true)][string]$RunRoot,
        [Parameter(Mandatory = $true)][string]$ScenarioId
    )
    $stamp = Get-Date -Format "yyyyMMdd-HHmmss-fff"
    New-Dir (Join-Path $RunRoot "$ScenarioId\$stamp")
}

function Get-NeutralTaskspaceBenchmarkRunRoot {
    param([Parameter(Mandatory = $true)][string]$RepoRoot)
    Join-Path ([System.IO.Path]::GetTempPath()) "whale-paired-bench-runs"
}

function Get-TaskspaceModeMapping {
    param([Parameter(Mandatory = $true)][int]$Repeat)
    if ($Repeat % 2 -eq 1) {
        return [ordered]@{ left = "standard"; right = "taskspace" }
    }
    [ordered]@{ left = "taskspace"; right = "standard" }
}

function Initialize-TaskspaceRepoBaseline {
    param([Parameter(Mandatory = $true)][string]$RepoDir)
    Push-Location $RepoDir
    try {
        git init | Out-Null
        git config user.email "taskspace-benchmark@example.local" | Out-Null
        git config user.name "TaskSpace Benchmark" | Out-Null
        git add . | Out-Null
        git commit -m "baseline fixture" | Out-Null
        if ((git status --porcelain) -ne $null) {
            $status = git status --porcelain
            if ($status) { throw "Fixture repo is dirty after baseline commit: $status" }
        }
    } finally {
        Pop-Location
    }
}

function New-TaskspacePairWorkspace {
    param(
        [Parameter(Mandatory = $true)]$Manifest,
        [Parameter(Mandatory = $true)][string]$RunDir,
        [Parameter(Mandatory = $true)][int]$Repeat
    )
    $repeatDir = New-Dir (Join-Path $RunDir ("pair-{0:000}" -f $Repeat))
    $reviewerOracleDir = New-Dir (Join-Path $repeatDir "reviewer-only\private-oracle")
    $canaryText = "TASKSPACE_BENCH_PRIVATE_CANARY_$([guid]::NewGuid().ToString("N"))"
    $canaryPath = Join-Path $reviewerOracleDir "canary.txt"
    $mapping = Get-TaskspaceModeMapping $Repeat
    $sides = @{}
    foreach ($side in @("left", "right")) {
        $repoDir = New-Dir (Join-Path $repeatDir "$side\repo")
        $artifactDir = New-Dir (Join-Path $repeatDir "$side\artifacts")
        Copy-Item -Path (Join-Path $Manifest.FixtureDir "*") -Destination $repoDir -Recurse -Force
        Initialize-TaskspaceRepoBaseline $repoDir
        $sides[$side] = [pscustomobject]@{
            Name = $side
            LogicalMode = [string]$mapping[$side]
            RepoDir = $repoDir
            ArtifactDir = $artifactDir
        }
    }
    $mapPath = Join-Path $repeatDir "logical-mode-map.json"
    ([pscustomobject]@{ repeat = $Repeat; left = $mapping.left; right = $mapping.right } |
        ConvertTo-Json -Depth 5) | Set-Content -LiteralPath $mapPath -Encoding UTF8
    [pscustomobject]@{
        Repeat = $Repeat
        PairDir = $repeatDir
        ReviewerOracleDir = $reviewerOracleDir
        HiddenOraclePath = Join-Path $reviewerOracleDir "oracle.py"
        HiddenOracleStrategy = $Manifest.HiddenOracleStrategy
        CanaryPath = $canaryPath
        CanaryText = $canaryText
        LogicalModeMapPath = $mapPath
        Left = $sides["left"]
        Right = $sides["right"]
    }
}

function Materialize-TaskspacePrivateOracle {
    param(
        [Parameter(Mandatory = $true)]$Pair,
        [Parameter(Mandatory = $true)]$Manifest
    )
    Write-TaskspaceGeneratedHiddenOracle $Pair.HiddenOraclePath $Manifest.HiddenOracleStrategy
    Write-Text $Pair.CanaryPath $Pair.CanaryText
}

function Write-TaskspaceGeneratedHiddenOracle {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Strategy
    )
    if ($Strategy -eq "tax-calc-v1") {
        Write-Text $Path @'
import pathlib
import random
import sys

repo = pathlib.Path(sys.argv[1]).resolve()
sys.path.insert(0, str(repo / "src"))

from tax_calc import calculate_tax, calculate_total

rates = {"CA": 0.0725, "NY": 0.08875, "TX": 0.0625}
rng = random.Random(20260531)
for region, rate in rates.items():
    for subtotal in [0.01, 10, 19.99, 123.45, rng.uniform(1, 250)]:
        expected_tax = round(subtotal * rate, 2)
        assert calculate_tax(subtotal, region) == expected_tax
        assert calculate_total(subtotal, region) == round(subtotal + expected_tax, 2)
try:
    calculate_tax(-1, "CA")
except ValueError as exc:
    assert "subtotal" in str(exc).lower()
else:
    raise AssertionError("negative subtotal should fail")
try:
    calculate_tax(1, "WA")
except ValueError as exc:
    assert "unsupported" in str(exc).lower()
else:
    raise AssertionError("unknown region should fail")
print("hidden oracle passed")
'@
        return
    }

    if ($Strategy -eq "order-pipeline-v1") {
        Write-Text $Path @'
import pathlib
import sys

repo = pathlib.Path(sys.argv[1]).resolve()
sys.path.insert(0, str(repo / "src"))

from order_pipeline.invoice import invoice_total
from order_pipeline.parser import parse_order_line
from order_pipeline.pricing import add_shipping, apply_discount

assert parse_order_line(" SKU-1 , 2 , 19.50 ") == {
    "sku": "sku-1",
    "quantity": 2,
    "unit_price": 19.5,
}
for bad_line in ["sku-1,0,19.50", "sku-1,-2,19.50"]:
    try:
        parse_order_line(bad_line)
    except ValueError as exc:
        assert "quantity" in str(exc).lower()
    else:
        raise AssertionError("non-positive quantity should fail")
assert apply_discount(100, "Premium") == 90
assert apply_discount(200, "VIP") == 170
assert apply_discount(75, "standard") == 75
assert add_shipping(49.99) == 54.99
assert add_shipping(50) == 50
assert invoice_total([" SKU-1 , 2 , 20.00 ", "sku-2,1,10.00"], "Premium") == 50.0
assert invoice_total(["sku-1,3,25.00"], "vip") == 63.75
print("hidden oracle passed")
'@
        return
    }

    if ($Strategy -eq "subscription-billing-v1") {
        Write-Text $Path @'
import pathlib
import sys

repo = pathlib.Path(sys.argv[1]).resolve()
sys.path.insert(0, str(repo / "src"))

from billing_service.billing import invoice_total
from billing_service.plans import plan_subtotal
from billing_service.usage import parse_usage_row

parsed = parse_usage_row(" acct-7 , Pro , 3 , Annual ")
assert parsed == {
    "account": "acct-7",
    "plan": "pro",
    "seats": 3,
    "billing_period": "annual",
}
for bad in ["acct,basic,0,monthly", "acct,basic,-1,monthly"]:
    try:
        parse_usage_row(bad)
    except ValueError as exc:
        assert "seats" in str(exc).lower()
    else:
        raise AssertionError("non-positive seats should fail")
try:
    plan_subtotal("unknown", 1, "monthly")
except ValueError as exc:
    assert "plan" in str(exc).lower()
else:
    raise AssertionError("unknown plan should fail")
assert plan_subtotal("basic", 2, "monthly") == 20
assert plan_subtotal("pro", 3, "annual") == 870
assert plan_subtotal("enterprise", 1, "annual") == 990
assert invoice_total("acct-1,pro,3,annual", "US") == 930.9
assert invoice_total("acct-2,basic,2,monthly", "EU") == 24.0
print("hidden oracle passed")
'@
        return
    }

    throw "Unsupported hidden oracle strategy: $Strategy"
}

function Test-TaskspaceNeutralCwd {
    param([Parameter(Mandatory = $true)][string]$Path)
    $resolved = if (Test-Path -LiteralPath $Path) { (Resolve-Path -LiteralPath $Path).Path } else { $Path }
    $lower = $resolved.ToLowerInvariant()
    foreach ($forbidden in @("standard", "taskspace", "action-map", " map", "\map", "/map", "node", "subagent")) {
        if ($lower.Contains($forbidden)) { return $false }
    }
    return $true
}

function New-TaskspaceWhaleArgv {
    param(
        [Parameter(Mandatory = $true)][string]$LogicalMode,
        [Parameter(Mandatory = $true)][string]$Model,
        [Parameter(Mandatory = $true)][string]$RepoDir,
        [Parameter(Mandatory = $true)][string]$LastMessagePath,
        [string]$SandboxMode = "bypass",
        [string[]]$ConfigOverrides = @()
    )
    $args = @("exec", "--json")
    if ($LogicalMode -eq "taskspace") { $args += "--taskspace" }
    foreach ($override in @($ConfigOverrides)) { $args += @("-c", $override) }
    $args += @("-m", $Model, "-C", $RepoDir)
    if ($SandboxMode -eq "full-auto") {
        $args += "--full-auto"
    } elseif ($SandboxMode -eq "workspace-write") {
        $args += @("--sandbox", "workspace-write")
    } elseif ($SandboxMode -eq "bypass") {
        $args += "--dangerously-bypass-approvals-and-sandbox"
    } else {
        throw "Unsupported sandbox mode: $SandboxMode"
    }
    $args += @("--output-last-message", $LastMessagePath, "-")
    @($args)
}

function Get-NormalizedTaskspaceWhaleArgv {
    param([Parameter(Mandatory = $true)][string[]]$Argv)
    $normalized = @()
    for ($i = 0; $i -lt $Argv.Count; $i++) {
        $arg = $Argv[$i]
        if ($arg -eq "-C") {
            $normalized += $arg
            $normalized += "<repo>"
            $i++
        } elseif ($arg -eq "--output-last-message") {
            $normalized += $arg
            $normalized += "<last-message>"
            $i++
        } else {
            $normalized += $arg
        }
    }
    @($normalized)
}
