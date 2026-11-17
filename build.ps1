# Локальная сборка под Windows: receiver-micro -> resources -> uploader-desktop.
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

Write-Host "==> 1/3 receiver-micro" -ForegroundColor Cyan
Push-Location apps/receiver-micro
npm install
npm run tauri build -- --no-bundle
Pop-Location

Write-Host "==> 2/3 копирование шаблона в ресурсы отправителя" -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path "apps/uploader-desktop/src-tauri/resources" | Out-Null
Copy-Item "target/release/receiver-micro.exe" "apps/uploader-desktop/src-tauri/resources/receiver-micro.exe" -Force

Write-Host "==> 3/3 uploader-desktop" -ForegroundColor Cyan
Push-Location apps/uploader-desktop
npm install
npm run tauri build
Pop-Location

Write-Host "Готово. Установщики: target/release/bundle/" -ForegroundColor Green
