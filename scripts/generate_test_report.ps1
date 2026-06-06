param(
    [Parameter(Mandatory = $true)]
    [string]$Module
)

$ErrorActionPreference = "Stop"

$cargo = Join-Path $env:USERPROFILE ".cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) {
    $cargo = "cargo"
}

$crate = "rust-can-$Module"
$date = Get-Date -Format "yyyy-MM-dd"
$rustcVersion = & $cargo --version 2>&1
$llvmCovVersion = & $cargo llvm-cov --version 2>&1

# Cargo prints absolute paths in compilation lines (e.g. "Compiling
# <crate> v0.1.0 (<repo>/<crate>)"). Strip the workspace root from
# generated reports so they don't leak machine-specific paths when the
# reports are committed.
$workspaceRoot = (Get-Location).ProviderPath
function Format-CargoOutput {
    param([string]$Raw)
    if ([string]::IsNullOrEmpty($Raw)) { return $Raw }
    return ($Raw -split "`r?`n" | ForEach-Object {
        $line = $_
        if ($workspaceRoot) {
            $line = $line.Replace($workspaceRoot, "<repo>")
        }
        $line
    }) -join "`n"
}

Write-Output "# rust-can-$Module 测试报告"
Write-Output ""
Write-Output "> 设计文档：[../../design/details/$Module.md](../../design/details/$Module.md)"
Write-Output ""
Write-Output "## 测试范围与环境"
Write-Output ""
Write-Output "| 项 | 内容 |"
Write-Output "| --- | --- |"
Write-Output "| Crate | ``$crate`` |"
Write-Output "| 测试日期 | $date |"
Write-Output "| Rust 工具链 | $rustcVersion |"
Write-Output "| Coverage 工具 | $llvmCovVersion |"
Write-Output ""
Write-Output "## 单元/集成测试"
Write-Output ""
Write-Output "### 执行命令"
Write-Output ""
Write-Output '```powershell'
Write-Output "cargo test -p $crate --all-features"
Write-Output '```'
Write-Output ""
Write-Output "### 结果摘要"
Write-Output ""

$testOutput = & $cargo test -p $crate --all-features 2>&1 | Out-String
$testOutput = Format-CargoOutput $testOutput
Write-Output '```text'
Write-Output $testOutput.TrimEnd()
Write-Output '```'
Write-Output ""

Write-Output "### 覆盖率（``cargo llvm-cov``）"
Write-Output ""
Write-Output '```powershell'
Write-Output "cargo llvm-cov -p $crate --all-features --summary-only"
Write-Output '```'
Write-Output ""

$covOutput = & $cargo llvm-cov -p $crate --all-features --summary-only 2>&1 | Out-String
$covOutput = Format-CargoOutput $covOutput
Write-Output '```text'
Write-Output $covOutput.TrimEnd()
Write-Output '```'
Write-Output ""
Write-Output "> 生成于 ``scripts/generate_test_report.ps1``；请人工补全 E2E/Perf 章节后写入 ``docs/test/details/$Module.md``。"
