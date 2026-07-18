param(
  [string]$InstallDir = "$env:LOCALAPPDATA\soil",
  [switch]$Uninstall
)

$ErrorActionPreference = "Stop"

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12

$Repo = "windwhiterain/open-literature-and-art"
$Tag  = "nightly"
$Binary = "soil"

function Write-Color($Text, $Color) {
  Write-Host $Text -ForegroundColor $Color
}

if ($Uninstall) {
  Write-Color "==> Uninstalling ${Binary}..." Green

  if (Test-Path $InstallDir) {
    Remove-Item -Recurse -Force $InstallDir
    Write-Color "==> Removed ${InstallDir}" Green
  } else {
    Write-Color "==> ${InstallDir} not found, nothing to remove." Yellow
  }

  $currentPath = [Environment]::GetEnvironmentVariable("Path", "User")
  if ($currentPath -and ($currentPath -split ";" -contains $InstallDir)) {
    $newPath = ($currentPath -split ";" | Where-Object { $_ -ne $InstallDir }) -join ";"
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Color "==> Removed ${InstallDir} from user PATH." Green
  }

  Write-Color "==> Uninstall complete." Green
  return
}

Write-Color "==> Installing ${Binary} for Windows..." Green
$asset = "${Binary}.exe"

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
