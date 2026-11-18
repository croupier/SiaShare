echo "`n`r========`n`r`t LOG_INFO: Собираем receiver-micro`n`r========`n`r"; `
cd apps\receiver-micro; npm install; npm run build; cd ..\..\; `

echo "`n`r========`n`r`t LOG_INFO: Собираем uploader-desktop`n`r========`n`r"; `
cd apps\uploader-desktop; npm install; npm run build; cd ..\..\; `

echo "`n`r========`n`r`t LOG_INFO: Прогоняем все тесты`n`r========`n`r"; `
cargo test --workspace; `

echo "`n`r========`n`r`t LOG_INFO: Собираем tauri receiver-micro`n`r========`n`r"; `
cd apps\receiver-micro; npm run tauri build -- --no-bundle; cd ..\..\; `

echo "`n`r========`n`r`t LOG_INFO: Копируем receiver-micro.exe в uploader-desktop`n`r========`n`r"; `
Copy-Item   target\release\receiver-micro.exe `
            apps\uploader-desktop\src-tauri\resources\ -Force; `

echo "`n`r========`n`r`t LOG_INFO: Освобождаем порт 1420`n`r========`n`r"; `
Get-NetTCPConnection `
    -LocalPort 1420 -State Listen -ErrorAction SilentlyContinue | `
    ForEach-Object { `
        Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue `
    }; `
echo "`n`r========`n`r`t LOG_INFO: Собираем tauri dev uploader-desktop`n`r========`n`r"; `
cd apps\uploader-desktop; npm run tauri dev; cd ..\..\;
