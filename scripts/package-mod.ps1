<#
.SYNOPSIS
    Build the Timberbot C# mod against a local Timberborn 1.1 Managed\ directory
    and package ONLY Timberbot's own GPL-3.0 artifacts into dist\timberbot-mod-<version>.zip.

.DESCRIPTION
    Compiles timberbot\src\Timberbot.csproj in Release mode with -p:GameManagedDir=...
    and creates a clean ZIP containing a top-level Timberbot\ folder ready to extract
    directly into Timberborn's Mods\ folder or upload to a GitHub Release.
    Explicitly excludes all Timberborn, Unity, Bindito, and other Managed\ game DLLs.

.EXAMPLE
    .\scripts\package-mod.ps1 -GameManagedDir "C:\Program Files (x86)\Steam\steamapps\common\Timberborn\Timberborn_Data\Managed"

.EXAMPLE
    .\scripts\package-mod.ps1 -GameManagedDir "C:\Games\Steam\steamapps\common\Timberborn\Timberborn_Data\Managed" -OutDir dist
#>

[CmdletBinding()]
param(
    [string]$GameManagedDir = $env:GAME_MANAGED_DIR,
    [string]$OutDir = "",
    [string]$Version = "",
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$SrcDir   = Join-Path $RepoRoot "timberbot\src"
$Csproj   = Join-Path $SrcDir "Timberbot.csproj"
$Manifest = Join-Path $SrcDir "manifest.json"

if (-not (Test-Path $Csproj) -or -not (Test-Path $Manifest)) {
    throw "package-mod: missing $Csproj or $Manifest"
}

if ([string]::IsNullOrWhiteSpace($OutDir)) {
    $OutDir = Join-Path $RepoRoot "dist"
}

# Auto-detect common Windows Steam Timberborn Managed\ locations when omitted
if (-not $SkipBuild -and [string]::IsNullOrWhiteSpace($GameManagedDir)) {
    $Candidates = @(
        "C:\Program Files (x86)\Steam\steamapps\common\Timberborn\Timberborn_Data\Managed",
        "C:\Games\Steam\steamapps\common\Timberborn\Timberborn_Data\Managed",
        "D:\SteamLibrary\steamapps\common\Timberborn\Timberborn_Data\Managed"
    )
    foreach ($cand in $Candidates) {
        if (Test-Path $cand) {
            $GameManagedDir = $cand
            break
        }
    }
}

if (-not $SkipBuild) {
    if ([string]::IsNullOrWhiteSpace($GameManagedDir) -or -not (Test-Path $GameManagedDir)) {
        throw "package-mod: Timberborn Managed\ directory not found. Pass -GameManagedDir `"C:\path\to\Timberborn_Data\Managed`"."
    }
    $SentinelDll = Join-Path $GameManagedDir "Timberborn.Common.dll"
    if (-not (Test-Path $SentinelDll)) {
        throw "package-mod: $GameManagedDir does not contain Timberborn.Common.dll"
    }
}

if ([string]::IsNullOrWhiteSpace($Version)) {
    $ManifestJson = Get-Content -Raw -Path $Manifest | ConvertFrom-Json
    $Version = [string]$ManifestJson.Version
}
if ([string]::IsNullOrWhiteSpace($Version)) {
    throw "package-mod: failed to read Version from $Manifest"
}

$WorkDir = Join-Path ([System.IO.Path]::GetTempPath()) ("timberbot-pkg-" + [System.Guid]::NewGuid().ToString("N"))
$BuildDeployStage = Join-Path $WorkDir "msbuild-deploy"
$ZipStage = Join-Path $WorkDir "package"
$ModStage = Join-Path $ZipStage "Timberbot"
$DocsStage = Join-Path $ModStage "docs"

try {
    New-Item -ItemType Directory -Force -Path $BuildDeployStage, $DocsStage, $OutDir | Out-Null

    if (-not $SkipBuild) {
        Write-Host "package-mod: building Release against GameManagedDir=$GameManagedDir"
        & dotnet build $Csproj `
            -c Release `
            "-p:GameManagedDir=$GameManagedDir" `
            "-p:ModDir=$BuildDeployStage"
        if ($LASTEXITCODE -ne 0) {
            throw "dotnet build failed with exit code $LASTEXITCODE"
        }
    }

    $ModDll = Join-Path $SrcDir "bin\Release\netstandard2.1\Timberbot.dll"
    if (-not (Test-Path $ModDll)) {
        throw "package-mod: expected built DLL at $ModDll"
    }

    # Explicit GPL-3.0 allowlist: copy ONLY Timberbot's own artifacts (never Managed\ game DLLs)
    Copy-Item -Path $ModDll -Destination (Join-Path $ModStage "Timberbot.dll")
    Copy-Item -Path $Manifest -Destination (Join-Path $ModStage "manifest.json")
    Copy-Item -Path (Join-Path $SrcDir "settings.json") -Destination (Join-Path $ModStage "settings.json")

    $Thumb = Join-Path $SrcDir "thumbnail.png"
    if (Test-Path $Thumb) {
        Copy-Item -Path $Thumb -Destination (Join-Path $ModStage "thumbnail.png")
    }
    $License = Join-Path $RepoRoot "LICENSE"
    if (Test-Path $License) {
        Copy-Item -Path $License -Destination (Join-Path $ModStage "LICENSE")
    }

    $DocFiles = @(
        "api-reference.md",
        "connect-and-play.md",
        "developing.md",
        "events.md",
        "features.md",
        "getting-started.md",
        "timberbot.md",
        "websocket-protocol.md"
    )
    foreach ($doc in $DocFiles) {
        $srcDoc = Join-Path $RepoRoot (Join-Path "docs" $doc)
        if (Test-Path $srcDoc) {
            Copy-Item -Path $srcDoc -Destination (Join-Path $DocsStage $doc)
        }
    }

    # Explicit safety audit: refuse to package if any DLL other than Timberbot.dll is present
    $StagedDlls = @(Get-ChildItem -Path $ZipStage -Recurse -File -Filter "*.dll")
    if ($StagedDlls.Count -ne 1 -or $StagedDlls[0].Name -ne "Timberbot.dll") {
        $Names = ($StagedDlls | ForEach-Object { $_.FullName }) -join "`n  "
        throw "package-mod: SAFETY CHECK FAILED — expected ONLY Timberbot.dll, found:`n  $Names"
    }

    $ZipPath = Join-Path (Resolve-Path $OutDir).Path "timberbot-mod-$Version.zip"
    if (Test-Path $ZipPath) {
        Remove-Item -Force $ZipPath
    }

    Compress-Archive -Path $ModStage -DestinationPath $ZipPath -CompressionLevel Optimal

    $Hash = (Get-FileHash -Path $ZipPath -Algorithm SHA256).Hash.ToLowerInvariant()
    Write-Host "package-mod: created $ZipPath"
    Write-Host "package-mod: sha256  $Hash"
    Write-Host "package-mod: verified ONLY Timberbot.dll included (zero game Managed\ DLLs)."
    Write-Host ""
    Write-Host "Next step (optional GitHub Release upload):"
    Write-Host "  gh release create v$Version `"$ZipPath`" --title `"Timberbot Mod v$Version`" --notes `"GPL-3.0 Timberbot C# mod built against Timberborn 1.1 Managed DLLs (excludes all game DLLs).`""
}
finally {
    if (Test-Path $WorkDir) {
        Remove-Item -Recurse -Force $WorkDir
    }
}
