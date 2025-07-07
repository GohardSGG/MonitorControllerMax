@echo off
title MonitorControllerMax - Claude Development Assistant
color 0A

echo.
echo ========================================================
echo    MonitorControllerMax - Claude Development Assistant
echo ========================================================
echo.

:MENU
echo 请选择操作：
echo.
echo [1] 清理并重新编译 Debug 版本
echo [2] 清理并重新编译 Release 版本  
echo [3] 快速编译 Debug 版本
echo [4] 快速编译 Release 版本
echo [5] 运行独立应用 (Debug)
echo [6] 安装 VST3 插件到系统
echo [7] 查看编译日志
echo [8] 清理所有临时文件
echo [9] 退出
echo.
set /p choice="请输入选项 (1-9): "

if "%choice%"=="1" goto CLEAN_BUILD_DEBUG
if "%choice%"=="2" goto CLEAN_BUILD_RELEASE
if "%choice%"=="3" goto QUICK_BUILD_DEBUG
if "%choice%"=="4" goto QUICK_BUILD_RELEASE
if "%choice%"=="5" goto RUN_STANDALONE
if "%choice%"=="6" goto INSTALL_VST3
if "%choice%"=="7" goto VIEW_LOGS
if "%choice%"=="8" goto CLEAN_ALL
if "%choice%"=="9" goto EXIT

echo 无效选项，请重新选择。
pause
goto MENU

:CLEAN_BUILD_DEBUG
echo.
echo [DEBUG 清理编译] 正在进行...
cd /d "C:\REAPER\Effects\Masking Effects\MonitorControllerMax\Builds\VisualStudio2022"
"C:\Program Files\Microsoft Visual Studio\2022\Community\MSBuild\Current\Bin\amd64\MSBuild.exe" MonitorControllerMax.sln /p:Configuration=Debug /p:Platform=x64 /t:Clean
"C:\Program Files\Microsoft Visual Studio\2022\Community\MSBuild\Current\Bin\amd64\MSBuild.exe" MonitorControllerMax.sln /p:Configuration=Debug /p:Platform=x64 /m /v:minimal > debug_build_log.txt 2>&1
if %ERRORLEVEL% equ 0 (
    echo ✅ Debug 编译成功！
) else (
    echo ❌ Debug 编译失败，请查看 debug_build_log.txt
)
pause
goto MENU

:CLEAN_BUILD_RELEASE
echo.
echo [RELEASE 清理编译] 正在进行...
cd /d "C:\REAPER\Effects\Masking Effects\MonitorControllerMax\Builds\VisualStudio2022"
"C:\Program Files\Microsoft Visual Studio\2022\Community\MSBuild\Current\Bin\amd64\MSBuild.exe" MonitorControllerMax.sln /p:Configuration=Release /p:Platform=x64 /t:Clean
"C:\Program Files\Microsoft Visual Studio\2022\Community\MSBuild\Current\Bin\amd64\MSBuild.exe" MonitorControllerMax.sln /p:Configuration=Release /p:Platform=x64 /m /v:minimal > release_build_log.txt 2>&1
if %ERRORLEVEL% equ 0 (
    echo ✅ Release 编译成功！
) else (
    echo ❌ Release 编译失败，请查看 release_build_log.txt
)
pause
goto MENU

:QUICK_BUILD_DEBUG
echo.
echo [DEBUG 快速编译] 正在进行...
cd /d "C:\REAPER\Effects\Masking Effects\MonitorControllerMax\Builds\VisualStudio2022"
"C:\Program Files\Microsoft Visual Studio\2022\Community\MSBuild\Current\Bin\amd64\MSBuild.exe" MonitorControllerMax.sln /p:Configuration=Debug /p:Platform=x64 /m /v:minimal > debug_quick_log.txt 2>&1
if %ERRORLEVEL% equ 0 (
    echo ✅ Debug 快速编译成功！
) else (
    echo ❌ Debug 编译失败，请查看 debug_quick_log.txt
)
pause
goto MENU

:QUICK_BUILD_RELEASE
echo.
echo [RELEASE 快速编译] 正在进行...
cd /d "C:\REAPER\Effects\Masking Effects\MonitorControllerMax\Builds\VisualStudio2022"
"C:\Program Files\Microsoft Visual Studio\2022\Community\MSBuild\Current\Bin\amd64\MSBuild.exe" MonitorControllerMax.sln /p:Configuration=Release /p:Platform=x64 /m /v:minimal > release_quick_log.txt 2>&1
if %ERRORLEVEL% equ 0 (
    echo ✅ Release 快速编译成功！
) else (
    echo ❌ Release 编译失败，请查看 release_quick_log.txt
)
pause
goto MENU

:RUN_STANDALONE
echo.
echo [运行独立应用] 启动中...
if exist "C:\REAPER\Effects\Masking Effects\MonitorControllerMax\Builds\VisualStudio2022\x64\Debug\Standalone Plugin\MonitorControllerMax.exe" (
    start "" "C:\REAPER\Effects\Masking Effects\MonitorControllerMax\Builds\VisualStudio2022\x64\Debug\Standalone Plugin\MonitorControllerMax.exe"
    echo ✅ 独立应用已启动！
) else (
    echo ❌ 找不到独立应用，请先编译 Debug 版本
)
pause
goto MENU

:INSTALL_VST3
echo.
echo [安装 VST3 插件] 正在安装到系统...
set VST3_PATH=%COMMONPROGRAMFILES%\VST3
if exist "C:\REAPER\Effects\Masking Effects\MonitorControllerMax\Builds\VisualStudio2022\x64\Debug\VST3\MonitorControllerMax.vst3" (
    if not exist "%VST3_PATH%" mkdir "%VST3_PATH%"
    xcopy /E /I /Y "C:\REAPER\Effects\Masking Effects\MonitorControllerMax\Builds\VisualStudio2022\x64\Debug\VST3\MonitorControllerMax.vst3" "%VST3_PATH%\MonitorControllerMax.vst3"
    echo ✅ VST3 插件已安装到：%VST3_PATH%
) else (
    echo ❌ 找不到 VST3 插件，请先编译 Debug 版本
)
pause
goto MENU

:VIEW_LOGS
echo.
echo [编译日志] 可用的日志文件：
cd /d "C:\REAPER\Effects\Masking Effects\MonitorControllerMax\Builds\VisualStudio2022"
dir *.txt /B 2>nul
echo.
set /p logfile="请输入要查看的日志文件名（或按回车返回菜单）: "
if "%logfile%"=="" goto MENU
if exist "%logfile%" (
    type "%logfile%"
) else (
    echo 文件不存在！
)
pause
goto MENU

:CLEAN_ALL
echo.
echo [清理临时文件] 正在清理...
cd /d "C:\REAPER\Effects\Masking Effects\MonitorControllerMax\Builds\VisualStudio2022"
del /Q *.txt 2>nul
rmdir /S /Q x64 2>nul
echo ✅ 清理完成！
pause
goto MENU

:EXIT
echo.
echo 感谢使用 Claude Development Assistant！
echo 如遇到任何编译问题，请将日志文件发送给我进行分析。
pause
exit
