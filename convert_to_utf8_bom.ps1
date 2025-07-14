param(
    # 定义脚本参数，允许您指定目标目录，默认为 'MonitorControllerMax/Source'
    [string]$targetDir = "MonitorControllerMax/Source"
)

# 检查目标目录是否存在
if (-not (Test-Path $targetDir)) {
    Write-Error "错误：找不到目标目录 '$targetDir'。"
    exit 1
}

# 获取目录下所有文件（包括子目录）
$files = Get-ChildItem -Path $targetDir -Recurse -File

# 创建一个带BOM的UTF-8编码器
$utf8WithBomEncoding = New-Object System.Text.UTF8Encoding($true)

Write-Host "开始处理文件，目标目录: $targetDir"

# 遍历所有文件
foreach ($file in $files) {
    $filePath = $file.FullName
    
    # 只读取文件的前3个字节来检查BOM，这样更高效
    $stream = New-Object System.IO.FileStream($filePath, [System.IO.FileMode]::Open, [System.IO.FileAccess]::Read)
    $bytes = New-Object byte[] 3
    $count = $stream.Read($bytes, 0, 3)
    $stream.Close()
    $stream.Dispose()

    $hasBom = $false
    # UTF-8 BOM 的字节序列是 EF BB BF
    if ($count -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
        $hasBom = $true
    }

    if ($hasBom) {
        Write-Host "跳过 '$($file.Name)': 文件已是 UTF-8 with BOM 格式。"
    } else {
        Write-Host "正在转换 '$($file.Name)' 为 UTF-8 with BOM..."
        try {
            # 使用默认编码读取文件内容。
            # .NET/.NET Core的ReadAllText会自动检测多种编码。对于没有BOM的源文件，这通常是系统默认的ANSI代码页或UTF-8。
            $content = [System.IO.File]::ReadAllText($filePath)
            
            # 使用带BOM的UTF-8编码写回文件
            [System.IO.File]::WriteAllText($filePath, $content, $utf8WithBomEncoding)
            
            Write-Host " -> 成功转换。" -ForegroundColor Green
        } catch {
            Write-Error "转换失败 '$($file.Name)': $_"
        }
    }
}

Write-Host "脚本执行完毕。"
