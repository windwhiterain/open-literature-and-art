$repo = "windwhiterain/open-literature-and-art"
$base = "https://github.com/${repo}/releases/latest/download"
$binary = "soil-windows-x86_64.exe"

$installDir = "$env:LOCALAPPDATA\soil"
New-Item -ItemType Directory -Force $installDir | Out-Null

Write-Host "正在下载 soil (${binary})..."
Invoke-WebRequest -Uri "${base}/${binary}" -OutFile "${installDir}\soil.exe"

# Add to user PATH permanently
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*${installDir}*") {
    [Environment]::SetEnvironmentVariable("Path", "${userPath};${installDir}", "User")
    Write-Host "已将 ${installDir} 追加到用户 PATH"
}

# Refresh current session PATH
$env:Path = [Environment]::GetEnvironmentVariable("Path", "User")

Write-Host ""
Write-Host "soil 安装完成！"
Write-Host "重新打开终端后运行: soil"
