# PowerShell installation script for th (Teleport Helper)
param(
    [string]$InstallPath = "$env:LOCALAPPDATA\th"
)

# Colors for output
$Red = "Red"
$Green = "Green" 
$Yellow = "Yellow"
$Blue = "Blue"

# GitHub repository
$Repo = "nikomain/th-rust"
$GitHubUrl = "https://github.com/$Repo"

Write-Host "🚀 Installing Teleport Helper (th)..." -ForegroundColor $Blue
Write-Host ""

# Detect architecture
$Arch = $env:PROCESSOR_ARCHITECTURE
switch ($Arch) {
    "AMD64" { $Arch = "x86_64" }
    "ARM64" { $Arch = "aarch64" }
    default {
        Write-Host "❌ Unsupported architecture: $Arch" -ForegroundColor $Red
        exit 1
    }
}

$BinaryFile = "th-$Arch-pc-windows-msvc.exe"
$BinaryName = "th.exe"

Write-Host "📋 Detected platform: windows-$Arch" -ForegroundColor $Blue
Write-Host "📦 Binary: $BinaryFile" -ForegroundColor $Blue
Write-Host ""

# Get latest release info
Write-Host "🔍 Fetching latest release..." -ForegroundColor $Blue
try {
    $Response = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    $TagName = $Response.tag_name
    Write-Host "✅ Latest version: $TagName" -ForegroundColor $Green
} catch {
    Write-Host "❌ Failed to get latest release information" -ForegroundColor $Red
    Write-Host "Error: $($_.Exception.Message)" -ForegroundColor $Red
    exit 1
}

# Download URL
$DownloadUrl = "https://github.com/$Repo/releases/download/$TagName/$BinaryFile"

# Create install directory
Write-Host "📁 Creating install directory: $InstallPath" -ForegroundColor $Blue
New-Item -ItemType Directory -Force -Path $InstallPath | Out-Null

# Download binary
$TempFile = "$env:TEMP\$BinaryName"
Write-Host "⬇️  Downloading $BinaryFile..." -ForegroundColor $Blue

try {
    Invoke-WebRequest -Uri $DownloadUrl -OutFile $TempFile
} catch {
    Write-Host "❌ Failed to download binary" -ForegroundColor $Red
    Write-Host "   URL: $DownloadUrl" -ForegroundColor $Red
    Write-Host "   Error: $($_.Exception.Message)" -ForegroundColor $Red
    exit 1
}

# Install binary
$InstallFile = "$InstallPath\$BinaryName"
Write-Host "📦 Installing to $InstallFile..." -ForegroundColor $Blue
Move-Item -Path $TempFile -Destination $InstallFile -Force

# Create wrapper batch file
$WrapperFile = "$InstallPath\th.bat"
Write-Host "📝 Creating wrapper script at $WrapperFile..." -ForegroundColor $Blue

@"
@echo off
setlocal

REM Run the actual th binary
"$InstallFile" %*
set TH_EXIT_CODE=%ERRORLEVEL%

REM Source any credential files that were created (Windows equivalent)
for %%f in ("%TEMP%\yl_*" "%TEMP%\admin_*") do (
    if exist "%%f" (
        call "%%f"
        goto :found_creds
    )
)
:found_creds

endlocal & (
    REM Preserve environment variables that were set
    for /f "delims=" %%i in ('type "%TEMP%\yl_*" 2^>nul ^| findstr /i "set "') do %%i 2>nul
    for /f "delims=" %%i in ('type "%TEMP%\admin_*" 2^>nul ^| findstr /i "set "') do %%i 2>nul
)

exit /b %TH_EXIT_CODE%
"@ | Out-File -FilePath $WrapperFile -Encoding ASCII

# Add to PATH
$CurrentPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($CurrentPath -notlike "*$InstallPath*") {
    Write-Host "🔧 Adding $InstallPath to user PATH..." -ForegroundColor $Blue
    $NewPath = "$CurrentPath;$InstallPath"
    [Environment]::SetEnvironmentVariable("Path", $NewPath, "User")
    
    # Update current session PATH
    $env:Path = "$env:Path;$InstallPath"
} else {
    Write-Host "✅ $InstallPath already in PATH" -ForegroundColor $Green
}

Write-Host ""
Write-Host "✅ Installation completed successfully!" -ForegroundColor $Green
Write-Host ""
Write-Host "📋 Usage:" -ForegroundColor $Blue
Write-Host "  th                 - Show help" -ForegroundColor $Yellow
Write-Host "  th a               - AWS login" -ForegroundColor $Yellow
Write-Host "  th k               - Kubernetes login" -ForegroundColor $Yellow
Write-Host "  th d               - Database login" -ForegroundColor $Yellow
Write-Host "  th update          - Update to latest version" -ForegroundColor $Yellow
Write-Host ""
Write-Host "🚀 Quick start:" -ForegroundColor $Blue
Write-Host "th" -ForegroundColor $Yellow
Write-Host ""

# Check if binary works
try {
    $null = Get-Command th -ErrorAction Stop
    Write-Host "🎉 th is now available in your PATH!" -ForegroundColor $Green
} catch {
    Write-Host "⚠️  You may need to restart your terminal or PowerShell session" -ForegroundColor $Yellow
}

Write-Host ""
Write-Host "📝 Note: Restart your terminal/PowerShell to ensure PATH changes take effect" -ForegroundColor $Blue