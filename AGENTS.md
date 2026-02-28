# AGENTS.md - Developer Guide for JLC2KiCad Tauri

## Project Overview
- **Type**: Tauri 2 desktop application (Rust backend + Vanilla JS frontend)
- **Purpose**: Generate KiCad libraries from EasyEDA/JLCPCB components
- **Stack**: Rust, Vite, Vanilla JavaScript
- **Documentation**: https://v2.tauri.app/

---

## Build Commands

### Rust Backend (src-tauri/)
```bash
# Development build
cargo check                    # Quick syntax check
cargo build                   # Debug build
cargo build --release         # Release build

# Run Tauri app
cargo tauri dev               # Development with hot reload
cargo tauri build             # Production build

# With WebKitGTK workaround (Linux NVIDIA + Wayland)
WEBKIT_DISABLE_COMPOSITING_MODE=1 \
WEBKIT_DISABLE_DMABUF_RENDERER=1 \
LIBGL_ALWAYS_SOFTWARE=1 \
cargo tauri dev
```

### Frontend (Root)
```bash
# Development server
npm run dev                   # Vite dev server on port 1420

# Production build
npm run build                 # Builds to dist/

# Preview production build
npm run preview
```

### Full Stack
```bash
# IMPORTANT: After ANY frontend change, first build the frontend:
npm run build

# Then run Tauri dev (will start vite dev server in background)
cargo tauri dev

# Alternative stable dev with software rendering
npm run tauri:dev:stable
```

---

## Project Structure (Official Tauri v2)

This is the **official** Tauri v2 project structure:

```
jlc2kicad-tauri/               # Project root
├── Cargo.toml                  # Workspace config ONLY
├── package.json                # npm config
├── vite.config.js              # Vite config
├── src/                       # Frontend source
│   ├── index.html             # Frontend entry
│   ├── main.js                # Frontend logic
│   └── style.css              # Styles
├── dist/                       # Frontend build output
├── node_modules/               # npm dependencies
└── src-tauri/                 # Rust backend
    ├── Cargo.toml             # Rust crate config (NOT workspace)
    ├── Cargo.lock             # Rust lock file
    ├── build.rs               # Build script
    ├── tauri.conf.json        # Tauri config
    ├── capabilities/          # Tauri permissions
    │   └── default.json
    ├── icons/                 # App icons
    │   └── icon.png
    ├── gen/                   # Generated code
    └── src/                   # Rust source
        ├── main.rs            # Binary entry
        └── lib.rs             # Library entry
```

### Critical Rules

1. **Root Cargo.toml**: Workspace config ONLY
   - MUST contain: `[workspace] members = ["src-tauri"]`
   - MUST NOT contain package metadata

2. **Rust crate in src-tauri/**
   - Cargo.toml has `[package]` and `[dependencies]`
   - NOT in root

3. **Frontend in src/**
   - index.html, main.js, style.css in src/
   - NOT at project root

4. **Build output to dist/**
   - Vite output: `dist/` (at project root)
   - Tauri reads: `frontendDist: "../dist"`

5. **Vite port 1420**
   - Fixed port for Tauri
   - Config: `server.port: 1420` in vite.config.js

---

## Tauri Configuration

### tauri.conf.json (src-tauri/tauri.conf.json)
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "JLC2KiCad",
  "version": "1.0.0",
  "identifier": "com.jlc2kicad.tauri",
  "build": {
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build",
    "devUrl": "http://localhost:1420",
    "frontendDist": "../dist"
  },
  "app": {
    "withGlobalTauri": true,
    "windows": [...],
    "security": {
      "capabilities": ["main-capability"]
    }
  },
  "bundle": {...}
}
```

### Key Points
- `frontendDist`: MUST be `"../dist"` (relative to src-tauri/)
- `identifier`: MUST NOT end with `.app` (conflicts with macOS)
- `capabilities`: References files in `capabilities/` directory

### vite.config.js
```javascript
import { defineConfig } from "vite";
import { resolve } from "path";

export default defineConfig({
  root: "src",
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: false,
    host: "127.0.0.1",
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    outDir: "../dist",
    emptyOutDir: true,
  },
});
```

---

## Capabilities & Permissions

### File: src-tauri/capabilities/default.json
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Main window capability",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "core:event:default",
    "dialog:default",
    {
      "identifier": "dialog:allow-open",
      "allow": [{"path": "**/*"}]
    },
    "shell:allow-open",
    "opener:default"
  ]
}
```

### Important Notes
- Linux does NOT support folder picker. Use `directory: false`
- After modifying capabilities, MUST rebuild: `touch src-tauri/src/main.rs && cargo build`

---

## Code Style Guidelines

### Rust Conventions
- **Imports**: `use crate::types::Foo`, `use serde::{Deserialize, Serialize}`
- **Formatting**: `cargo fmt` before commits (4-space, default)
- **Naming**: PascalCase (structs), snake_case (functions/variables)
- **Error Handling**: Use `thiserror` with `#[error(...)]` macros
- **Types**: Explicit in public APIs, `String`/`&str`, `Option<T>`, `Vec<T>`

### JavaScript Conventions
- ES modules (`import`/`export`)
- No TypeScript (vanilla JS)
- const/let only
- Async/await for Tauri invoke calls

---

## Key Patterns

### Tauri Commands (main.rs)
```rust
#[tauri::command]
async fn my_command(
    args: MyArgs,
    window: tauri::Window,
) -> Result<MyResult, String> {
    window.emit("progress", "...").ok();
    // business logic
}
```

### Frontend Tauri Calls
```javascript
const result = await invoke("command_name", { arg1: value });
```

---

## Common Issues & Solutions

### Linux WebKitGTK Crash (SIGSEGV)
```bash
export WAYLAND_DISPLAY=""
cargo tauri dev
```

### Dialog/File Picker Grayed Out
1. Check capabilities has proper permissions with scope
2. MUST rebuild after changes: `touch src-tauri/src/main.rs && cargo build`
3. Linux: Use `directory: false` (folder picker NOT supported)

### White Screen
1. Run `npm run build` to create dist/
2. Check `frontendDist: "../dist"` in tauri.conf.json

### Bundle Identifier Warning
- NEVER end identifier with `.app` (macOS conflict)
- Use: `com.jlc2kicad.tauri` not `com.jlc2kicad.app`

---

## Dependencies (Key Crates)
- `tauri` v2 - Desktop framework
- `serde`/`serde_json` - Serialization
- `reqwest` - HTTP client
- `tokio` - Async runtime
- `thiserror` - Error definitions
- `regex` - Pattern matching
- `zip` - Archive handling
- `once_cell` - Static initialization

---

## Search API

### SearchResult Struct (src-tauri/src/lib.rs)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,              // 元件编号
    pub name: String,           // 元件名称
    pub description: String,    // 描述（仅描述文本，不含封装/制造商）
    pub package: Option<String>,      // 封装
    pub manufacturer: Option<String>, // 厂家
    pub category: Option<String>,     // 分类
    pub price: Option<String>,        // 价格
    pub stock: Option<String>,        // 库存
}
```

### Frontend Display (main.js)
搜索结果显示格式：
```
[编号] C494551
[名称] SYJ22UF/400V13X20
[描述] 容值:22uF;精度:±20%;额定电压:400V...
[封装|厂家] CAP-TH_BD13.0-P5.00-D1.2-FD | KNSCHA(科尼盛)
[价格|库存] ¥0.0326 | 32000
```

### Input Placeholders (index.html)
- EasyEDA输入框: `placeholder="支持立创商城零件编号(C开头)或元器件型号"`
- LCSC输入框: `placeholder="输入LCSC编号或型号"`

---

## Linting
```bash
cargo clippy        # Lint Rust code
cargo fmt --check   # Check formatting
```

No pre-commit hooks configured.
