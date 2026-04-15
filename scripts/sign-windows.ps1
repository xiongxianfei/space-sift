param(
  [Parameter(Mandatory = $true)]
  [string]$BinaryPath
)

$ErrorActionPreference = "Stop"

function Resolve-SignToolPath {
  $signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($signtool) {
    return $signtool.Source
  }

  $sdkRoot = "C:\Program Files (x86)\Windows Kits\10\bin"
  if (-not (Test-Path $sdkRoot)) {
    throw "signtool.exe was not found and the Windows SDK bin directory is missing."
  }

  $candidate = Get-ChildItem -Path $sdkRoot -Recurse -Filter signtool.exe |
    Sort-Object FullName -Descending |
    Select-Object -First 1

  if (-not $candidate) {
    throw "signtool.exe was not found in the Windows SDK."
  }

  return $candidate.FullName
}

$codeSigningEnabled = $env:SPACE_SIFT_ENABLE_CODE_SIGNING
if ([string]::IsNullOrWhiteSpace($codeSigningEnabled) -or $codeSigningEnabled.ToLowerInvariant() -notin @("1", "true", "yes")) {
  Write-Host "Space Sift signing skipped for $BinaryPath because SPACE_SIFT_ENABLE_CODE_SIGNING is not enabled."
  exit 0
}

if ([string]::IsNullOrWhiteSpace($env:SPACE_SIFT_WINDOWS_CERTIFICATE_PATH)) {
  throw "Missing SPACE_SIFT_WINDOWS_CERTIFICATE_PATH."
}

if (-not (Test-Path $env:SPACE_SIFT_WINDOWS_CERTIFICATE_PATH)) {
  throw "Windows signing certificate file not found at $($env:SPACE_SIFT_WINDOWS_CERTIFICATE_PATH)."
}

if ([string]::IsNullOrWhiteSpace($env:SPACE_SIFT_WINDOWS_CERTIFICATE_PASSWORD)) {
  throw "Missing SPACE_SIFT_WINDOWS_CERTIFICATE_PASSWORD."
}

$timestampUrl = if ([string]::IsNullOrWhiteSpace($env:SPACE_SIFT_WINDOWS_TIMESTAMP_URL)) {
  "http://timestamp.digicert.com"
} else {
  $env:SPACE_SIFT_WINDOWS_TIMESTAMP_URL
}

$signtoolPath = Resolve-SignToolPath

& $signtoolPath sign `
  /fd SHA256 `
  /f $env:SPACE_SIFT_WINDOWS_CERTIFICATE_PATH `
  /p $env:SPACE_SIFT_WINDOWS_CERTIFICATE_PASSWORD `
  /tr $timestampUrl `
  /td SHA256 `
  $BinaryPath

if ($LASTEXITCODE -ne 0) {
  throw "signtool.exe failed while signing $BinaryPath."
}
