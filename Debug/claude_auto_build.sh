#!/bin/bash

# Claude Code 自动化编译脚本 - 直接MSBuild版本
# 专为Claude Code在WSL环境中的自动化开发设计
# 直接调用MSBuild，避免PowerShell编码问题

set -e  # 遇到错误立即退出

# 配置路径
PROJECT_ROOT="/mnt/c/REAPER/Effects/Masking Effects/MonitorControllerMax"
BUILD_DIR="$PROJECT_ROOT/Builds/VisualStudio2022"
DEBUG_DIR="/mnt/c/REAPER/Effects/Masking Effects/Debug"
POWERSHELL_TOOLKIT="C:/REAPER/Effects/Masking Effects/Debug/claude_dev_toolkit_simple.ps1"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查编译环境
check_environment() {
    log_info "检查编译环境..."
    
    # 检查MSBuild
    if [ ! -f "/mnt/c/Program Files/Microsoft Visual Studio/2022/Community/MSBuild/Current/Bin/MSBuild.exe" ]; then
        log_error "未找到MSBuild，请确保安装了Visual Studio 2022"
        exit 1
    fi
    
    if [ ! -d "$BUILD_DIR" ]; then
        log_error "未找到项目构建目录: $BUILD_DIR"
        exit 1
    fi
    
    log_success "编译环境检查通过"
}

# Debug编译 - 强制清理后重新编译（确保最新代码）
debug_build() {
    log_info "🚀 开始Debug编译 (强制清理模式)..."
    
    cd "$BUILD_DIR"
    
    # 强制清理 - 确保编译最新代码
    log_info "清理之前的构建..."
    "/mnt/c/Program Files/Microsoft Visual Studio/2022/Community/MSBuild/Current/Bin/MSBuild.exe" MonitorControllerMax.sln \
        /p:Configuration=Debug \
        /p:Platform=x64 \
        /t:Clean \
        /v:quiet > /dev/null 2>&1
    
    # 删除所有输出文件确保完全清理
    rm -rf x64/Debug/ 2>/dev/null || true
    
    # 完整编译
    log_info "执行完整编译..."
    if "/mnt/c/Program Files/Microsoft Visual Studio/2022/Community/MSBuild/Current/Bin/MSBuild.exe" MonitorControllerMax.sln \
        /p:Configuration=Debug \
        /p:Platform=x64 \
        /v:minimal > debug_build.log 2>&1; then
        
        log_success "Debug编译成功"
        check_build_outputs "Debug"
        return 0
    else
        log_error "Debug编译失败"
        echo "错误详情:"
        grep -i "error\|failed" debug_build.log | head -5
        return 1
    fi
}


# Release编译 - 强制清理后编译（确保最新代码）
release_build() {
    log_info "🏁 开始Release编译 (强制清理模式)..."
    
    cd "$BUILD_DIR"
    
    # 强制清理 - 确保编译最新代码
    log_info "清理之前的构建..."
    "/mnt/c/Program Files/Microsoft Visual Studio/2022/Community/MSBuild/Current/Bin/MSBuild.exe" MonitorControllerMax.sln \
        /p:Configuration=Release \
        /p:Platform=x64 \
        /t:Clean \
        /v:quiet > /dev/null 2>&1
    
    # 删除所有输出文件确保完全清理
    rm -rf x64/Release/ 2>/dev/null || true
    
    # 完整编译
    log_info "执行完整编译..."
    if "/mnt/c/Program Files/Microsoft Visual Studio/2022/Community/MSBuild/Current/Bin/MSBuild.exe" MonitorControllerMax.sln \
        /p:Configuration=Release \
        /p:Platform=x64 \
        /v:minimal > release_build.log 2>&1; then
        
        log_success "Release编译成功"
        check_build_outputs "Release"
        return 0
    else
        log_error "Release编译失败"
        echo "错误详情:"
        grep -i "error\|failed" release_build.log | head -5
        return 1
    fi
}

# 检查构建输出
check_build_outputs() {
    local config=$1
    local standalone_path="$BUILD_DIR/x64/$config/Standalone Plugin/MonitorControllerMax.exe"
    local vst3_path="$BUILD_DIR/x64/$config/VST3/MonitorControllerMax.vst3"
    
    if [ -f "$standalone_path" ]; then
        local size=$(stat -c%s "$standalone_path" 2>/dev/null || echo "unknown")
        log_success "独立程序: 已生成 (${size} bytes)"
    else
        log_warning "独立程序: 未生成"
    fi
    
    if [ -f "$vst3_path" ]; then
        log_success "VST3插件: 已生成"
    else
        log_warning "VST3插件: 未生成"
    fi
}

# 验证构建结果
verify_build() {
    log_info "🔍 验证构建结果..."
    
    local standalone_path="$BUILD_DIR/x64/Debug/Standalone Plugin/MonitorControllerMax.exe"
    
    if [ -f "$standalone_path" ]; then
        log_success "Debug版本构建验证通过"
        
        # 检查文件大小（太小可能表示编译有问题）
        local size=$(stat -c%s "$standalone_path" 2>/dev/null || echo "0")
        if [ "$size" -gt 1000000 ]; then  # 大于1MB
            log_success "文件大小正常: $(echo $size | numfmt --to=iec-i)B"
        else
            log_warning "文件大小异常: ${size}B - 可能编译不完整"
        fi
        
        return 0
    else
        log_error "构建验证失败: 未找到可执行文件"
        return 1
    fi
}

# 编译并运行独立程序
build_and_run() {
    log_info "🚀 编译并运行独立程序..."
    
    # 执行强制清理编译确保最新代码
    if debug_build; then
        log_info "🎯 编译成功，准备启动独立程序..."
        
        local standalone_path="$BUILD_DIR/x64/Debug/Standalone Plugin/MonitorControllerMax.exe"
        
        if [ -f "$standalone_path" ]; then
            log_success "发现独立程序，正在启动..."
            
            # 使用wslpath转换路径
            local windows_path=$(wslpath -w "$standalone_path")
            
            log_info "程序路径: $windows_path"
            
            # 启动独立程序 (在后台运行，不阻塞命令行)
            # 使用powershell.exe启动，更可靠
            powershell.exe -Command "Start-Process -FilePath '$windows_path'" 2>/dev/null
            
            log_success "独立程序已启动！"
            echo ""
            echo "🎮 测试建议:"
            echo "   - 验证Solo/Mute按钮逻辑"
            echo "   - 测试布局切换功能" 
            echo "   - 检查UI响应性能"
            echo "   - 验证音频处理效果"
            echo ""
            log_info "独立程序正在后台运行，你可以继续使用命令行"
            
        else
            log_error "编译成功但未找到独立程序文件"
            log_info "查看编译日志: $BUILD_DIR/debug_build.log"
            return 1
        fi
    else
        log_error "编译失败，无法启动独立程序"
        log_info "建议:"
        log_info "  1. 检查编译错误信息"
        log_info "  2. 查看日志: $BUILD_DIR/debug_build.log"
        log_info "  3. 检查代码语法错误"
        return 1
    fi
}

# 仅运行独立程序 (不编译)
run_standalone() {
    log_info "🎯 启动独立程序..."
    
    local standalone_path="$BUILD_DIR/x64/Debug/Standalone Plugin/MonitorControllerMax.exe"
    
    if [ -f "$standalone_path" ]; then
        log_success "发现独立程序，正在启动..."
        
        # 显示文件信息
        local size=$(stat -c%s "$standalone_path" 2>/dev/null || echo "unknown")
        local size_mb=$(echo "scale=2; $size / 1024 / 1024" | bc 2>/dev/null || echo "unknown")
        local mod_time=$(stat -c %y "$standalone_path" 2>/dev/null | cut -d. -f1)
        
        echo "  文件大小: ${size_mb}MB"
        echo "  修改时间: $mod_time"
        echo ""
        
        # 转换为Windows路径格式并启动
        local windows_path=$(wslpath -w "$standalone_path")
        echo "  程序路径: $windows_path"
        # 使用powershell.exe启动，更可靠
        powershell.exe -Command "Start-Process -FilePath '$windows_path'" 2>/dev/null
        
        log_success "独立程序已启动！"
        
    else
        log_error "未找到独立程序"
        log_info "请先编译: ./claude_auto_build.sh debug"
        log_info "或者编译并运行: ./claude_auto_build.sh run"
        return 1
    fi
}

# 显示构建状态
show_build_status() {
    log_info "📊 当前构建状态:"
    echo ""
    
    local debug_exe="$BUILD_DIR/x64/Debug/Standalone Plugin/MonitorControllerMax.exe"
    local debug_vst3="$BUILD_DIR/x64/Debug/VST3/MonitorControllerMax.vst3"
    local release_exe="$BUILD_DIR/x64/Release/Standalone Plugin/MonitorControllerMax.exe"
    
    echo "Debug版本:"
    if [ -f "$debug_exe" ]; then
        local mod_time=$(stat -c %y "$debug_exe" 2>/dev/null | cut -d. -f1)
        local size=$(stat -c%s "$debug_exe" 2>/dev/null || echo "unknown")
        local size_mb=$(echo "scale=2; $size / 1024 / 1024" | bc 2>/dev/null || echo "unknown")
        echo "  ✅ 独立程序: 已构建 (${size_mb}MB, $mod_time)"
    else
        echo "  ❌ 独立程序: 未构建"
    fi
    
    if [ -f "$debug_vst3" ]; then
        echo "  ✅ VST3插件: 已构建"
    else
        echo "  ❌ VST3插件: 未构建"
    fi
    
    echo ""
    echo "Release版本:"
    if [ -f "$release_exe" ]; then
        echo "  ✅ 独立程序: 已构建"
    else
        echo "  ⚪ 独立程序: 未构建"
    fi
    
    echo ""
    echo "最近的日志文件:"
    cd "$BUILD_DIR"
    ls -lt *.log 2>/dev/null | head -3 | while read line; do
        echo "  📄 $line"
    done
}

# 清理构建文件
clean_build() {
    log_info "🧽 清理构建文件..."
    
    cd "$BUILD_DIR"
    
    # 删除输出目录
    rm -rf x64/ 2>/dev/null || true
    
    # 删除日志文件
    rm -f *.log *.txt 2>/dev/null || true
    
    log_success "清理完成"
}

# 主函数
main() {
    echo "🚀 Claude Code 自动化编译脚本"
    echo "====================================="
    echo ""
    
    # 检查参数
    case "${1:-debug}" in
        "debug"|"d"|"quick"|"q")
            check_environment
            if debug_build; then
                log_success "Debug编译完成 - 强制清理模式确保最新代码"
            else
                log_error "Debug编译失败"
                exit 1
            fi
            ;;
        "release"|"r")
            check_environment
            if release_build; then
                log_success "Release编译完成 - 强制清理模式确保最新代码"
            else
                log_error "Release编译失败"
                exit 1
            fi
            ;;
        "run")
            check_environment
            if build_and_run; then
                log_success "编译并运行完成"
            else
                log_error "编译或运行失败"
                exit 1
            fi
            ;;
        "start"|"st")
            run_standalone
            ;;
        "status"|"s")
            show_build_status
            ;;
        "clean"|"c")
            clean_build
            ;;
        "help"|"h"|*)
            echo "用法: $0 [命令]"
            echo ""
            echo "编译命令 (所有编译都强制清理，确保最新代码):"
            echo "  debug, d     Debug编译 (默认, Claude Code日常开发)"
            echo "  release, r   Release编译 (最终发布准备)"
            echo ""
            echo "运行命令:"
            echo "  run          编译并运行Debug独立程序 (一键操作)"
            echo "  start, st    仅运行已有的Debug独立程序"
            echo ""
            echo "工具命令:"
            echo "  status, s    显示构建状态"
            echo "  clean, c     清理构建文件"
            echo "  help, h      显示此帮助"
            echo ""
            echo "重要说明:"
            echo "  - 所有编译操作都会强制清理并重新编译"
            echo "  - 这确保每次编译都使用最新的代码"
            echo "  - 避免因增量编译导致的代码更新问题"
            echo ""
            echo "常用示例:"
            echo "  $0 run       # 编译并运行 (最常用的开发命令)"
            echo "  $0 debug     # 仅编译Debug版本"
            echo "  $0 start     # 仅运行已有程序"
            echo "  $0 status    # 检查当前状态"
            ;;
    esac
}

# 执行主函数
main "$@"