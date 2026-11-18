# Prod-сборка под Windows: фронтенды -> тесты -> receiver-micro (no-bundle) -> resources -> установщики отправителя.
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

function Write-Step($message) {
    Write-Host "`n========`n`t LOG_INFO: $message`n========`n" -ForegroundColor Cyan
}

function Assert-ExitCode($step) {
    if ($LASTEXITCODE -ne 0) {
        Write-Host "`n========`n`t LOG_ERROR: $step завершился с кодом $LASTEXITCODE`n========" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

Write-Step "Собираем фронтенд receiver-micro"
Push-Location apps\receiver-micro
npm install
Assert-ExitCode "npm install receiver-micro"
npm run build
Assert-ExitCode "build фронтенда receiver-micro"
Pop-Location

Write-Step "Собираем фронтенд uploader-desktop"
Push-Location apps\uploader-desktop
npm install
Assert-ExitCode "npm install uploader-desktop"
npm run build
Assert-ExitCode "build фронтенда uploader-desktop"
Pop-Location

# generate_context! вшивает frontendDist на этапе компиляции,
# поэтому cargo test возможен только после сборки фронтендов.
Write-Step "Прогоняем все тесты"
cargo test --workspace
Assert-ExitCode "cargo test"

Write-Step "Собираем tauri receiver-micro (release, без установщика)"
Push-Location apps\receiver-micro
npm run tauri build -- --no-bundle
Assert-ExitCode "tauri build receiver-micro"
Pop-Location

Write-Step "Копируем receiver-micro.exe в uploader-desktop"
New-Item -ItemType Directory -Force -Path apps\uploader-desktop\src-tauri\resources | Out-Null
Copy-Item target\release\receiver-micro.exe apps\uploader-desktop\src-tauri\resources\ -Force

Write-Step "Собираем tauri uploader-desktop (release, с установщиками)"
Push-Location apps\uploader-desktop
npm run tauri build
Assert-ExitCode "tauri build uploader-desktop"
Pop-Location

Write-Step "Готово. Установщики: target\release\bundle\"