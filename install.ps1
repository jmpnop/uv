# uv fork installer (jmpnop/uv) for Windows PowerShell.
#
# Usage:
#   powershell -ExecutionPolicy ByPass -c "irm https://github.com/jmpnop/uv/releases/latest/download/uv-installer.ps1 | iex"
#
# Options (via env vars):
#   $env:UV_INSTALL_DIR  directory to install into (default: $env:USERPROFILE\.local\bin)
#   $env:UV_VERSION      release tag to install (default: latest)

$ErrorActionPreference = "Stop"

$Repo       = "jmpnop/uv"
$InstallDir = if ($env:UV_INSTALL_DIR) { $env:UV_INSTALL_DIR } else { Join-Path $env:USERPROFILE ".local\bin" }
$Version    = if ($env:UV_VERSION)     { $env:UV_VERSION }     else { "latest" }

$arch = $env:PROCESSOR_ARCHITECTURE
switch ($arch) {
    "AMD64" { $target = "x86_64-pc-windows-msvc" }
    default { throw "Unsupported architecture: $arch (supported: AMD64)" }
}

$archive = "uv-$target.zip"
$url = if ($Version -eq "latest") {
    "https://github.com/$Repo/releases/latest/download/$archive"
} else {
    "https://github.com/$Repo/releases/download/$Version/$archive"
}

Write-Host "Downloading $archive..."
$tmp = Join-Path $env:TEMP ("uv-install-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tmp | Out-Null
try {
    $archivePath = Join-Path $tmp $archive
    Invoke-WebRequest -Uri $url -OutFile $archivePath -UseBasicParsing
    Expand-Archive -Path $archivePath -DestinationPath $tmp -Force

    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    Copy-Item -Path (Join-Path $tmp "uv-$target\uv.exe") -Destination (Join-Path $InstallDir "uv.exe") -Force
    $uvx = Join-Path $tmp "uv-$target\uvx.exe"
    if (Test-Path $uvx) {
        Copy-Item -Path $uvx -Destination (Join-Path $InstallDir "uvx.exe") -Force
    }

    Write-Host "Installed uv to $InstallDir\uv.exe"

    $path = [Environment]::GetEnvironmentVariable("Path", "User")
    if (-not ($path -split ";" | Where-Object { $_ -eq $InstallDir })) {
        Write-Host "Note: $InstallDir is not on your user PATH."
        Write-Host "      Add it via: [Environment]::SetEnvironmentVariable('Path', [Environment]::GetEnvironmentVariable('Path','User') + ';$InstallDir', 'User')"
    }
} finally {
    Remove-Item -Path $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
