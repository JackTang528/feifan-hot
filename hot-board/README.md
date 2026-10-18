# 非凡热点榜

Tauri 2 + React。榜单名单来自 `https://sport.tomist.com/hot-board/boards.json`。内置抓取 Hacker News、微博热搜、B站热门、今日头条热榜；Google 趋势和 Google 新闻是云端配置的 RSS。SQLite 缓存，点击标题用系统浏览器打开。

打包见 [docs/打包流程.md](docs/打包流程.md)。

## 环境

- Node.js 20+
- [Rust](https://www.rust-lang.org/tools/install)（stable）
- Windows：**Visual Studio C++ 生成工具**（「使用 C++ 的桌面开发」），见 [Tauri 前置条件](https://v2.tauri.app/start/prerequisites/)

## 运行

```bash
cd hot-board
npm install
npm run tauri dev
```

首次 `tauri dev` 会编译 Rust，可能需要几分钟。

## 更新检测

设置面板打开时自动检查云端 `latest.json`，有新版本会在齿轮按钮上点红点并给出下载地址。清单地址在 `src-tauri/update_config.url`，写法见 [docs/更新检测.md](docs/更新检测.md)。

## 后续扩展

加减 RSS 榜：改服务器上的 `boards.json`，不用改代码。新的非 RSS 站点要在 `src-tauri/src/sources/` 写适配器，并在 `src-tauri/src/catalog.rs` 的 `builtin()` 里登记。云端地址在 `src-tauri/board_config.url`，改地址要重新打包。
