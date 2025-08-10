@echo off
REM Windows batch file wrapper for th (Teleport Helper) Rust implementation
REM This provides cross-platform support and matches the behavior of th_wrapper.sh

setlocal EnableExtensions EnableDelayedExpansion

REM Get the directory where this batch file is located
set "TH_DIR=%~dp0"

REM Build the Rust binary if it doesn't exist or if source is newer
if not exist "%TH_DIR%target\release\th.exe" (
    echo Building th...
    cd /d "%TH_DIR%"
    cargo build --release
    if errorlevel 1 (
        echo Failed to build th
        exit /b 1
    )
)

REM Check if source files are newer than binary
for %%F in ("%TH_DIR%src\*.rs") do (
    if "%%F" newer than "%TH_DIR%target\release\th.exe" (
        echo Source files updated, rebuilding...
        cd /d "%TH_DIR%"
        cargo build --release
        if errorlevel 1 (
            echo Failed to build th
            exit /b 1
        )
        goto :run_binary
    )
)

:run_binary
REM Create temporary files for credential handling (Windows equivalent)
set "TEMP_FILE=%TEMP%\th_aws_creds_%RANDOM%.bat"

REM Run the Rust binary and capture AWS credential output
"%TH_DIR%target\release\th.exe" %* > "%TEMP_FILE%" 2>&1

REM Check if the output contains AWS credential exports
findstr /C:"export AWS_" "%TEMP_FILE%" >nul 2>&1
if %errorlevel% equ 0 (
    REM Convert Unix export commands to Windows set commands
    for /f "tokens=*" %%i in ('findstr /C:"export AWS_" "%TEMP_FILE%"') do (
        set "line=%%i"
        REM Convert "export VAR=value" to "set VAR=value"
        set "line=!line:export =set !"
        REM Execute the set command
        !line!
    )
    echo AWS credentials have been set in the current session.
) else (
    REM No AWS credentials, just display the output
    type "%TEMP_FILE%"
)

REM Get the exit code from the Rust binary
set "EXIT_CODE=%errorlevel%"

REM Clean up temporary file
if exist "%TEMP_FILE%" del "%TEMP_FILE%"

REM Preserve exit code
exit /b %EXIT_CODE%