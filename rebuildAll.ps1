cargo test --workspace; `
cd apps\receiver-micro; npm install; npm run build; cd ..\..\; `
cd apps\uploader-desktop; npm install; npm run build; cd ..\..\; `
cargo test --workspace; `
cd apps\receiver-micro; npm run tauri build -- --no-bundle; cd ..\..\; `
Copy-Item target\release\receiver-micro.exe apps\uploader-desktop\src-tauri\resources\ -Force; `
cd apps\uploader-desktop; npm run tauri dev; cd ..\..\;
