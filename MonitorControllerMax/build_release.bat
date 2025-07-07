@echo off
echo Starting MonitorControllerMax Release Build...
cd /d "C:\REAPER\Effects\Masking Effects\MonitorControllerMax\Builds\VisualStudio2022"

echo Cleaning previous build...
"C:\Program Files\Microsoft Visual Studio\2022\Community\MSBuild\Current\Bin\MSBuild.exe" MonitorControllerMax.sln /p:Configuration=Release /p:Platform=x64 /t:Clean

echo Building Release configuration...
"C:\Program Files\Microsoft Visual Studio\2022\Community\MSBuild\Current\Bin\MSBuild.exe" MonitorControllerMax.sln /p:Configuration=Release /p:Platform=x64 /m /v:detailed > build_release_log.txt 2>&1

echo Build completed. Check build_release_log.txt for details.
type build_release_log.txt | findstr /i "error\|failed\|successful"
pause