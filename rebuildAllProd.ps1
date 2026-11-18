# Prod-сборка под Windows: тесты -> receiver-micro (no-bundle) -> resources -> uploader-desktop (установщики).
$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

function Assert-ExitCode($step) {
    if ($LASTEXITCODE -ne 0) {
        Write-Host "`n========`n`t LOG_ERROR: $step завершился с кодом $LASTEXITCODE`n========" -ForegroundColor Red
        exit $LASTEXITCODE
    }
}

echo "`n`r========`n`r`t LOG_INFO: Собираем фронтенд receiver-micro`n`r========`n`r"; `
cd apps\receiver-micro; npm install; Assert-ExitCode "npm install receiver-micro"; `
npm run build; Assert-ExitCode "build фронтенда receiver-micro"; cd ..\..\; `

echo "`n`r========`n`r`t LOG_INFO: Собираем фронтенд uploader-desktop`n`r========`n`r"; `
cd apps\uploader-desktop; npm install; Assert-ExitCode "npm install uploader-desktop"; `
npm run build; Assert-ExitCode "build фронтенда uploader-desktop"; cd ..\..\; `

# generate_context! вшивает frontendDist на этапе компиляции,
# поэтому cargo test возможен только после сборки фронтендов.
echo "`n`r========`n`r`t LOG_INFO: Прогоняем все тесты`n`r========`n`r"; `
cargo test --workspace; Assert-ExitCode "cargo test"; `

echo "`n`r========`n`r`t LOG_INFO: Собираем tauri receiver-micro (release, без установщика)`n`r========`n`r"; `
cd apps\receiver-micro; npm run tauri build -- --no-bundle; Assert-ExitCode "tauri build receiver-micro"; cd ..\..\; `

echo "`n`r========`n`r`t LOG_INFO: Копируем receiver-micro.exe в uploader-desktop`n`r========`n`r"; `
New-Item -ItemType Directory -Force -Path apps\uploader-desktop\src-tauri\resources | Out-Null; `
Copy-Item   target\release\receiver-micro.exe `
            apps\uploader-desktop\src-tauri\resources\ -Force; `

echo "`n`r========`n`r`t LOG_INFO: Собираем tauri uploader-desktop (release, с установщиками)`n`r========`n`r"; `
cd apps\uploader-desktop; npm run tauri build; Assert-ExitCode "tauri build uploader-desktop"; cd ..\..\; `

echo "`n`r========`n`r`t LOG_INFO: Готово. Установщики: target\release\bundle\`n`r========`n`r"