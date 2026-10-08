# Construit les fichiers de release de Dualink dans dist\ :
#   Dualink-Setup-v<version>.exe          installateur (Inno Setup) : vérifie et installe ViGEmBus / HidHide
#   Dualink-v<version>-windows-x64.exe    l'application seule (dualink.exe), sans installateur
#   SHA256SUMS.txt
#
# Les installateurs ViGEmBus et HidHide sont téléchargés depuis les dépôts officiels et vérifiés par
# leur SHA256 (empreintes de leurs manifestes winget). Rien d'autre n'est modifié sur la machine.
#
# Utilisation : powershell -ExecutionPolicy Bypass -File scripts\package-release.ps1 [-Version <x.y.z>]
#   (par défaut : la version de Cargo.toml). Si Inno Setup (ISCC.exe) est absent, seule l'application seule est construite.

param([string]$Version = '')

$ErrorActionPreference = 'Stop'
$root  = Split-Path $PSScriptRoot -Parent
if (-not $Version) { $Version = (Select-String -Path (Join-Path $root 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"').Matches[0].Groups[1].Value }
$dist  = Join-Path $root 'dist'
$cache = Join-Path $dist 'cache'
$name  = "Dualink-v$Version-windows-x64"
$stage = Join-Path $dist "stage-v$Version"

$components = @(
    @{
        Name    = 'ViGEmBus'
        File    = 'ViGEmBus_1.22.0_x64_x86_arm64.exe'
        Url     = 'https://github.com/nefarius/ViGEmBus/releases/download/v1.22.0/ViGEmBus_1.22.0_x64_x86_arm64.exe'
        Sha256  = '89220a7865076b342892f98865f3499fb7c4cfd673159e89d352c360fd014c6a'
        License = 'BSD-3-Clause'
        LicenseUrl = 'https://raw.githubusercontent.com/nefarius/ViGEmBus/master/LICENSE'
        Source  = 'https://github.com/nefarius/ViGEmBus'
    },
    @{
        Name    = 'HidHide'
        File    = 'HidHide_1.5.230_x64.exe'
        Url     = 'https://github.com/nefarius/HidHide/releases/download/v1.5.230.0/HidHide_1.5.230_x64.exe'
        Sha256  = 'f4bbbcb82e6258641b887c74bc81c4c5f66e4aa811808dfc304347687b7605f6'
        License = 'MIT'
        LicenseUrl = 'https://raw.githubusercontent.com/nefarius/HidHide/master/LICENSE'
        Source  = 'https://github.com/nefarius/HidHide'
    }
)

New-Item -ItemType Directory -Force $cache | Out-Null
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
New-Item -ItemType Directory -Force (Join-Path $stage 'redist') | Out-Null

Write-Host '== Compilation de Dualink (release)'
Push-Location $root
try { cargo build --release; if ($LASTEXITCODE -ne 0) { throw 'cargo build a échoué' } } finally { Pop-Location }
Copy-Item (Join-Path $root 'target\release\dualink.exe') $stage

$notices = @(
    'Dualink embarque les installateurs de composants tiers ci-dessous, redistribués sans modification.',
    'Leurs textes de licence sont reproduits ici.',
    ''
)

foreach ($c in $components) {
    $cached = Join-Path $cache $c.File
    if (-not (Test-Path $cached)) {
        Write-Host "== Téléchargement de $($c.File)"
        Invoke-WebRequest -Uri $c.Url -OutFile $cached -UseBasicParsing
    }
    $hash = (Get-FileHash $cached -Algorithm SHA256).Hash.ToLower()
    if ($hash -ne $c.Sha256) {
        Remove-Item $cached -Force
        throw "Empreinte inattendue pour $($c.File) : $hash (attendu $($c.Sha256))"
    }
    Write-Host "   $($c.File) : SHA256 vérifié"
    Copy-Item $cached (Join-Path $stage 'redist')

    $license = (Invoke-WebRequest -Uri $c.LicenseUrl -UseBasicParsing).Content
    $notices += ('=' * 78)
    $notices += "$($c.Name) ($($c.License))  -  $($c.Source)"
    $notices += "Fichier : redist\$($c.File)  (SHA256 $($c.Sha256))"
    $notices += ('=' * 78)
    $notices += $license
    $notices += ''
}
$notices | Set-Content -Encoding UTF8 (Join-Path $stage 'THIRD_PARTY_NOTICES.txt')

Write-Host '== Application seule'
$portable = Join-Path $dist "$name.exe"
Copy-Item (Join-Path $stage 'dualink.exe') $portable -Force

# Installateur : Inno Setup (ISCC.exe), dans le PATH ou à un emplacement d'installation habituel.
$iscc = (Get-Command iscc -ErrorAction SilentlyContinue).Source
if (-not $iscc) {
    foreach ($candidate in @(
            "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
            "$env:ProgramFiles\Inno Setup 6\ISCC.exe",
            "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe")) {
        if (Test-Path $candidate) { $iscc = $candidate; break }
    }
}

$setup = Join-Path $dist "Dualink-Setup-v$Version.exe"
if (Test-Path $setup) { Remove-Item -Force $setup }
if ($iscc) {
    Write-Host '== Installateur (Inno Setup)'
    & $iscc "/DAppVersion=$Version" "/DStageDir=$stage" "/DViGEmFile=$($components[0].File)" "/DHidHideFile=$($components[1].File)" (Join-Path $root 'installer\dualink.iss')
    if ($LASTEXITCODE -ne 0) { throw "ISCC a échoué (code $LASTEXITCODE)" }
} else {
    Write-Warning 'Inno Setup (ISCC.exe) introuvable : installateur non construit, seule l''application seule est produite.'
}

$sums = @()
foreach ($file in @($setup, $portable)) {
    if (Test-Path $file) { $sums += "{0}  {1}" -f (Get-FileHash $file -Algorithm SHA256).Hash.ToLower(), (Split-Path $file -Leaf) }
}
$sums | Set-Content -Encoding ASCII (Join-Path $dist 'SHA256SUMS.txt')

Write-Host ''
foreach ($file in @($setup, $portable)) {
    if (Test-Path $file) { Write-Host ("Terminé : {0} ({1:N1} Mo)" -f $file, ((Get-Item $file).Length / 1MB)) }
}
