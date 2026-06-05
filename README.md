# MarkdownGUI

[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)](https://github.com/Theline233/MarkdownGUI)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](#license)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/Theline233/MarkdownGUI/releases)

MarkdownGUI 是一个跨平台桌面工具，用于将 Excel、PDF、Word 等文档解析为 Markdown，并提供清洗、编辑、导出、AI 分析和自动化推送能力。

## 功能特性
- Excel / 文档解析与清洗，支持表格预处理、空行空列过滤和 Markdown 输出
- 双模编辑器，支持 Monaco 编辑器和 Markdown 渲染预览
- AI 智能分析，支持 DeepSeek、OpenAI、Gemini、硅基流动和自定义兼容接口
- 批量导出，支持 ZIP 打包导出 Markdown、HTML、JSON
- Webhook 自动推送，解析完成后可静默 POST 到指定地址
- 自动更新，基于 Tauri updater 检查并安装新版本
- 中英文国际化
- 暗黑模式与主题切换

## 技术栈

- 前端：Vue 3 + Element Plus + Monaco Editor
- 后端：Tauri (Rust)
- 解析引擎：Python (markitdown + pandas + openpyxl)
- 打包：PyInstaller Sidecar

## 安装与使用

### 下载安装包
前往 [GitHub Releases](https://github.com/Theline233/MarkdownGUI/releases) 下载适合当前系统的安装包。
支持平台：
- Windows
- macOS Apple Silicon
- Linux

### 开发环境启动
请先安装 Node.js、Rust、Python 以及 Tauri 所需的系统依赖。
```bash
npm install
npm run tauri dev
```

## 构建

```bash
npm run tauri build
```

构建桌面端时，Python 解析引擎会作为 sidecar 一起打包。发布流水线会使用 PyInstaller 生成对应平台的 `engine-server` 可执行文件。

## 配置说明

### AI API Key

打开应用左下角设置面板，进入「AI 模型配置」：

1. 选择模型供应商
2. 填写对应供应商的 API Key
3. 使用「检测可用模型」获取模型列表，或手动填写模型 ID
4. 保存设置

API Key 按供应商分别保存，切换供应商时不会覆盖其他供应商的密钥。

### 支持的模型供应商

- DeepSeek
- OpenAI
- Gemini
- 硅基流动
- 自定义兼容 OpenAI API 的服务

## License

MIT

---

# MarkdownGUI

[![Version](https://img.shields.io/badge/version-1.0.0-blue.svg)](https://github.com/Theline233/MarkdownGUI)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](#license)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/Theline233/MarkdownGUI/releases)

MarkdownGUI is a cross-platform desktop tool for converting Excel, PDF, Word, and other documents into Markdown, with cleaning, editing, exporting, AI analysis, and automation features.

## Features

- Excel and document parsing with table cleaning, empty row / column filtering, and Markdown output
- Dual-mode editor with Monaco Editor and rendered Markdown preview
- AI analysis with DeepSeek, OpenAI, Gemini, SiliconFlow, and custom compatible providers
- Batch export to ZIP, supporting Markdown, HTML, and JSON
- Webhook auto-push after successful parsing
- Automatic updates powered by Tauri updater
- Chinese and English internationalization
- Dark mode and theme switching

## Tech Stack

- Frontend: Vue 3 + Element Plus + Monaco Editor
- Backend: Tauri (Rust)
- Parsing engine: Python (markitdown + pandas + openpyxl)
- Packaging: PyInstaller Sidecar

## Installation and Usage

### Download

Download the installer for your system from [GitHub Releases](https://github.com/Theline233/MarkdownGUI/releases).

Supported platforms:

- Windows
- macOS Apple Silicon
- Linux

### Development

Install Node.js, Rust, Python, and the required Tauri system dependencies first.

```bash
npm install
npm run tauri dev
```

## Build

```bash
npm run tauri build
```

For desktop builds, the Python parsing engine is packaged as a sidecar. The release workflow uses PyInstaller to generate the platform-specific `engine-server` executable.

## Configuration

### AI API Key

Open the settings panel from the bottom-left corner and go to "AI Model Configuration":

1. Select a provider
2. Enter the provider API Key
3. Use "Detect Available Models" to load models, or enter a model ID manually
4. Save settings

API Keys are stored per provider, so switching providers will not overwrite keys for other providers.

### Supported Providers

- DeepSeek
- OpenAI
- Gemini
- SiliconFlow
- Custom OpenAI-compatible services

## License

MIT
