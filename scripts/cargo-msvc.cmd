@echo off
setlocal

set "VSCMD_SKIP_SENDTELEMETRY=1"
rem The newest Visual Studio or Build Tools with the C++ tools, whatever its version folder.
set "VSDEVCMD="
for /f "usebackq delims=" %%I in (`"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath`) do set "VSDEVCMD=%%I\Common7\Tools\VsDevCmd.bat"

if not exist "%VSDEVCMD%" (
    echo error: Visual Studio Build Tools developer command script not found at "%VSDEVCMD%"
    exit /b 1
)

call "%VSDEVCMD%" -arch=x64 -host_arch=x64 >nul
if errorlevel 1 exit /b %errorlevel%

pushd "%~dp0..\rust" >nul
if errorlevel 1 (
    echo error: could not enter rust workspace
    exit /b 1
)

cargo %*
set "EXITCODE=%ERRORLEVEL%"

popd >nul
exit /b %EXITCODE%
