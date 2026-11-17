<#
    collect-context.ps1 — компактный дамп проекта для передачи AI-агенту.
    Запуск:
      powershell -ExecutionPolicy Bypass -File .\collect-context.ps1 -Root D:\Projects\SiaShare -SiaSdkApi
    Результат: CONTEXT_DUMP.md в корне проекта.
    Внимание: дамп может содержать приватные данные — просмотри перед отправкой.
#>
[CmdletBinding()]
param(
    [string]$Root = (Get-Location).Path,   # корень репозитория
    [string]$Out  = 'CONTEXT_DUMP.md',     # файл-результат
    [int]$MaxFileKB = 48,                  # файлы больше — не включаем целиком
    [int]$MaxHits = 300,                   # лимит совпадений на секцию
    [switch]$SiaSdkApi                     # добавить реальные сигнатуры sia_storage из кэша cargo
)

$ErrorActionPreference = 'Stop'
$Root = (Resolve-Path -LiteralPath $Root).Path.TrimEnd('\')
$sb = New-Object System.Text.StringBuilder

# --- вспомогательные функции ---
function Add-Section([string]$title) {
    [void]$sb.AppendLine(''); [void]$sb.AppendLine("## $title"); [void]$sb.AppendLine('')
}
function Add-Code([string]$text, [string]$lang = '') {
    [void]$sb.AppendLine('```' + $lang)
    [void]$sb.AppendLine($text.TrimEnd())
    [void]$sb.AppendLine('```')
}
function Get-Rel([string]$full) { $full.Substring($Root.Length).TrimStart('\') }

# --- обход дерева с отсечением тяжёлых каталогов ---
$excludeDirs   = @('target','node_modules','.git','.svelte-kit','dist','build','out','.turbo','coverage','__pycache__')
$skipFileNames = @('Cargo.lock','package-lock.json','pnpm-lock.yaml','yarn.lock')
$srcExt        = @('.rs','.ts','.js','.svelte','.json','.toml','.sql','.css','.html','.md','.ps1','.yml','.yaml')

$allFiles = New-Object System.Collections.Generic.List[object]
$allDirs  = New-Object System.Collections.Generic.List[string]
$stack = New-Object System.Collections.Generic.Stack[string]
$stack.Push($Root)
while ($stack.Count -gt 0) {
    $cur = $stack.Pop()
    $entries = @()
    try { $entries = Get-ChildItem -LiteralPath $cur -Force -ErrorAction Stop } catch { continue }
    foreach ($e in $entries) {
        if ($e.PSIsContainer) {
            if ($excludeDirs -contains $e.Name) { continue }   # внутрь не заходим
            $allDirs.Add($e.FullName); $stack.Push($e.FullName)
        } else { $allFiles.Add($e) }
    }
}

$src = $allFiles | Where-Object {
    $srcExt -contains $_.Extension.ToLower() -and
    $skipFileNames -notcontains $_.Name -and
    $_.Name -notlike '*.min.*' -and
    $_.Name -notlike '.env*'                 # секреты не тащим
} | Sort-Object FullName
$rs = @($src | Where-Object { $_.Extension -eq '.rs' })
$fe = @($src | Where-Object { $_.Extension -in @('.ts','.js','.svelte') })

[void]$sb.AppendLine("# CONTEXT DUMP: $(Split-Path $Root -Leaf)")
[void]$sb.AppendLine("Корень: ``$Root``  |  Собран: $(Get-Date -Format 'yyyy-MM-dd HH:mm')  |  Файлов исходников: $($src.Count)")

# --- 1. каталоги ---
Add-Section 'Каталоги'
$tree = $allDirs | Sort-Object | ForEach-Object {
    $rel = Get-Rel $_
    ('  ' * (($rel -split '\\').Count - 1)) + (Split-Path $_ -Leaf) + '/'
}
Add-Code ($tree -join "`n")

# --- 2. файлы и объём (где живёт логика) ---
Add-Section 'Исходные файлы (путь — строк)'
$list = foreach ($f in $src) {
    $n = 0
    try { $n = (Get-Content -LiteralPath $f.FullName -ErrorAction Stop | Measure-Object -Line).Lines } catch {}
    '{0} — {1}' -f (Get-Rel $f.FullName), $n
}
Add-Code ($list -join "`n")

# --- 3. манифесты и конфиги целиком ---
Add-Section 'Манифесты и конфиги'
$keyNames = @('Cargo.toml','package.json','tauri.conf.json','svelte.config.js','vite.config.ts','vite.config.js','tsconfig.json','build.rs')
foreach ($f in $src) {
    $rel = Get-Rel $f.FullName
    $isKey = ($keyNames -contains $f.Name) -or ($rel -like '*capabilities\*.json') -or ($rel -eq 'crates\sia-core\src\lib.rs')
    if (-not $isKey) { continue }
    if ($f.Length -gt ($MaxFileKB * 1KB)) { [void]$sb.AppendLine("### $rel — пропущен (>$MaxFileKB КБ)"); continue }
    [void]$sb.AppendLine("### $rel")
    Add-Code (Get-Content -LiteralPath $f.FullName -Raw)
}

# --- 4. контракт IPC: Tauri-команды ---
Add-Section 'Tauri-команды (сигнатуры)'
if ($rs.Count) {
    $h = Select-String -LiteralPath $rs.FullName -Pattern '#\[tauri::command\]' -Context 0,10 | Select-Object -First $MaxHits
    $t = foreach ($x in $h) { "--- {0}:{1}`n{2}`n{3}" -f (Get-Rel $x.Path), $x.LineNumber, $x.Line, ($x.Context.PostContext -join "`n") }
    Add-Code ($t -join "`n") 'rust'
}

# --- 5. публичный API общего ядра ---
Add-Section 'sia-core: публичный API'
$core = @($rs | Where-Object { (Get-Rel $_.FullName) -like 'crates\sia-core\*' })
if ($core.Count) {
    $h = Select-String -LiteralPath $core.FullName -Pattern '^\s*pub (async )?(fn|struct|enum|trait|mod|const|type)\b' | Select-Object -First $MaxHits
    $t = $h | ForEach-Object { '{0}:{1}: {2}' -f (Get-Rel $_.Path), $_.LineNumber, $_.Line.Trim() }
    Add-Code ($t -join "`n") 'rust'
}

# --- 6. что фронт реально вызывает ---
Add-Section 'Фронтенд: invoke / listen / emit'
if ($fe.Count) {
    $h = Select-String -LiteralPath $fe.FullName -Pattern 'invoke[<(]|listen[<(]|emit\(' | Select-Object -First $MaxHits
    $t = $h | ForEach-Object { '{0}:{1}: {2}' -f (Get-Rel $_.Path), $_.LineNumber, $_.Line.Trim() }
    Add-Code ($t -join "`n") 'ts'
}

# --- 7. формат ссылки и схема приложения ---
Add-Section 'Ссылки / схема / ключи (зацепки)'
if ($src.Count) {
    $h = Select-String -LiteralPath $src.FullName -Pattern 'siashare:|build_link|parse_link|fn .*link|fragment|deep-link' | Select-Object -First $MaxHits
    $t = $h | ForEach-Object { '{0}:{1}: {2}' -f (Get-Rel $_.Path), $_.LineNumber, $_.Line.Trim() }
    Add-Code ($t -join "`n")
}

# --- 8. недоделки ---
Add-Section 'TODO / FIXME / unimplemented'
if ($src.Count) {
    $h = Select-String -LiteralPath $src.FullName -Pattern 'TODO|FIXME|HACK|unimplemented!|todo!\(' | Select-Object -First $MaxHits
    $t = $h | ForEach-Object { '{0}:{1}: {2}' -f (Get-Rel $_.Path), $_.LineNumber, $_.Line.Trim() }
    Add-Code ($t -join "`n")
}

# --- 9. история git (что уже делалось) ---
if (Test-Path (Join-Path $Root '.git')) {
    Add-Section 'git: последние коммиты и статус'
    $log = (& git -C $Root log --oneline -30) -join "`n"
    $st  = (& git -C $Root status --short) -join "`n"
    Add-Code ("### log`n$log`n`n### status`n$st")
}

# --- 10. реальные сигнатуры SDK (лечит галлюцинации по API) ---
if ($SiaSdkApi) {
    Add-Section 'sia_storage 0.13.x: публичные сигнатуры из кэша cargo'
    $reg = Join-Path $env:USERPROFILE '.cargo\registry\src'
    $sdk = @()
    if (Test-Path $reg) {
        $sdk = @(Get-ChildItem -LiteralPath $reg -Directory | ForEach-Object {
            Get-ChildItem -LiteralPath $_.FullName -Directory -Filter 'sia_storage-0.13*' -ErrorAction SilentlyContinue
        })
    }
    if (-not $sdk.Count) {
        [void]$sb.AppendLine('Крейт не найден в кэше. Выполни `cargo fetch` в корне проекта и повтори с -SiaSdkApi.')
    }
    foreach ($d in $sdk) {
        [void]$sb.AppendLine("### $($d.Name)")
        $files = @(Get-ChildItem -LiteralPath $d.FullName -Recurse -File -Filter '*.rs')
        $h = Select-String -LiteralPath $files.FullName -Pattern '^\s*pub (async )?(fn|struct|enum|trait)\b' | Select-Object -First $MaxHits
        $t = $h | ForEach-Object { '{0}:{1}: {2}' -f $_.Path.Substring($d.FullName.Length).TrimStart('\'), $_.LineNumber, $_.Line.Trim() }
        Add-Code ($t -join "`n") 'rust'
    }
}

# --- запись в UTF-8 без BOM ---
$outPath = Join-Path $Root $Out
[System.IO.File]::WriteAllText($outPath, $sb.ToString(), (New-Object System.Text.UTF8Encoding($false)))
Write-Host ("Готово: {0} ({1:N0} КБ)" -f $outPath, ((Get-Item $outPath).Length / 1KB)) -ForegroundColor Green