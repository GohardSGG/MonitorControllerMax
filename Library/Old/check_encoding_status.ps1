param(
    # 定义脚本参数，允许您指定目标目录，默认为 'MonitorControllerMax/Source'
    [string]$targetDir = "MonitorControllerMax/Source"
)

# 检查目标目录是否存在
if (-not (Test-Path $targetDir)) {
    Write-Error "错误：找不到目标目录 '$targetDir'。"
    exit 1
}

Write-Host "开始检查目录 '$targetDir' 下所有文件的编码格式..."
Write-Host "----------------------------------------------------"

# 获取目录下所有文件（包括子目录）
$files = Get-ChildItem -Path $targetDir -Recurse -File

$filesWithBom = @()
$filesWithoutBom = @()

# 遍历所有文件
foreach ($file in $files) {
    $filePath = $file.FullName
    
    # 高效地读取文件的前3个字节来检查BOM
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
        $filesWithBom += $file.Name
    } else {
        $filesWithoutBom += $file.Name
    }
}

# 报告结果
Write-Host ""
if ($filesWithBom.Count -gt 0) {
    Write-Host "以下文件已是 UTF-8 with BOM 格式:" -ForegroundColor Green
    $filesWithBom | ForEach-Object { Write-Host " - $_" }
} else {
    Write-Host "没有找到任何已经是 UTF-8 with BOM 格式的文件。" -ForegroundColor Yellow
}

Write-Host ""
if ($filesWithoutBom.Count -gt 0) {
    Write-Host "以下文件不是 UTF-8 with BOM 格式 (需要转换):" -ForegroundColor Yellow
    $filesWithoutBom | ForEach-Object { Write-Host " - $_" }
} else {
    Write-Host "所有文件都已是 UTF-8 with BOM 格式。" -ForegroundColor Green
}

Write-Host ""
Write-Host "----------------------------------------------------"
Write-Host "检查完毕。共检查了 $($files.Count) 个文件。"
Write-Host " -> $($filesWithBom.Count) 个文件符合格式。"
Write-Host " -> $($filesWithoutBom.Count) 个文件需要转换。"
 