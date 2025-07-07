@echo off
echo Starting MonitorControllerMax Debug Build...
cd /d "C:\REAPER\Effects\Masking Effects\MonitorControllerMax\Builds\VisualStudio2022"

echo Cleaning previous build...
"C:\Program Files\Microsoft Visual Studio\2022\Community\MSBuild\Current\Bin\MSBuild.exe" MonitorControllerMax.sln /p:Configuration=Debug /p:Platform=x64 /t:Clean

echo Building Debug configuration...
"C:\Program Files\Microsoft Visual Studio\2022\Community\MSBuild\Current\Bin\MSBuild.exe" MonitorControllerMax.sln /p:Configuration=Debug /p:Platform=x64 /m /v:detailed > build_log.txt 2>&1

echo Build completed. Check build_log.txt for details.
type build_log.txt | findstr /i "error\|failed\|successful"
pause