param(
  [string]$InstallDir = "$env:LOCALAPPDATA\soil"
)

$ErrorActionPreference = "Stop"

$Repo = "windwhiterain/open-literature-and-art"
$Tag  = "nightly"
$Binary = "soil"

function Write-Color($Text, $Color) {
  Write-Host $Text -ForegroundColor $Color
}

$arch = if ([Environment]::Is64BitOperatingSystem) { "x86_64" } else { "x86" }
$asset = "${Binary}-windows-${arch}.exe"

Write-Color "==> Installing ${Binary} for Windows ${arch}..." Green

$url = "https://github.com/${Repo}/releases/download/${Tag}/${asset}"
Write-Host "    Downloading ${url}"

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

$dest = Join-Path $InstallDir "${Binary}.exe"

Invoke-WebRequest -Uri $url -OutFile $dest -UseBasicParsing

Write-Color "==> Installed ${Binary} to ${dest}" Green

$currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if (-not $currentPath) { $currentPath = "" }
if ($currentPath -split ";" -notcontains $InstallDir) {
  [Environment]::SetEnvironmentVariable("Path", "${currentPath};${InstallDir}", "User")
  $env:Path = "${env:Path};${InstallDir}"
  Write-Color "==> Added ${InstallDir} to user PATH." Green
}

Write-Color "==> Run '${Binary} --help' to get started." Green
