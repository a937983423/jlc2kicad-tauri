# JLC2KiCad Tauri 项目初始化与交接记录

更新时间：2026-02-27

## 1. 项目目标
- 提供 Tauri 2 桌面工具，从 EasyEDA / 立创商城 / 本地库导入器件并导出 KiCad9 可用符号、封装、3D。
- 支持网络代理配置（主要用于 EasyEDA 访问）。
- 支持本地离线导入（支持 `.elibz/.elibz2`）。

## 2. 当前可用功能（已实现）
- EasyEDA 搜索与导出（符号/封装/3D）。
- 立创商城搜索入口（含多级回退）。
- 本地文件导入：
  - 支持文件或文件夹选择。
  - 支持递归扫描。
  - 支持格式：`json/txt/csv/tsv/eda/lcsc/elibz/elibz2`。
- 本地离线转换：
  - `.elibz` 解析 `device.json + FOOTPRINT/*.efoo + SYMBOL/*.esym`。
  - `.elibz2` 解析 `device2.json + *.elibu`（从事件流重建 symbol/footprint 图元）。
  - 离线生成 `.kicad_mod` / `.kicad_sym`。
  - 3D 在离线模式下仅使用本地已有 `step/stp/wrl`（不强制联网）。

## 3. 项目结构（Tauri v2 标准结构）

```
jlc2kicad-tauri/               # 项目根目录（前端 + workspace）
├── index.html                  # 前端入口
├── main.js                     # 前端逻辑
├── style.css                   # 样式文件
├── package.json                # npm 配置
├── vite.config.js              # Vite 配置
├── Cargo.toml                  # Workspace 配置
├── PROJECT_INIT.md             # 本文档
├── dist/                       # 前端构建产物
├── node_modules/                # npm 依赖
└── src-tauri/                  # Rust 后端（Tauri 2 标准结构）
    ├── Cargo.toml              # Rust 依赖配置
    ├── build.rs                # 构建脚本
    ├── tauri.conf.json         # Tauri 配置
    ├── capabilities/            # Tauri 权限配置
    │   └── main.json
    ├── icons/                  # 应用图标
    │   └── icon.png
    ├── gen/                    # Tauri 生成的代码
    └── src/                    # Rust 源码（模块化结构）
        ├── lib.rs              # 库入口 + 主要业务逻辑
        ├── main.rs             # Tauri 命令入口
        ├── types.rs            # 数据结构定义
        ├── network.rs          # 网络设置
        └── helpers.rs          # 辅助函数
```

## 4. 运行与构建

### 开发模式（热更新）
```bash
# 方式1：直接运行（推荐）
cargo tauri dev

# 方式2：使用软件渲染（如果遇到 WebKitGTK 崩溃）
WEBKIT_DISABLE_COMPOSITING_MODE=1 \
WEBKIT_DISABLE_DMABUF_RENDERER=1 \
LIBGL_ALWAYS_SOFTWARE=1 \
cargo tauri dev

# 方式3：仅运行前端（独立调试）
npm run dev
# 访问 http://localhost:1420
```

### 生产构建
```bash
# 构建前端
npm run build

# 构建 Tauri 应用
cargo tauri build

# Rust 检查
cargo check
```

### 注意事项
- 权限变更后需要重启 `cargo tauri dev`。
- 前端文件修改后会自动热更新（无需重启）。
- 修改 Rust 代码后需要重新编译（自动触发）。

## 5. 本地离线解析实现要点
- 本地扫描入口：
  - `load_local_folder(path)`：读取并展示本地器件列表。
  - `convert_local_folder(path, ...)`：批量转换。
- `.elibz` 解析：
  - 解包并读取 `device.json`。
  - 读取 `FOOTPRINT/*.efoo` 与 `SYMBOL/*.esym` 的 dataStr。
  - 设备显示优先取可读名称，避免 UUID 直出。
  - 封装信息优先取 `device.footprint.display_title`，其次按 footprint UUID 反查标题。

## 6. 与 Python 参考实现对齐点
- 参考：
  - `../jlc-kicad-lib-loader-1.0.6-pcm/plugins/component_loader.py`
  - `../jlc-kicad-lib-loader-1.0.6-pcm/plugins/easyeda_lib_loader.py`
  - `../jlc-kicad-lib-loader-1.0.6-pcm/plugins/decryptor.py`
- 已对齐内容：
  - EasyEDA Pro 搜索链路（含 `searchByCodes` / `devices/search`）。
  - LCSC 在 Python 插件中的"lcsc facet"思路（`uid/path=lcsc`）已引入。
  - `.elibz` 结构读取方式已对齐（`device.json + efoo + esym`）。

## 7. 已知限制
- 立创官方 OpenAPI 需要申请 key/secret；无 key 时可用公开接口/回退方案，但稳定性受限。
- 离线 `.elibz` 3D 通常不内置，需本地已有模型文件或在线下载。
- `.elibz2` 的 `.elibu` 事件模型较新，当前已覆盖常用图元（PIN/RECT/ELLIPSE/POLY/PAD/FILL），极少数特殊图元仍可能需要增量适配。
- **Linux WebKitGTK 崩溃问题**：在 NVIDIA GPU + Wayland 环境下可能出现 Segmentation Fault。可使用软件渲染或 X11 会话解决。

## 8. 下次接手建议顺序
1. `cargo check` 确认后端状态。
2. `npm run build` 确认前端产物更新。
3. 用一个已知 `.elibz` 回归验证：
   - 本地列表是否显示可读 `C` 编号、名称、封装、制造商。
   - 导出符号/封装是否成功。
4. 若用户反馈字段异常，优先打印并对照 `device.json` 实际结构补映射。

## 9. 变更原则（重要）
- EasyEDA 在线流程和本地离线流程是两条路径，修改时避免相互破坏。
- 前端入口变更后同步检查 Tauri command 是否仍注册。
- 涉及权限（dialog/event）修改时，同时更新两个 capability 文件。
- **模块化**：Rust 代码已模块化到 `src-tauri/src/` 目录下，修改时只需重新编译相关模块。

## 10. 热更新开发说明

### 前端热更新
- 修改 `index.html`, `main.js`, `style.css` 后会自动热更新，无需重启。
- 运行 `npm run dev` 可以独立调试前端。

### Rust 热更新
- 修改 Rust 代码后，`cargo tauri dev` 会自动重新编译并热更新。
- 如果仅修改 `lib.rs` 中的业务逻辑，不需要重启整个应用。

### 常见问题
- **白屏**：检查 `dist/` 目录是否存在，确保 `npm run build` 已执行。
- **WebKitGTK 崩溃**：设置环境变量 `WEBKIT_DISABLE_COMPOSITING_MODE=1` 或使用 X11。
- **权限错误**：检查 `capabilities/main.json` 配置，重启开发服务器。
