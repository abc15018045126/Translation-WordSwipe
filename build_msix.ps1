param(
    [string]$PackageIdentityName = "",
    [string]$Publisher = "",
    [string]$PublisherDisplayName = "",
    [switch]$SelfSign = $false
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host " Translation WordSwipe - MSIX Packager  " -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# 1. 查找 Windows SDK 工具 makeappx.exe
Write-Host "[1/5] 正在定位 Windows SDK makeappx.exe 工具..." -ForegroundColor Yellow
$sdkPaths = @(
    "C:\Program Files (x86)\Windows Kits\10\bin\10.0.26100.0\x64\makeappx.exe",
    "C:\Program Files (x86)\Windows Kits\10\bin\10.0.22621.0\x64\makeappx.exe",
    "C:\Program Files (x86)\Windows Kits\10\bin\10.0.19041.0\x64\makeappx.exe"
)

$makeappx = $null
foreach ($p in $sdkPaths) {
    if (Test-Path $p) {
        $makeappx = $p
        break
    }
}

if (-not $makeappx) {
    $found = Get-ChildItem -Path "C:\Program Files (x86)\Windows Kits\10\bin" -Filter "makeappx.exe" -Recurse -ErrorAction SilentlyContinue | Where-Object { $_.FullName -like "*x64*" } | Select-Object -First 1
    if ($found) {
        $makeappx = $found.FullName
    }
}

if (-not $makeappx) {
    Write-Error "未找到 Windows SDK makeappx.exe，请确保已安装 Windows 10/11 SDK。"
    exit 1
}
Write-Host "已定位 makeappx: $makeappx" -ForegroundColor Green

# 2. 生成图标与资产
Write-Host "[2/5] 检查并生成图标素材与 MSIX 资产..." -ForegroundColor Yellow
if (-not (Test-Path "msix/Assets/StoreLogo.png")) {
    python scripts/create_logo_assets.py
}
Write-Host "图标资产准备就绪。" -ForegroundColor Green

# 3. 编译 Rust Release 二进制
Write-Host "[3/5] 正在以 Release 模式编译 TranslationWordSwipe..." -ForegroundColor Yellow
cargo build --release --bin TranslationWordSwipe
$binPath = "target/release/TranslationWordSwipe.exe"
if (-not (Test-Path $binPath)) {
    Write-Error "编译产物未找到: $binPath"
    exit 1
}
Write-Host "编译成功: $binPath" -ForegroundColor Green

# 4. 组装 MSIX 打包暂存目录
Write-Host "[4/5] 正在组装暂存文件 (Staging)..." -ForegroundColor Yellow
$stagingDir = "target/msix_staging"
if (Test-Path $stagingDir) {
    Remove-Item -Recurse -Force $stagingDir
}
New-Item -ItemType Directory -Path $stagingDir | Out-Null
New-Item -ItemType Directory -Path "$stagingDir/Assets" | Out-Null

Copy-Item $binPath -Destination "$stagingDir/TranslationWordSwipe.exe"
Copy-Item "msix/Assets/*" -Destination "$stagingDir/Assets/" -Recurse

# 处理 AppxManifest.xml（若传入了开发者信息则替换）
$manifestContent = Get-Content "msix/AppxManifest.xml" -Raw -Encoding utf8
if ($PackageIdentityName) {
    $manifestContent = $manifestContent -replace 'Name="TranslationWordSwipe"', "Name=`"$PackageIdentityName`""
}
if ($Publisher) {
    $manifestContent = $manifestContent -replace 'Publisher="CN=Developer"', "Publisher=`"$Publisher`""
}
if ($PublisherDisplayName) {
    $manifestContent = $manifestContent -replace '<PublisherDisplayName>Developer</PublisherDisplayName>', "<PublisherDisplayName>$PublisherDisplayName</PublisherDisplayName>"
}
Set-Content -Path "$stagingDir/AppxManifest.xml" -Value $manifestContent -Encoding utf8
Write-Host "暂存目录组装完毕: $stagingDir" -ForegroundColor Green

# 5. 调用 makeappx 打包
Write-Host "[5/5] 正在生成 MSIX 安装包..." -ForegroundColor Yellow
$outputMsix = "target/TranslationWordSwipe.msix"
if (Test-Path $outputMsix) {
    Remove-Item -Force $outputMsix
}

& $makeappx pack /d $stagingDir /p $outputMsix /o
if ($LASTEXITCODE -ne 0) {
    Write-Error "makeappx 打包失败，退出码: $LASTEXITCODE"
    exit $LASTEXITCODE
}

Write-Host "========================================" -ForegroundColor Green
Write-Host " 打包成功! 生成的 MSIX 安装包位于:" -ForegroundColor Green
Write-Host " $outputMsix" -ForegroundColor White
Write-Host "========================================" -ForegroundColor Green
Write-Host "提示：" -ForegroundColor Cyan
Write-Host "1. 提交到微软商店（Partner Center）时，直接上传该 MSIX 文件即可。" -ForegroundColor Gray
Write-Host "   商店会自动使用微软官方受信任证书对该包进行签名。" -ForegroundColor Gray
Write-Host "2. 若在本地直接双击测试安装，请参考本地自签名测试步骤。" -ForegroundColor Gray
