# CONTEXT DUMP: SiaShare
Корень: `D:\_sites\3406\SiaShare`  |  Собран: 2026-11-17 21:04  |  Файлов исходников: 95

## Каталоги

```
.github/
  workflows/
.srcBuilders/
  .github/
    workflows/
  apps/
    receiver-micro/
      src/
      src-tauri/
        capabilities/
        icons/
        src/
    uploader-desktop/
      src/
      src-tauri/
        capabilities/
        icons/
        resources/
        src/
  crates/
    sia-core/
      src/
apps/
  receiver-micro/
    src/
    src-tauri/
      capabilities/
      gen/
        schemas/
      icons/
      src/
  uploader-desktop/
    src/
    src-tauri/
      capabilities/
      gen/
        schemas/
      icons/
      resources/
      src/
crates/
  sia-core/
    src/
```

## Исходные файлы (путь — строк)

```
.github\workflows\ci-cd.yml — 109
.srcBuilders\.github\workflows\ci-cd.yml — 109
.srcBuilders\apps\receiver-micro\index.html — 12
.srcBuilders\apps\receiver-micro\package.json — 26
.srcBuilders\apps\receiver-micro\src\App.svelte — 183
.srcBuilders\apps\receiver-micro\src\main.ts — 5
.srcBuilders\apps\receiver-micro\src\vite-env.d.ts — 2
.srcBuilders\apps\receiver-micro\src-tauri\build.rs — 3
.srcBuilders\apps\receiver-micro\src-tauri\capabilities\default.json — 7
.srcBuilders\apps\receiver-micro\src-tauri\Cargo.toml — 14
.srcBuilders\apps\receiver-micro\src-tauri\src\commands.rs — 112
.srcBuilders\apps\receiver-micro\src-tauri\src\main.rs — 32
.srcBuilders\apps\receiver-micro\src-tauri\tauri.conf.json — 32
.srcBuilders\apps\receiver-micro\svelte.config.js — 4
.srcBuilders\apps\receiver-micro\tsconfig.json — 16
.srcBuilders\apps\receiver-micro\tsconfig.node.json — 10
.srcBuilders\apps\receiver-micro\vite.config.ts — 14
.srcBuilders\apps\uploader-desktop\index.html — 12
.srcBuilders\apps\uploader-desktop\package.json — 26
.srcBuilders\apps\uploader-desktop\src\App.svelte — 208
.srcBuilders\apps\uploader-desktop\src\main.ts — 5
.srcBuilders\apps\uploader-desktop\src\vite-env.d.ts — 2
.srcBuilders\apps\uploader-desktop\src-tauri\build.rs — 3
.srcBuilders\apps\uploader-desktop\src-tauri\capabilities\default.json — 14
.srcBuilders\apps\uploader-desktop\src-tauri\Cargo.toml — 25
.srcBuilders\apps\uploader-desktop\src-tauri\src\commands.rs — 210
.srcBuilders\apps\uploader-desktop\src-tauri\src\db.rs — 346
.srcBuilders\apps\uploader-desktop\src-tauri\src\main.rs — 42
.srcBuilders\apps\uploader-desktop\src-tauri\src\worker.rs — 168
.srcBuilders\apps\uploader-desktop\src-tauri\tauri.conf.json — 34
.srcBuilders\apps\uploader-desktop\svelte.config.js — 4
.srcBuilders\apps\uploader-desktop\tsconfig.json — 16
.srcBuilders\apps\uploader-desktop\tsconfig.node.json — 10
.srcBuilders\apps\uploader-desktop\vite.config.ts — 17
.srcBuilders\build.ps1 — 17
.srcBuilders\Cargo.toml — 35
.srcBuilders\crates\sia-core\Cargo.toml — 18
.srcBuilders\crates\sia-core\src\crypto.rs — 202
.srcBuilders\crates\sia-core\src\downloader.rs — 246
.srcBuilders\crates\sia-core\src\lib.rs — 7
.srcBuilders\crates\sia-core\src\models.rs — 61
.srcBuilders\crates\sia-core\src\sia_client.rs — 305
.srcBuilders\README.md — 66
apps\receiver-micro\index.html — 12
apps\receiver-micro\package.json — 26
apps\receiver-micro\src\App.svelte — 183
apps\receiver-micro\src\main.ts — 5
apps\receiver-micro\src\vite-env.d.ts — 2
apps\receiver-micro\src-tauri\build.rs — 3
apps\receiver-micro\src-tauri\capabilities\default.json — 7
apps\receiver-micro\src-tauri\Cargo.toml — 14
apps\receiver-micro\src-tauri\gen\schemas\acl-manifests.json — 1
apps\receiver-micro\src-tauri\gen\schemas\capabilities.json — 1
apps\receiver-micro\src-tauri\gen\schemas\desktop-schema.json — 2382
apps\receiver-micro\src-tauri\gen\schemas\windows-schema.json — 2382
apps\receiver-micro\src-tauri\src\commands.rs — 112
apps\receiver-micro\src-tauri\src\main.rs — 32
apps\receiver-micro\src-tauri\tauri.conf.json — 32
apps\receiver-micro\svelte.config.js — 4
apps\receiver-micro\tsconfig.json — 16
apps\receiver-micro\tsconfig.node.json — 10
apps\receiver-micro\vite.config.ts — 14
apps\uploader-desktop\index.html — 12
apps\uploader-desktop\package.json — 26
apps\uploader-desktop\src\App.svelte — 208
apps\uploader-desktop\src\main.ts — 5
apps\uploader-desktop\src\vite-env.d.ts — 2
apps\uploader-desktop\src-tauri\build.rs — 3
apps\uploader-desktop\src-tauri\capabilities\default.json — 14
apps\uploader-desktop\src-tauri\Cargo.toml — 25
apps\uploader-desktop\src-tauri\gen\schemas\acl-manifests.json — 1
apps\uploader-desktop\src-tauri\gen\schemas\capabilities.json — 1
apps\uploader-desktop\src-tauri\gen\schemas\desktop-schema.json — 2615
apps\uploader-desktop\src-tauri\gen\schemas\windows-schema.json — 2615
apps\uploader-desktop\src-tauri\src\commands.rs — 210
apps\uploader-desktop\src-tauri\src\db.rs — 346
apps\uploader-desktop\src-tauri\src\main.rs — 42
apps\uploader-desktop\src-tauri\src\worker.rs — 168
apps\uploader-desktop\src-tauri\tauri.conf.json — 34
apps\uploader-desktop\svelte.config.js — 4
apps\uploader-desktop\tsconfig.json — 16
apps\uploader-desktop\tsconfig.node.json — 10
apps\uploader-desktop\vite.config.ts — 17
build.ps1 — 17
Cargo copy.toml — 31
Cargo.toml — 35
collect-context.ps1 — 151
crates\sia-core\Cargo.toml — 18
crates\sia-core\src\crypto.rs — 202
crates\sia-core\src\downloader.rs — 246
crates\sia-core\src\lib.rs — 7
crates\sia-core\src\models.rs — 61
crates\sia-core\src\sia_client.rs — 305
QWEN.md — 183
README.md — 66
```

## Манифесты и конфиги

### .srcBuilders\apps\receiver-micro\package.json
```
{
  "name": "receiver-micro",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-check --tsconfig ./tsconfig.json",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^3.1.2",
    "@tauri-apps/cli": "^2",
    "@tsconfig/svelte": "^5.0.4",
    "svelte": "^4.2.19",
    "svelte-check": "^3.8.6",
    "typescript": "^5.6.3",
    "vite": "^5.4.10"
  }
}
```
### .srcBuilders\apps\receiver-micro\src-tauri\build.rs
```
fn main() {
    tauri_build::build()
}
```
### .srcBuilders\apps\receiver-micro\src-tauri\capabilities\default.json
```
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "РџСЂР°РІР° РґРѕСЃС‚СѓРїР° РґР»СЏ РјРёРєСЂРѕ-РєР»РёРµРЅС‚Р° РїРѕР»СѓС‡Р°С‚РµР»СЏ",
  "windows": ["main"],
  "permissions": ["core:default", "dialog:default"]
}
```
### .srcBuilders\apps\receiver-micro\src-tauri\Cargo.toml
```
[package]
name = "receiver-micro"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
sia-core = { path = "../../../crates/sia-core" }
sia_storage = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
```
### .srcBuilders\apps\receiver-micro\src-tauri\tauri.conf.json
```
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "SiaShare Receiver",
  "mainBinaryName": "receiver-micro",
  "version": "0.1.0",
  "identifier": "com.siashare.receiver",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1430",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      { "label": "main", "title": "SiaShare вЂ” РЎРєР°С‡РёРІР°РЅРёРµ С„Р°Р№Р»РѕРІ", "width": 680, "height": 480, "resizable": true }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: http://ipc.localhost",
      "capabilities": ["default"]
    }
  },
  "bundle": {
    "active": false,
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```
### .srcBuilders\apps\receiver-micro\svelte.config.js
```
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
};
```
### .srcBuilders\apps\receiver-micro\tsconfig.json
```
{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": {
    "target": "ESNext",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "resolveJsonModule": true,
    "allowJs": true,
    "checkJs": true,
    "isolatedModules": true,
    "moduleResolution": "bundler",
    "strict": true
  },
  "include": ["src/**/*.ts", "src/**/*.js", "src/**/*.svelte"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```
### .srcBuilders\apps\receiver-micro\vite.config.ts
```
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1431 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
```
### .srcBuilders\apps\uploader-desktop\package.json
```
{
  "name": "uploader-desktop",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-check --tsconfig ./tsconfig.json",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^3.1.2",
    "@tauri-apps/cli": "^2",
    "@tsconfig/svelte": "^5.0.4",
    "svelte": "^4.2.19",
    "svelte-check": "^3.8.6",
    "typescript": "^5.6.3",
    "vite": "^5.4.10"
  }
}
```
### .srcBuilders\apps\uploader-desktop\src-tauri\build.rs
```
fn main() {
    tauri_build::build()
}
```
### .srcBuilders\apps\uploader-desktop\src-tauri\capabilities\default.json
```
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "РџСЂР°РІР° РґРѕСЃС‚СѓРїР° РґР»СЏ РѕСЃРЅРѕРІРЅРѕРіРѕ РѕРєРЅР° РѕС‚РїСЂР°РІРёС‚РµР»СЏ",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    {
      "identifier": "opener:allow-open-url",
      "allow": [{ "url": "https://sia.storage/*" }]
    }
  ]
}
```
### .srcBuilders\apps\uploader-desktop\src-tauri\Cargo.toml
```
[package]
name = "uploader-desktop"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
sia-core = { path = "../../../crates/sia-core" }
sia_storage = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
rusqlite = { workspace = true }
uuid = { workspace = true }
zip = { workspace = true }
chrono = { workspace = true }
hex = { workspace = true }
walkdir = { workspace = true }
keyring = { workspace = true }
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
tauri-plugin-opener = "2"
tauri-plugin-single-instance = "2"

[dev-dependencies]
tempfile = { workspace = true }
```
### .srcBuilders\apps\uploader-desktop\src-tauri\tauri.conf.json
```
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "SiaShare Uploader",
  "mainBinaryName": "sia-share-uploader",
  "version": "0.1.0",
  "identifier": "com.siashare.uploader",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      { "label": "main", "title": "SiaShare Uploader", "width": 720, "height": 540, "resizable": true }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: http://ipc.localhost",
      "capabilities": ["default"]
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "resources": ["resources/*"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```
### .srcBuilders\apps\uploader-desktop\svelte.config.js
```
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
};
```
### .srcBuilders\apps\uploader-desktop\tsconfig.json
```
{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": {
    "target": "ESNext",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "resolveJsonModule": true,
    "allowJs": true,
    "checkJs": true,
    "isolatedModules": true,
    "moduleResolution": "bundler",
    "strict": true
  },
  "include": ["src/**/*.ts", "src/**/*.js", "src/**/*.svelte"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```
### .srcBuilders\apps\uploader-desktop\vite.config.ts
```
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// TAURI_DEV_HOST Р·Р°РґР°РµС‚СЃСЏ РїСЂРё РѕС‚Р»Р°РґРєРµ РЅР° РјРѕР±РёР»СЊРЅРѕРј СѓСЃС‚СЂРѕР№СЃС‚РІРµ
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  // Tauri СЃР°Рј СЂРёСЃСѓРµС‚ РѕРєРЅРѕ: РЅРµ РѕС‡РёС‰Р°РµРј РµРіРѕ РІС‹РІРѕРґ РІ С‚РµСЂРјРёРЅР°Р»Рµ
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    // Rust-РєРѕРґ РїРµСЂРµСЃРѕР±РёСЂР°РµС‚ Tauri, Vite Р·Р° РЅРёРј РЅРµ СЃР»РµРґРёС‚
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
```
### .srcBuilders\Cargo.toml
```
[workspace]
resolver = "2"
members = [
    "crates/sia-core",
    "apps/uploader-desktop/src-tauri",
    "apps/receiver-micro/src-tauri",
]

[workspace.dependencies]
# РћС„РёС†РёР°Р»СЊРЅС‹Р№ Rust SDK Sia Storage. Р’РµС‚РєР° 0.13.x вЂ” API, РїРѕРґ РєРѕС‚РѕСЂС‹Р№ РЅР°РїРёСЃР°РЅ РїСЂРѕРµРєС‚.
sia_storage = "0.13"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
aes-gcm = "0.10"
argon2 = "0.5"
rand = "0.8"
base64 = "0.22"
# ZIP64 (large_file) РЅСѓР¶РµРЅ РґР»СЏ С„Р°Р№Р»РѕРІ >4 Р“РёР‘; async_zip РЅРµ РёСЃРїРѕР»СЊР·СѓРµРј.
zip = { version = "2", default-features = false, features = ["deflate"] }
rusqlite = { version = "0.32", features = ["bundled"] }
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
hex = "0.4"
walkdir = "2"
tempfile = "3"
# РЎРёСЃС‚РµРјРЅС‹Р№ keychain: AppKey РЅРµР»СЊР·СЏ С…СЂР°РЅРёС‚СЊ plaintext-С„Р°Р№Р»РѕРј.
keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }

# РћРїС‚РёРјРёР·Р°С†РёСЏ РїРѕРґ СЂР°Р·РјРµСЂ: РєСЂРёС‚РёС‡РЅРѕ РґР»СЏ РїРµСЂРµРЅРѕСЃРёРјРѕРіРѕ receiver-micro.
[profile.release]
panic = "abort"
codegen-units = 1
lto = true
opt-level = "z"
strip = true
```
### .srcBuilders\crates\sia-core\Cargo.toml
```
[package]
name = "sia-core"
version = "0.1.0"
edition = "2021"

[dependencies]
sia_storage = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
aes-gcm = { workspace = true }
argon2 = { workspace = true }
rand = { workspace = true }
base64 = { workspace = true }
zip = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```
### apps\receiver-micro\package.json
```
{
  "name": "receiver-micro",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-check --tsconfig ./tsconfig.json",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^3.1.2",
    "@tauri-apps/cli": "^2",
    "@tsconfig/svelte": "^5.0.4",
    "svelte": "^4.2.19",
    "svelte-check": "^3.8.6",
    "typescript": "^5.6.3",
    "vite": "^5.4.10"
  }
}
```
### apps\receiver-micro\src-tauri\build.rs
```
fn main() {
    tauri_build::build()
}
```
### apps\receiver-micro\src-tauri\capabilities\default.json
```
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "РџСЂР°РІР° РґРѕСЃС‚СѓРїР° РґР»СЏ РјРёРєСЂРѕ-РєР»РёРµРЅС‚Р° РїРѕР»СѓС‡Р°С‚РµР»СЏ",
  "windows": ["main"],
  "permissions": ["core:default", "dialog:default"]
}
```
### apps\receiver-micro\src-tauri\Cargo.toml
```
[package]
name = "receiver-micro"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
sia-core = { path = "../../../crates/sia-core" }
sia_storage = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
```
### apps\receiver-micro\src-tauri\tauri.conf.json
```
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "SiaShare Receiver",
  "mainBinaryName": "receiver-micro",
  "version": "0.1.0",
  "identifier": "com.siashare.receiver",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1430",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      { "label": "main", "title": "SiaShare вЂ” РЎРєР°С‡РёРІР°РЅРёРµ С„Р°Р№Р»РѕРІ", "width": 680, "height": 480, "resizable": true }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: http://ipc.localhost",
      "capabilities": ["default"]
    }
  },
  "bundle": {
    "active": false,
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```
### apps\receiver-micro\svelte.config.js
```
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
};
```
### apps\receiver-micro\tsconfig.json
```
{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": {
    "target": "ESNext",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "resolveJsonModule": true,
    "allowJs": true,
    "checkJs": true,
    "isolatedModules": true,
    "moduleResolution": "bundler",
    "strict": true
  },
  "include": ["src/**/*.ts", "src/**/*.js", "src/**/*.svelte"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```
### apps\receiver-micro\vite.config.ts
```
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  clearScreen: false,
  server: {
    port: 1430,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1431 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
```
### apps\uploader-desktop\package.json
```
{
  "name": "uploader-desktop",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "check": "svelte-check --tsconfig ./tsconfig.json",
    "tauri": "tauri"
  },
  "dependencies": {
    "@tauri-apps/api": "^2",
    "@tauri-apps/plugin-dialog": "^2"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^3.1.2",
    "@tauri-apps/cli": "^2",
    "@tsconfig/svelte": "^5.0.4",
    "svelte": "^4.2.19",
    "svelte-check": "^3.8.6",
    "typescript": "^5.6.3",
    "vite": "^5.4.10"
  }
}
```
### apps\uploader-desktop\src-tauri\build.rs
```
fn main() {
    tauri_build::build()
}
```
### apps\uploader-desktop\src-tauri\capabilities\default.json
```
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "РџСЂР°РІР° РґРѕСЃС‚СѓРїР° РґР»СЏ РѕСЃРЅРѕРІРЅРѕРіРѕ РѕРєРЅР° РѕС‚РїСЂР°РІРёС‚РµР»СЏ",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "dialog:default",
    {
      "identifier": "opener:allow-open-url",
      "allow": [{ "url": "https://sia.storage/*" }]
    }
  ]
}
```
### apps\uploader-desktop\src-tauri\Cargo.toml
```
[package]
name = "uploader-desktop"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
sia-core = { path = "../../../crates/sia-core" }
sia_storage = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
rusqlite = { workspace = true }
uuid = { workspace = true }
zip = { workspace = true }
chrono = { workspace = true }
hex = { workspace = true }
walkdir = { workspace = true }
keyring = { workspace = true }
tauri = { version = "2", features = [] }
tauri-plugin-dialog = "2"
tauri-plugin-opener = "2"
tauri-plugin-single-instance = "2"

[dev-dependencies]
tempfile = { workspace = true }
```
### apps\uploader-desktop\src-tauri\tauri.conf.json
```
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "SiaShare Uploader",
  "mainBinaryName": "sia-share-uploader",
  "version": "0.1.0",
  "identifier": "com.siashare.uploader",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      { "label": "main", "title": "SiaShare Uploader", "width": 720, "height": 540, "resizable": true }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' data:; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: http://ipc.localhost",
      "capabilities": ["default"]
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "resources": ["resources/*"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```
### apps\uploader-desktop\svelte.config.js
```
import { vitePreprocess } from "@sveltejs/vite-plugin-svelte";

export default {
  preprocess: vitePreprocess(),
};
```
### apps\uploader-desktop\tsconfig.json
```
{
  "extends": "@tsconfig/svelte/tsconfig.json",
  "compilerOptions": {
    "target": "ESNext",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "resolveJsonModule": true,
    "allowJs": true,
    "checkJs": true,
    "isolatedModules": true,
    "moduleResolution": "bundler",
    "strict": true
  },
  "include": ["src/**/*.ts", "src/**/*.js", "src/**/*.svelte"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```
### apps\uploader-desktop\vite.config.ts
```
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";

// TAURI_DEV_HOST Р·Р°РґР°РµС‚СЃСЏ РїСЂРё РѕС‚Р»Р°РґРєРµ РЅР° РјРѕР±РёР»СЊРЅРѕРј СѓСЃС‚СЂРѕР№СЃС‚РІРµ
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [svelte()],
  // Tauri СЃР°Рј СЂРёСЃСѓРµС‚ РѕРєРЅРѕ: РЅРµ РѕС‡РёС‰Р°РµРј РµРіРѕ РІС‹РІРѕРґ РІ С‚РµСЂРјРёРЅР°Р»Рµ
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    // Rust-РєРѕРґ РїРµСЂРµСЃРѕР±РёСЂР°РµС‚ Tauri, Vite Р·Р° РЅРёРј РЅРµ СЃР»РµРґРёС‚
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
```
### Cargo.toml
```
[workspace]
resolver = "2"
members = [
    "crates/sia-core",
    "apps/uploader-desktop/src-tauri",
    "apps/receiver-micro/src-tauri",
]

[workspace.dependencies]
# РћС„РёС†РёР°Р»СЊРЅС‹Р№ Rust SDK Sia Storage. Р’РµС‚РєР° 0.13.x вЂ” API, РїРѕРґ РєРѕС‚РѕСЂС‹Р№ РЅР°РїРёСЃР°РЅ РїСЂРѕРµРєС‚.
sia_storage = "0.13"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
aes-gcm = "0.10"
argon2 = "0.5"
rand = "0.8"
base64 = "0.22"
# ZIP64 (large_file) РЅСѓР¶РµРЅ РґР»СЏ С„Р°Р№Р»РѕРІ >4 Р“РёР‘; async_zip РЅРµ РёСЃРїРѕР»СЊР·СѓРµРј.
zip = { version = "2", default-features = false, features = ["deflate"] }
rusqlite = { version = "0.32", features = ["bundled"] }
uuid = { version = "1", features = ["v4"] }
chrono = "0.4"
hex = "0.4"
walkdir = "2"
tempfile = "3"
# РЎРёСЃС‚РµРјРЅС‹Р№ keychain: AppKey РЅРµР»СЊР·СЏ С…СЂР°РЅРёС‚СЊ plaintext-С„Р°Р№Р»РѕРј.
keyring = { version = "3", features = ["apple-native", "windows-native", "sync-secret-service"] }

# РћРїС‚РёРјРёР·Р°С†РёСЏ РїРѕРґ СЂР°Р·РјРµСЂ: РєСЂРёС‚РёС‡РЅРѕ РґР»СЏ РїРµСЂРµРЅРѕСЃРёРјРѕРіРѕ receiver-micro.
[profile.release]
panic = "abort"
codegen-units = 1
lto = true
opt-level = "z"
strip = true
```
### crates\sia-core\Cargo.toml
```
[package]
name = "sia-core"
version = "0.1.0"
edition = "2021"

[dependencies]
sia_storage = { workspace = true }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
aes-gcm = { workspace = true }
argon2 = { workspace = true }
rand = { workspace = true }
base64 = { workspace = true }
zip = { workspace = true }
chrono = { workspace = true }

[dev-dependencies]
tempfile = { workspace = true }
```
### crates\sia-core\src\lib.rs
```
//! РћР±С‰РµРµ СЏРґСЂРѕ SiaShare: СЂР°Р±РѕС‚Р° СЃ СЃРµС‚СЊСЋ Sia, РєСЂРёРїС‚РѕРіСЂР°С„РёСЏ РєР»СЋС‡Р° РґРѕСЃС‚СѓРїР°,
//! РЅР°СЂРµР·РєР°/СЃРєР»РµР№РєР° С‡Р°РЅРєРѕРІ Рё РїРѕС‚РѕРєРѕРІР°СЏ Р°СЂС…РёРІР°С†РёСЏ.
//! РљСЂРµР№С‚ РЅРµ Р·Р°РІРёСЃРёС‚ РѕС‚ Tauri Рё С‚РµСЃС‚РёСЂСѓРµС‚СЃСЏ РѕР±С‹С‡РЅС‹Рј `cargo test -p sia-core`.

pub mod crypto;
pub mod downloader;
pub mod models;
pub mod sia_client;
```

## Tauri-команды (сигнатуры)

```rust
--- .srcBuilders\apps\receiver-micro\src-tauri\src\commands.rs:37
#[tauri::command]
pub async fn init_receiver(state: State<'_, ReceiverState>) -> Result<InitResponse, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    match extract_key_from_path(&exe_path) {
        Ok(ExtractedKey::Plain(raw_key)) => Ok(match load_files_with_key(&state, &raw_key).await {
            Ok(files) => InitResponse::Ready(files),
            Err(e) => InitResponse::Error(e),
        }),
        Ok(ExtractedKey::Encrypted(blob)) => {
            *state.encrypted_blob.lock().await = Some(blob);
            Ok(InitResponse::PasswordRequired)
--- .srcBuilders\apps\receiver-micro\src-tauri\src\commands.rs:55
#[tauri::command]
pub async fn submit_key_payload(payload: String, state: State<'_, ReceiverState>) -> Result<InitResponse, String> {
    match parse_payload(&payload) {
        Ok(ExtractedKey::Plain(raw)) => Ok(match load_files_with_key(&state, &raw).await {
            Ok(files) => InitResponse::Ready(files),
            Err(e) => InitResponse::Error(e),
        }),
        Ok(ExtractedKey::Encrypted(blob)) => {
            *state.encrypted_blob.lock().await = Some(blob);
            Ok(InitResponse::PasswordRequired)
        }
--- .srcBuilders\apps\receiver-micro\src-tauri\src\commands.rs:70
#[tauri::command]
pub async fn unlock_with_password(password: String, state: State<'_, ReceiverState>) -> Result<Vec<SharedFileEntry>, String> {
    let blob = state.encrypted_blob.lock().await.clone().ok_or("Зашифрованный ключ не найден")?;
    let raw_key = decrypt_with_password(&blob, &password).map_err(|e| e.to_string())?;
    load_files_with_key(&state, &raw_key).await
}

async fn load_files_with_key(state: &State<'_, ReceiverState>, raw_key: &[u8]) -> Result<Vec<SharedFileEntry>, String> {
    let shared = connect_shared(raw_key).await.map_err(|e| e.to_string())?;
    let files = fetch_shared_tree(&shared).await.map_err(|e| e.to_string())?;
    *state.sdk.lock().await = Some(Arc::new(shared));
--- .srcBuilders\apps\receiver-micro\src-tauri\src\commands.rs:85
#[tauri::command]
pub async fn cancel_download(state: State<'_, ReceiverState>) -> Result<(), String> {
    state.cancel.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn download_one(rel_path: String, save_to: String, app: AppHandle, state: State<'_, ReceiverState>) -> Result<(), String> {
    let _guard = state.download_gate.try_lock().map_err(|_| "Уже идет скачивание")?;
    state.cancel.store(false, Ordering::Relaxed);
    let sdk = state.sdk.lock().await.clone().ok_or("Клиент не инициализирован")?;
--- .srcBuilders\apps\receiver-micro\src-tauri\src\commands.rs:91
#[tauri::command]
pub async fn download_one(rel_path: String, save_to: String, app: AppHandle, state: State<'_, ReceiverState>) -> Result<(), String> {
    let _guard = state.download_gate.try_lock().map_err(|_| "Уже идет скачивание")?;
    state.cancel.store(false, Ordering::Relaxed);
    let sdk = state.sdk.lock().await.clone().ok_or("Клиент не инициализирован")?;
    let entry = state.files.lock().await.iter().find(|f| f.rel_path == rel_path).cloned().ok_or("Файл не найден")?;
    let cancel = state.cancel.clone();
    download_single_file(&sdk, &entry, &PathBuf::from(save_to), &cancel, |downloaded, total| {
        // троттлинг уже внутри downloader'а — здесь просто emit
        let _ = app.emit("download-progress", DownloadProgress { downloaded, total });
    })
--- .srcBuilders\apps\receiver-micro\src-tauri\src\commands.rs:106
#[tauri::command]
pub async fn download_selected_zip(rel_paths: Vec<String>, save_zip_to: String, app: AppHandle, state: State<'_, ReceiverState>) -> Result<(), String> {
    let _guard = state.download_gate.try_lock().map_err(|_| "Уже идет скачивание")?;
    state.cancel.store(false, Ordering::Relaxed);
    let sdk = state.sdk.lock().await.clone().ok_or("Клиент не инициализирован")?;
    let selected: Vec<SharedFileEntry> = state.files.lock().await.iter()
        .filter(|f| rel_paths.contains(&f.rel_path)).cloned().collect();
    if selected.is_empty() {
        return Err("Не выбрано ни одного файла".into());
    }
    let cancel = state.cancel.clone();
--- .srcBuilders\apps\uploader-desktop\src-tauri\src\commands.rs:39
#[tauri::command]
pub async fn check_saved_auth(state: State<'_, UploaderState>) -> Result<bool, String> {
    let hex_key = match keyring_entry()?.get_password() {
        Ok(k) => k,
        Err(_) => return Ok(false), // ключа нет — покажем экран входа
    };
    let bytes = hex::decode(&hex_key).map_err(|e| e.to_string())?;
    match connect_with_app_key(&bytes).await {
        Ok(Some(sdk)) => { activate_sdk(&state, sdk); Ok(true) }
        // None = ключ отозван/недействителен — нужен повторный approval
        _ => Ok(false),
--- .srcBuilders\apps\uploader-desktop\src-tauri\src\commands.rs:56
#[tauri::command]
pub async fn connect_sia(seed_phrase: Option<String>, app: AppHandle, state: State<'_, UploaderState>) -> Result<(), String> {
    let builder = begin_connection().await.map_err(|e| e.to_string())?;
    let url = builder.response_url().to_string();
    // Открываем системный браузер и дублируем ссылку в UI (вдруг браузер не открылся)
    let _ = app.opener().open_url(&url, None::<&str>);
    let _ = app.emit("auth-url", &url);

    // Ожидание может занять минуты — invoke висит, UI живет на событиях
    let (sdk, generated, app_key) = finish_connection(builder, seed_phrase).await.map_err(|e| e.to_string())?;
    if let Some(phrase) = generated {
--- .srcBuilders\apps\uploader-desktop\src-tauri\src\commands.rs:82
#[tauri::command]
pub async fn start_upload_session(
    session_name: String,
    paths: Vec<String>,
    valid_days: i64,
    state: State<'_, UploaderState>,
) -> Result<String, String> {
    if valid_days <= 0 { return Err("Срок жизни ссылки должен быть >= 1 дня".into()); }
    let sdk = current_sdk(&state)?;

    let mut collected = collect_files_recursive(&paths)?;
--- .srcBuilders\apps\uploader-desktop\src-tauri\src\commands.rs:151
#[tauri::command]
pub async fn generate_share_zip(
    output_zip_path: String,
    sharing_key_bytes: Vec<u8>,
    password: Option<String>,
    app: AppHandle,
) -> Result<(), String> {
    // Имя шаблона зафиксировано mainBinaryName в tauri.conf.json receiver'а
    let template_name = if cfg!(target_os = "windows") { "resources/receiver-micro.exe" } else { "resources/receiver-micro" };
    let template_path = app.path().resolve(template_name, tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Шаблон микро-клиента не найден: {e}"))?;
--- .srcBuilders\apps\uploader-desktop\src-tauri\src\commands.rs:185
#[tauri::command]
pub async fn revoke_session(session_id: String, state: State<'_, UploaderState>) -> Result<(), String> {
    let sdk = current_sdk(&state)?;
    let seed_hex = state.db.session_seed(&session_id).map_err(|e| e.to_string())?.ok_or("У сессии нет ключа")?;
    let seed: [u8; 32] = hex::decode(&seed_hex).map_err(|e| e.to_string())?
        .try_into().map_err(|_| "seed должен быть 32 байта")?;
    revoke_share(&sdk, seed).await.map_err(|e| e.to_string())?;
    state.db.mark_session_revoked(&session_id).map_err(|e| e.to_string())
}

/// Прогресс сессии для инициализации UI сразу после создания/перезапуска.
--- .srcBuilders\apps\uploader-desktop\src-tauri\src\commands.rs:196
#[tauri::command]
pub async fn get_session_progress(session_id: String, state: State<'_, UploaderState>) -> Result<(i64, i64, i64), String> {
    state.db.session_chunk_counts(&session_id).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
--- apps\receiver-micro\src-tauri\src\commands.rs:37
#[tauri::command]
pub async fn init_receiver(state: State<'_, ReceiverState>) -> Result<InitResponse, String> {
    let exe_path = std::env::current_exe().map_err(|e| e.to_string())?;
    match extract_key_from_path(&exe_path) {
        Ok(ExtractedKey::Plain(raw_key)) => Ok(match load_files_with_key(&state, &raw_key).await {
            Ok(files) => InitResponse::Ready(files),
            Err(e) => InitResponse::Error(e),
        }),
        Ok(ExtractedKey::Encrypted(blob)) => {
            *state.encrypted_blob.lock().await = Some(blob);
            Ok(InitResponse::PasswordRequired)
--- apps\receiver-micro\src-tauri\src\commands.rs:55
#[tauri::command]
pub async fn submit_key_payload(payload: String, state: State<'_, ReceiverState>) -> Result<InitResponse, String> {
    match parse_payload(&payload) {
        Ok(ExtractedKey::Plain(raw)) => Ok(match load_files_with_key(&state, &raw).await {
            Ok(files) => InitResponse::Ready(files),
            Err(e) => InitResponse::Error(e),
        }),
        Ok(ExtractedKey::Encrypted(blob)) => {
            *state.encrypted_blob.lock().await = Some(blob);
            Ok(InitResponse::PasswordRequired)
        }
--- apps\receiver-micro\src-tauri\src\commands.rs:70
#[tauri::command]
pub async fn unlock_with_password(password: String, state: State<'_, ReceiverState>) -> Result<Vec<SharedFileEntry>, String> {
    let blob = state.encrypted_blob.lock().await.clone().ok_or("Зашифрованный ключ не найден")?;
    let raw_key = decrypt_with_password(&blob, &password).map_err(|e| e.to_string())?;
    load_files_with_key(&state, &raw_key).await
}

async fn load_files_with_key(state: &State<'_, ReceiverState>, raw_key: &[u8]) -> Result<Vec<SharedFileEntry>, String> {
    let shared = connect_shared(raw_key).await.map_err(|e| e.to_string())?;
    let files = fetch_shared_tree(&shared).await.map_err(|e| e.to_string())?;
    *state.sdk.lock().await = Some(Arc::new(shared));
--- apps\receiver-micro\src-tauri\src\commands.rs:85
#[tauri::command]
pub async fn cancel_download(state: State<'_, ReceiverState>) -> Result<(), String> {
    state.cancel.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn download_one(rel_path: String, save_to: String, app: AppHandle, state: State<'_, ReceiverState>) -> Result<(), String> {
    let _guard = state.download_gate.try_lock().map_err(|_| "Уже идет скачивание")?;
    state.cancel.store(false, Ordering::Relaxed);
    let sdk = state.sdk.lock().await.clone().ok_or("Клиент не инициализирован")?;
--- apps\receiver-micro\src-tauri\src\commands.rs:91
#[tauri::command]
pub async fn download_one(rel_path: String, save_to: String, app: AppHandle, state: State<'_, ReceiverState>) -> Result<(), String> {
    let _guard = state.download_gate.try_lock().map_err(|_| "Уже идет скачивание")?;
    state.cancel.store(false, Ordering::Relaxed);
    let sdk = state.sdk.lock().await.clone().ok_or("Клиент не инициализирован")?;
    let entry = state.files.lock().await.iter().find(|f| f.rel_path == rel_path).cloned().ok_or("Файл не найден")?;
    let cancel = state.cancel.clone();
    download_single_file(&sdk, &entry, &PathBuf::from(save_to), &cancel, |downloaded, total| {
        // троттлинг уже внутри downloader'а — здесь просто emit
        let _ = app.emit("download-progress", DownloadProgress { downloaded, total });
    })
--- apps\receiver-micro\src-tauri\src\commands.rs:106
#[tauri::command]
pub async fn download_selected_zip(rel_paths: Vec<String>, save_zip_to: String, app: AppHandle, state: State<'_, ReceiverState>) -> Result<(), String> {
    let _guard = state.download_gate.try_lock().map_err(|_| "Уже идет скачивание")?;
    state.cancel.store(false, Ordering::Relaxed);
    let sdk = state.sdk.lock().await.clone().ok_or("Клиент не инициализирован")?;
    let selected: Vec<SharedFileEntry> = state.files.lock().await.iter()
        .filter(|f| rel_paths.contains(&f.rel_path)).cloned().collect();
    if selected.is_empty() {
        return Err("Не выбрано ни одного файла".into());
    }
    let cancel = state.cancel.clone();
--- apps\uploader-desktop\src-tauri\src\commands.rs:39
#[tauri::command]
pub async fn check_saved_auth(state: State<'_, UploaderState>) -> Result<bool, String> {
    let hex_key = match keyring_entry()?.get_password() {
        Ok(k) => k,
        Err(_) => return Ok(false), // ключа нет — покажем экран входа
    };
    let bytes = hex::decode(&hex_key).map_err(|e| e.to_string())?;
    match connect_with_app_key(&bytes).await {
        Ok(Some(sdk)) => { activate_sdk(&state, sdk); Ok(true) }
        // None = ключ отозван/недействителен — нужен повторный approval
        _ => Ok(false),
--- apps\uploader-desktop\src-tauri\src\commands.rs:56
#[tauri::command]
pub async fn connect_sia(seed_phrase: Option<String>, app: AppHandle, state: State<'_, UploaderState>) -> Result<(), String> {
    let builder = begin_connection().await.map_err(|e| e.to_string())?;
    let url = builder.response_url().to_string();
    // Открываем системный браузер и дублируем ссылку в UI (вдруг браузер не открылся)
    let _ = app.opener().open_url(&url, None::<&str>);
    let _ = app.emit("auth-url", &url);

    // Ожидание может занять минуты — invoke висит, UI живет на событиях
    let (sdk, generated, app_key) = finish_connection(builder, seed_phrase).await.map_err(|e| e.to_string())?;
    if let Some(phrase) = generated {
--- apps\uploader-desktop\src-tauri\src\commands.rs:82
#[tauri::command]
pub async fn start_upload_session(
    session_name: String,
    paths: Vec<String>,
    valid_days: i64,
    state: State<'_, UploaderState>,
) -> Result<String, String> {
    if valid_days <= 0 { return Err("Срок жизни ссылки должен быть >= 1 дня".into()); }
    let sdk = current_sdk(&state)?;

    let mut collected = collect_files_recursive(&paths)?;
--- apps\uploader-desktop\src-tauri\src\commands.rs:151
#[tauri::command]
pub async fn generate_share_zip(
    output_zip_path: String,
    sharing_key_bytes: Vec<u8>,
    password: Option<String>,
    app: AppHandle,
) -> Result<(), String> {
    // Имя шаблона зафиксировано mainBinaryName в tauri.conf.json receiver'а
    let template_name = if cfg!(target_os = "windows") { "resources/receiver-micro.exe" } else { "resources/receiver-micro" };
    let template_path = app.path().resolve(template_name, tauri::path::BaseDirectory::Resource)
        .map_err(|e| format!("Шаблон микро-клиента не найден: {e}"))?;
--- apps\uploader-desktop\src-tauri\src\commands.rs:185
#[tauri::command]
pub async fn revoke_session(session_id: String, state: State<'_, UploaderState>) -> Result<(), String> {
    let sdk = current_sdk(&state)?;
    let seed_hex = state.db.session_seed(&session_id).map_err(|e| e.to_string())?.ok_or("У сессии нет ключа")?;
    let seed: [u8; 32] = hex::decode(&seed_hex).map_err(|e| e.to_string())?
        .try_into().map_err(|_| "seed должен быть 32 байта")?;
    revoke_share(&sdk, seed).await.map_err(|e| e.to_string())?;
    state.db.mark_session_revoked(&session_id).map_err(|e| e.to_string())
}

/// Прогресс сессии для инициализации UI сразу после создания/перезапуска.
--- apps\uploader-desktop\src-tauri\src\commands.rs:196
#[tauri::command]
pub async fn get_session_progress(session_id: String, state: State<'_, UploaderState>) -> Result<(i64, i64, i64), String> {
    state.db.session_chunk_counts(&session_id).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
```

## sia-core: публичный API

```rust
crates\sia-core\src\crypto.rs:12: pub enum CryptoError {
crates\sia-core\src\crypto.rs:27: pub enum ExtractedKey {
crates\sia-core\src\crypto.rs:40: pub fn build_receiver_filename(
crates\sia-core\src\crypto.rs:66: pub fn extract_key_from_path(exe_path: &Path) -> Result<ExtractedKey, CryptoError> {
crates\sia-core\src\crypto.rs:86: pub fn parse_payload(payload: &str) -> Result<ExtractedKey, CryptoError> {
crates\sia-core\src\crypto.rs:123: pub fn decrypt_with_password(encrypted_blob: &[u8], password: &str) -> Result<Vec<u8>, CryptoError> {
crates\sia-core\src\downloader.rs:20: pub fn calculate_resume_chunk(existing_part_size: u64, chunk_size: u64) -> (usize, u64) {
crates\sia-core\src\downloader.rs:62: pub async fn download_single_file<F>(
crates\sia-core\src\downloader.rs:159: pub async fn download_as_zip<F>(
crates\sia-core\src\lib.rs:5: pub mod crypto;
crates\sia-core\src\lib.rs:6: pub mod downloader;
crates\sia-core\src\lib.rs:7: pub mod models;
crates\sia-core\src\lib.rs:8: pub mod sia_client;
crates\sia-core\src\models.rs:6: pub struct ChunkMetadata {
crates\sia-core\src\models.rs:21: pub struct SharedFileEntry {
crates\sia-core\src\models.rs:37: pub fn sanitize_rel_path(path: &str) -> Option<String> {
crates\sia-core\src\sia_client.rs:13: pub const DEFAULT_INDEXER_URL: &str = "https://sia.storage";
crates\sia-core\src\sia_client.rs:14: pub const DEFAULT_CHUNK_SIZE: u64 = 256 * 1024 * 1024; // 256 МиБ
crates\sia-core\src\sia_client.rs:20: pub const APP_META: AppMetadata = AppMetadata {
crates\sia-core\src\sia_client.rs:30: pub enum SiaClientError {
crates\sia-core\src\sia_client.rs:56: pub fn parse_object_id(s: &str) -> Result<Hash256, SiaClientError> {
crates\sia-core\src\sia_client.rs:63: pub async fn connect_with_app_key(saved_key: &[u8]) -> Result<Option<Sdk>, SiaClientError> {
crates\sia-core\src\sia_client.rs:74: pub async fn begin_connection() -> Result<Builder<RequestingApprovalState>, SiaClientError> {
crates\sia-core\src\sia_client.rs:86: pub async fn finish_connection(
crates\sia-core\src\sia_client.rs:114: pub async fn ensure_capacity(sdk: &Sdk, planned_bytes: u64) -> Result<(), SiaClientError> {
crates\sia-core\src\sia_client.rs:133: pub async fn upload_chunk<F>(
crates\sia-core\src\sia_client.rs:170: pub async fn share_session_objects(
crates\sia-core\src\sia_client.rs:194: pub async fn revoke_share(sdk: &Sdk, seed: [u8; 32]) -> Result<(), SiaClientError> {
crates\sia-core\src\sia_client.rs:201: pub async fn connect_shared(sharing_key_bytes: &[u8]) -> Result<SharedSdk, SiaClientError> {
crates\sia-core\src\sia_client.rs:210: pub async fn fetch_shared_tree(shared: &SharedSdk) -> Result<Vec<SharedFileEntry>, SiaClientError> {
crates\sia-core\src\sia_client.rs:228: pub fn group_summaries_into_tree(items: Vec<(String, Vec<u8>)>) -> Result<Vec<SharedFileEntry>, SiaClientError> {
```

## Фронтенд: invoke / listen / emit

```ts
.srcBuilders\apps\receiver-micro\src\App.svelte:47: await listen<{ downloaded: number; total: number }>('download-progress', (e) => {
.srcBuilders\apps\receiver-micro\src\App.svelte:50: try { applyInit(await invoke<InitResponse>('init_receiver')); }
.srcBuilders\apps\receiver-micro\src\App.svelte:57: files = await invoke<SharedFileEntry[]>('unlock_with_password', { password });
.srcBuilders\apps\receiver-micro\src\App.svelte:65: try { applyInit(await invoke<InitResponse>('submit_key_payload', { payload: manualPayload })); }
.srcBuilders\apps\receiver-micro\src\App.svelte:101: await runDownload('one', () => invoke('download_one', { relPath: entry.rel_path, saveTo }));
.srcBuilders\apps\receiver-micro\src\App.svelte:109: invoke('download_selected_zip', { relPaths: [...selectedPaths], saveZipTo }));
.srcBuilders\apps\receiver-micro\src\App.svelte:157: <button class="danger" on:click={() => invoke('cancel_download')}>Отменить</button>
.srcBuilders\apps\uploader-desktop\src\App.svelte:38: await listen<string>('auth-url', (e) => { authUrl = e.payload; });
.srcBuilders\apps\uploader-desktop\src\App.svelte:39: await listen<string>('auth-phrase', (e) => { backupPhrase = e.payload; });
.srcBuilders\apps\uploader-desktop\src\App.svelte:43: await listen<{ session_id: string; uploaded: number; total: number; failed: number }>(
.srcBuilders\apps\uploader-desktop\src\App.svelte:50: await listen<{ session_id: string; file_id: string; chunk_index: number; fraction: number }>(
.srcBuilders\apps\uploader-desktop\src\App.svelte:57: await listen<{ session_id: string; error: string; failed: boolean }>('chunk-error', (e) => {
.srcBuilders\apps\uploader-desktop\src\App.svelte:62: await listen<[string, number[]]>('session-completed', (e) => {
.srcBuilders\apps\uploader-desktop\src\App.svelte:68: await listen<[string, string]>('session-failed', (e) => {
.srcBuilders\apps\uploader-desktop\src\App.svelte:74: isConnected = await invoke<boolean>('check_saved_auth');
.srcBuilders\apps\uploader-desktop\src\App.svelte:80: invoke('connect_sia', { seedPhrase: newAccount ? null : seedPhrase.trim() || null })
.srcBuilders\apps\uploader-desktop\src\App.svelte:99: activeSessionId = await invoke<string>('start_upload_session', { sessionName, paths, validDays });
.srcBuilders\apps\uploader-desktop\src\App.svelte:101: const [t, u, f] = await invoke<[number, number, number]>(
.srcBuilders\apps\uploader-desktop\src\App.svelte:112: await invoke('generate_share_zip', {
.srcBuilders\apps\uploader-desktop\src\App.svelte:125: try { await invoke('revoke_session', { sessionId: activeSessionId }); revoked = true; }
apps\receiver-micro\src\App.svelte:47: await listen<{ downloaded: number; total: number }>('download-progress', (e) => {
apps\receiver-micro\src\App.svelte:50: try { applyInit(await invoke<InitResponse>('init_receiver')); }
apps\receiver-micro\src\App.svelte:57: files = await invoke<SharedFileEntry[]>('unlock_with_password', { password });
apps\receiver-micro\src\App.svelte:65: try { applyInit(await invoke<InitResponse>('submit_key_payload', { payload: manualPayload })); }
apps\receiver-micro\src\App.svelte:101: await runDownload('one', () => invoke('download_one', { relPath: entry.rel_path, saveTo }));
apps\receiver-micro\src\App.svelte:109: invoke('download_selected_zip', { relPaths: [...selectedPaths], saveZipTo }));
apps\receiver-micro\src\App.svelte:157: <button class="danger" on:click={() => invoke('cancel_download')}>Отменить</button>
apps\uploader-desktop\src\App.svelte:38: await listen<string>('auth-url', (e) => { authUrl = e.payload; });
apps\uploader-desktop\src\App.svelte:39: await listen<string>('auth-phrase', (e) => { backupPhrase = e.payload; });
apps\uploader-desktop\src\App.svelte:43: await listen<{ session_id: string; uploaded: number; total: number; failed: number }>(
apps\uploader-desktop\src\App.svelte:50: await listen<{ session_id: string; file_id: string; chunk_index: number; fraction: number }>(
apps\uploader-desktop\src\App.svelte:57: await listen<{ session_id: string; error: string; failed: boolean }>('chunk-error', (e) => {
apps\uploader-desktop\src\App.svelte:62: await listen<[string, number[]]>('session-completed', (e) => {
apps\uploader-desktop\src\App.svelte:68: await listen<[string, string]>('session-failed', (e) => {
apps\uploader-desktop\src\App.svelte:74: isConnected = await invoke<boolean>('check_saved_auth');
apps\uploader-desktop\src\App.svelte:80: invoke('connect_sia', { seedPhrase: newAccount ? null : seedPhrase.trim() || null })
apps\uploader-desktop\src\App.svelte:99: activeSessionId = await invoke<string>('start_upload_session', { sessionName, paths, validDays });
apps\uploader-desktop\src\App.svelte:101: const [t, u, f] = await invoke<[number, number, number]>(
apps\uploader-desktop\src\App.svelte:112: await invoke('generate_share_zip', {
apps\uploader-desktop\src\App.svelte:125: try { await invoke('revoke_session', { sessionId: activeSessionId }); revoked = true; }
```

## Ссылки / схема / ключи (зацепки)

```
.srcBuilders\crates\sia-core\src\lib.rs:1: //! Общее ядро SiaShare: работа с сетью Sia, криптография ключа доступа,
collect-context.ps1:122: $h = Select-String -LiteralPath $src.FullName -Pattern 'siashare:|build_link|parse_link|fn .*link|fragment|deep-link' | Select-Object -First $MaxHits
crates\sia-core\src\lib.rs:1: //! Общее ядро SiaShare: работа с сетью Sia, криптография ключа доступа,
```

## TODO / FIXME / unimplemented

```
collect-context.ps1:128: Add-Section 'TODO / FIXME / unimplemented'
collect-context.ps1:130: $h = Select-String -LiteralPath $src.FullName -Pattern 'TODO|FIXME|HACK|unimplemented!|todo!\(' | Select-Object -First $MaxHits
```

## git: последние коммиты и статус

```
### log
79933ea ╨Я╨╡╤А╨▓╨░╤П ╤Б╨▒╨╛╤А╨║╨░
d9b80dd Initial commit

### status
?? "Cargo copy.toml"
?? QWEN.md
?? collect-context.ps1
```

## sia_storage 0.13.x: публичные сигнатуры из кэша cargo

### sia_storage-0.13.6
```rust
src\app_client.rs:51: pub enum Error {
src\app_client.rs:94: pub enum PinObjectError {
src\app_client.rs:287: pub struct KeyStats {
src\builder.rs:20: pub struct DisconnectedState;
src\builder.rs:23: pub struct RequestingApprovalState {
src\builder.rs:31: pub struct ApprovedState {
src\builder.rs:38: pub struct Builder<S> {
src\builder.rs:47: pub enum BuilderError {
src\builder.rs:97: pub fn new<U: IntoUrl>(indexer_url: U, app_meta: AppMetadata) -> Result<Self, BuilderError> {
src\builder.rs:114: pub async fn connected(&self, app_key: &AppKey) -> Result<Option<Sdk>, BuilderError> {
src\builder.rs:137: pub async fn connect_pre_authorized(
src\builder.rs:187: pub async fn request_connection(
src\builder.rs:212: pub fn response_url(&self) -> &str {
src\builder.rs:222: pub async fn wait_for_approval(self) -> Result<Builder<ApprovedState>, BuilderError> {
src\builder.rs:255: pub fn reconnecting(&self) -> bool {
src\builder.rs:270: pub async fn matches_existing_app_key(&self, mnemonic: &str) -> Result<bool, BuilderError> {
src\builder.rs:286: pub async fn register(self, mnemonic: &str) -> Result<Sdk, BuilderError> {
src\client.rs:199: pub fn write_estimate(&self, bytes: u32) -> Duration {
src\client.rs:213: pub fn read_estimate(&self, bytes: u32) -> Duration {
src\download.rs:27: pub enum DownloadError {
src\download.rs:550: pub struct Download {
src\download.rs:705: pub async fn write_to_path<P: AsRef<std::path::Path>>(
src\download.rs:728: pub async fn read_chunk(&mut self) -> Result<Vec<u8>, DownloadError> {
src\encryption.rs:12: pub struct EncryptionKey([u8; 32]);
src\encryption.rs:111: pub fn apply_keystream(&mut self, buf: &mut [u8]) {
src\encryption.rs:134: pub fn new_v0(key: EncryptionKey, offset: u64) -> Self {
src\encryption.rs:148: pub fn new_v1(data_key: EncryptionKey, offset: u64, slab_key: &EncryptionKey) -> Self {
src\erasure_coding.rs:13: pub enum Error {
src\erasure_coding.rs:28: pub fn new(data_shards: usize, parity_shards: usize) -> Result<Self> {
src\erasure_coding.rs:34: pub fn data_shards(&self) -> usize {
src\erasure_coding.rs:40: pub fn encode_shards(&self, shards: &mut [Vec<u8>]) -> Result<()> {
src\erasure_coding.rs:46: pub fn reconstruct_data_shards(&self, shards: &mut [Option<Vec<u8>>]) -> Result<()> {
src\erasure_coding.rs:53: pub async fn write_data_shards<W: AsyncWrite + Unpin>(
src\erasure_coding.rs:141: pub fn length(&self) -> usize {
src\erasure_coding.rs:149: pub fn total_length(&self) -> u64 {
src\erasure_coding.rs:154: pub fn optimal_data_size(&self) -> usize {
src\erasure_coding.rs:159: pub fn finish(mut self) -> Option<ReadSlab> {
src\erasure_coding.rs:179: pub async fn read_slab<R: AsyncRead + Unpin>(
src\hosts.rs:35: pub struct Host {
src\hosts.rs:100: pub enum RPCError {
src\hosts.rs:128: pub fn new(transport: Client) -> Self {
src\hosts.rs:151: pub fn prioritize<H, F>(&self, items: &mut Vec<H>, f: F)
src\hosts.rs:173: pub fn update(&self, new_hosts: Vec<Host>, clear: bool) {
src\hosts.rs:205: pub fn available_for_upload(&self) -> usize {
src\hosts.rs:228: pub fn reserve_inflight_download(&self, host_key: &PublicKey) -> Option<InflightGuard> {
src\hosts.rs:241: pub fn with_refresher(
src\hosts.rs:260: pub async fn refresh(&self) -> Result<(), app_client::Error> {
src\hosts.rs:279: pub async fn ensure_upload_hosts(&self, required: usize) -> usize {
src\hosts.rs:317: pub fn upload_queue(&self, pending_initial: usize) -> HostQueue {
src\hosts.rs:321: pub fn write_estimate(&self, bytes: u32) -> Duration {
src\hosts.rs:325: pub fn read_estimate(&self, bytes: u32) -> Duration {
src\hosts.rs:332: pub async fn write_sector(
src\hosts.rs:352: pub async fn read_sector(
src\hosts.rs:394: pub fn add_failure(&self, host_key: PublicKey) {
src\hosts.rs:408: pub fn record_write_sample(&self, host_key: PublicKey, size: u32, elapsed: Duration) {
src\hosts.rs:412: pub fn record_read_sample(&self, host_key: PublicKey, size: u32, elapsed: Duration) {
src\hosts.rs:419: pub enum QueueError {
src\lib.rs:132: pub struct AppMetadata {
src\lib.rs:176: pub struct AppKey(pub(crate) PrivateKey);
src\lib.rs:180: pub fn import(buf: [u8; 32]) -> Self {
src\lib.rs:191: pub fn export(&self) -> [u8; 32] {
src\lib.rs:198: pub fn sign(&self, message: &[u8]) -> Signature {
src\lib.rs:203: pub fn public_key(&self) -> PublicKey {
src\lib.rs:210: pub struct ObjectsCursor {
src\lib.rs:220: pub struct GeoLocation {
src\lib.rs:239: pub struct HostQuery {
src\lib.rs:260: pub struct App {
src\lib.rs:278: pub struct Account {
src\lib.rs:303: pub struct SharingKeyOptions {
src\lib.rs:317: pub struct ShardProgress {
src\lib.rs:327: pub struct DownloadOptions {
src\lib.rs:347: pub fn on_shard_downloaded<F>(mut self, callback: F) -> Self
src\lib.rs:356: pub fn on_shard_downloaded<F>(mut self, callback: F) -> Self
src\lib.rs:367: pub struct UploadOptions {
src\lib.rs:393: pub fn on_shard_uploaded<F>(mut self, callback: F) -> Self
src\lib.rs:402: pub fn on_shard_uploaded<F>(mut self, callback: F) -> Self
src\lib.rs:413: pub fn optimal_data_size(&self) -> usize {
src\lib.rs:418: pub fn slab_size(&self) -> usize {
src\lib.rs:428: pub fn validate(&self) -> Result<(), UploadError> {
src\lib.rs:507: pub struct PackedUploadOptions {
src\lib.rs:539: pub fn optimal_data_size(&self) -> usize {
src\lib.rs:544: pub fn slab_size(&self) -> usize {
src\lib.rs:585: pub fn encoded_size(data_size: u64, data_shards: u8, parity_shards: u8) -> u64 {
src\lib.rs:594: pub fn generate_recovery_phrase() -> String {
src\lib.rs:599: pub fn validate_recovery_phrase(phrase: &str) -> Result<(), SeedError> {
src\mock.rs:15: pub struct MockNetwork {
src\mock.rs:22: pub fn new() -> Self {
src\mock.rs:35: pub fn add_hosts(&self, n: usize) -> Vec<PublicKey> {
src\mock.rs:56: pub fn set_slow_hosts(&self, hosts: impl IntoIterator<Item = PublicKey>, delay: Duration) {
src\mock.rs:61: pub fn reset_slow_hosts(&self) {
src\mock.rs:68: pub fn clear_sectors(&self) {
src\mock.rs:73: pub fn pinned_slabs(&self) -> usize {
src\mock.rs:79: pub fn set_pin_slabs_failures(&self, failures: usize) {
src\mock.rs:85: pub fn pin_slabs_calls(&self) -> usize {
src\mock.rs:90: pub async fn sdk(&self, app_key: AppKey) -> Result<Sdk, BuilderError> {
src\mock.rs:101: pub async fn shared_sdk(&self, seed: [u8; 32]) -> Result<SharedSdk, BuilderError> {
src\object_encryption.rs:18: pub enum DecryptError {
src\preauthorized_keys.rs:11: pub struct PreAuthorizedKeyRequest {
src\preauthorized_keys.rs:41: pub struct SignedPreAuthorizedKeyRequest {
src\preauthorized_keys.rs:60: pub struct PreAuthorizedKey(pub(crate) PrivateKey);
src\preauthorized_keys.rs:70: pub fn new() -> Self {
src\preauthorized_keys.rs:75: pub fn import(buf: [u8; 32]) -> Self {
src\preauthorized_keys.rs:85: pub fn export(&self) -> [u8; 32] {
src\preauthorized_keys.rs:92: pub fn sign(&self, message: &[u8]) -> Signature {
src\preauthorized_keys.rs:97: pub fn public_key(&self) -> PublicKey {
src\preauthorized_keys.rs:102: pub fn sign_request(&self, req: PreAuthorizedKeyRequest) -> SignedPreAuthorizedKeyRequest {
src\rhp4.rs:154: pub enum Error {
src\sdk.rs:34: pub enum Error {
src\sdk.rs:65: pub struct Sdk {
src\sdk.rs:122: pub fn app_key(&self) -> &AppKey {
src\sdk.rs:143: pub async fn upload<R: AsyncRead + Unpin + 'static>(
src\sdk.rs:172: pub async fn upload_path<P: AsRef<Path>>(
src\sdk.rs:190: pub fn upload_packed(&self, options: PackedUploadOptions) -> Result<PackedUpload, UploadError> {
src\sdk.rs:203: pub fn download(
src\sdk.rs:216: pub async fn hosts(&self, query: HostQuery) -> Result<Vec<Host>, Error> {
src\sdk.rs:224: pub async fn account(&self) -> Result<Account, Error> {
src\sdk.rs:235: pub async fn object(&self, key: &Hash256) -> Result<Object, Error> {
src\sdk.rs:253: pub async fn object_events(
src\sdk.rs:291: pub async fn object_event_summaries(
src\sdk.rs:328: pub async fn prune_slabs(&self, before: Option<DateTime<Utc>>) -> Result<(), Error> {
src\sdk.rs:341: pub async fn update_object_metadata(&self, object: &Object) -> Result<(), Error> {
src\sdk.rs:354: pub async fn delete_object(&self, id: &Hash256) -> Result<(), Error> {
src\sdk.rs:369: pub fn object_share_url(
src\sdk.rs:383: pub async fn object_from_share_url<U: IntoUrl>(&self, share_url: U) -> Result<Object, Error> {
src\sdk.rs:395: pub async fn pin_object(&self, object: &Object) -> Result<(), Error> {
src\sdk.rs:429: pub async fn create_sharing_key(
src\sdk.rs:454: pub async fn sharing_keys(
src\sdk.rs:467: pub async fn sharing_key(&self, key: &SharingKey) -> Result<KeyRecord, SharingError> {
src\shared_sdk.rs:40: pub struct SharedSdk {
src\shared_sdk.rs:76: pub async fn connect<U: IntoUrl>(indexer_url: U, seed: [u8; 32]) -> Result<Self, BuilderError> {
src\shared_sdk.rs:156: pub async fn stats(&self) -> Result<KeyStats, Error> {
src\shared_sdk.rs:164: pub async fn object(&self, key: &Hash256) -> Result<Object, Error> {
src\shared_sdk.rs:180: pub async fn objects(
src\shared_sdk.rs:200: pub async fn object_summaries(
src\shared_sdk.rs:218: pub async fn hosts(&self, query: HostQuery) -> Result<Vec<Host>, Error> {
src\shared_sdk.rs:232: pub fn download(
src\sharing.rs:137: pub struct SharingKey(pub(crate) PrivateKey);
src\sharing.rs:142: pub fn import(seed: [u8; 32]) -> Self {
src\sharing.rs:148: pub fn export(&self) -> [u8; 32] {
src\sharing.rs:155: pub fn public_key(&self) -> PublicKey {
src\sharing.rs:172: pub struct KeyRecord {
src\sharing.rs:183: pub enum SharingError {
src\slabs.rs:23: pub struct Sector {
src\slabs.rs:34: pub enum SlabVersion {
src\slabs.rs:83: pub struct Slab {
src\slabs.rs:103: pub fn digest(&self) -> Hash256 {
src\slabs.rs:159: pub enum SealedObjectError {
src\slabs.rs:187: pub struct SealedObject {
src\slabs.rs:284: pub fn id(&self) -> Hash256 {
src\slabs.rs:289: pub fn open(self, app_key: &AppKey) -> Result<Object, SealedObjectError> {
src\slabs.rs:321: pub struct ObjectSummary {
src\slabs.rs:390: pub struct ObjectEvent {
src\slabs.rs:404: pub struct ObjectEventSummary {
src\slabs.rs:420: pub struct Object {
src\slabs.rs:442: pub fn new(metadata: Option<Vec<u8>>) -> Self {
src\slabs.rs:474: pub fn less_safe_new(
src\slabs.rs:489: pub fn id(&self) -> Hash256 {
src\slabs.rs:501: pub fn data_key(&self) -> &EncryptionKey {
src\slabs.rs:506: pub fn slabs(&self) -> &[Slab] {
src\slabs.rs:511: pub fn size(&self) -> u64 {
src\slabs.rs:523: pub fn truncate(&self, length: u64) -> Self {
src\slabs.rs:553: pub fn encoded_size(&self) -> u64 {
src\slabs.rs:560: pub fn seal(&self, app_key: &AppKey) -> SealedObject {
src\upload.rs:472: pub enum UploadError {
src\upload.rs:886: pub struct PackedUpload {
src\upload.rs:908: pub fn remaining(&self) -> u64 {
src\upload.rs:913: pub fn length(&self) -> u64 {
src\upload.rs:918: pub fn optimal_data_size(&self) -> usize {
src\upload.rs:923: pub fn slabs(&self) -> usize {
src\upload.rs:939: pub async fn add<R: AsyncRead + Unpin>(&mut self, r: R) -> Result<u64, UploadError> {
src\upload.rs:951: pub async fn add_path<P: AsRef<Path>>(&mut self, path: P) -> Result<u64, UploadError> {
src\upload.rs:960: pub async fn finalize(self) -> Result<Vec<Object>, UploadError> {
src\client\metrics.rs:313: pub fn estimate_duration(&self, size: u32) -> Duration {
src\client\metrics.rs:323: pub fn pace(self) -> Option<TransferPace> {
src\client\metrics.rs:331: pub fn estimate_duration(&self, size: u32) -> Duration {
src\compat\mod.rs:26: pub struct AbortOnDropHandle<T>(tokio::task::JoinHandle<T>);
src\compat\mod.rs:35: pub fn new(handle: tokio::task::JoinHandle<T>) -> Self {
src\compat\wasm_time.rs:9: pub async fn sleep(duration: Duration) {
src\compat\wasm_time.rs:27: pub struct Elapsed;
src\compat\wasm_time.rs:39: pub async fn timeout<F: Future>(duration: Duration, future: F) -> Result<F::Output, Elapsed> {
src\rhp4\mock.rs:21: pub struct Client {
src\rhp4\mock.rs:43: pub fn new() -> Self {
src\rhp4\mock.rs:58: pub fn clear(&self) {
src\rhp4\mock.rs:64: pub fn set_slow_hosts(&self, hosts: impl IntoIterator<Item = PublicKey>, delay: Duration) {
src\rhp4\mock.rs:72: pub fn reset_slow_hosts(&self) {
src\rhp4\mock.rs:107: pub fn set_read_failures(&self, hosts: impl IntoIterator<Item = PublicKey>, count: usize) {
src\rhp4\mock.rs:116: pub fn set_initial_read_delay(&self, delay: Duration) {
src\rhp4\mock.rs:121: pub fn set_price_delay(&self, delay: Duration) {
src\rhp4\mock.rs:126: pub fn price_requests(&self) -> usize {
src\rhp4\mock.rs:132: pub fn set_price_failures(&self, count: usize) {
src\rhp4\mock.rs:137: pub fn read_requests(&self) -> usize {
src\rhp4\siamux.rs:31: pub enum ConnectError {
src\rhp4\siamux.rs:64: pub struct Client {
src\rhp4\siamux.rs:75: pub fn new() -> Self {
src\rhp4\web_transport.rs:372: pub struct Client {
src\rhp4\web_transport.rs:385: pub fn new() -> Self {
src\sdk\sharing_key.rs:16: pub async fn share_object(
src\sdk\sharing_key.rs:30: pub async fn shared_objects(
src\sdk\sharing_key.rs:47: pub async fn unshare_object(
src\sdk\sharing_key.rs:67: pub async fn revoke_sharing_key(&self, key: &SharingKey) -> Result<(), SharingError> {
```
