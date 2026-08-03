# Build the Microsoft Store MSIX from a release build (§20.4).
# Usage: pwsh packaging/msix/make-msix.ps1 [-Sign]
# Prereqs: Windows 10 SDK on PATH (makeappx, signtool), a release build:
#   pnpm tauri build --no-default-features --features channel-msstore

param([switch]$Sign)

$ErrorActionPreference = "Stop"
$root = Resolve-Path "$PSScriptRoot/../.."
$staging = "$PSScriptRoot/staging"
$exe = "$root/target/release/nameshift.exe"

if (-not (Test-Path $exe)) {
  throw "Build first: pnpm tauri build --no-default-features --features channel-msstore"
}

Remove-Item -Recurse -Force $staging -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Path "$staging/app" | Out-Null
New-Item -ItemType Directory -Path "$staging/assets" | Out-Null

Copy-Item $exe "$staging/app/nameshift.exe"
Copy-Item "$PSScriptRoot/AppxManifest.xml" "$staging/AppxManifest.xml"
Copy-Item "$root/src-tauri/icons/StoreLogo.png" "$staging/assets/StoreLogo.png"
Copy-Item "$root/src-tauri/icons/Square150x150Logo.png" "$staging/assets/Square150x150Logo.png"
Copy-Item "$root/src-tauri/icons/Square44x44Logo.png" "$staging/assets/Square44x44Logo.png"

$msix = "$PSScriptRoot/NameShift.msix"
makeappx pack /d $staging /p $msix /o
Write-Host "packed $msix"

if ($Sign) {
  # Local install only — the Store signs submissions with Microsoft's cert.
  $cert = New-SelfSignedCertificate -Type Custom -Subject "CN=NameShift Test" `
    -KeyUsage DigitalSignature -FriendlyName "NameShift Test" `
    -CertStoreLocation "Cert:\CurrentUser\My" `
    -TextExtension @("2.5.29.37={text}1.3.6.1.5.5.7.3.3")
  signtool sign /fd SHA256 /sha1 $cert.Thumbprint $msix
  Write-Host "signed with a self-signed test certificate"
}
