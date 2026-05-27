# ConfigPilot

ConfigPilot 是一个基于 Tauri 2、Rust、React 和 Vite 的个人配置文件同步管理系统。它面向 macOS 桌面环境，聚焦同步 zsh 与 Ghostty 配置，通过 GitHub Device Flow 授权后自动创建私有仓库 `configpilot-dotfiles`，并支持本地配置监听、备份、恢复和冲突处理。

## 功能

- 扫描 `~/.zshrc`、`~/.zprofile`、`~/.zshenv`、`~/.config/zsh/`
- 扫描 `~/.config/ghostty/config`、`~/.config/ghostty/`
- GitHub Device Flow 授权
- 自动创建或复用 GitHub 私有仓库 `configpilot-dotfiles`
- 本地 app data 工作区缓存 Git 仓库
- 手动备份、恢复和双向同步
- 自动监听配置文件变化并同步
- 冲突时保留本地和远端副本，不自动覆盖用户配置
- 提供软著申请文档目录

## 运行

首次运行需要准备 GitHub OAuth App 的 Client ID 和 Client Secret。请把 OAuth App 的 callback URL 设置为：

```text
http://127.0.0.1:39119/callback
```

ConfigPilot 浏览器登录时会临时监听 `127.0.0.1:39119` 接收 GitHub 回调。

可以直接在项目根目录创建 `.env`：

```bash
cp .env.example .env
```

然后编辑 `.env`：

```text
CONFIGPILOT_GITHUB_CLIENT_ID="你的 GitHub OAuth Client ID"
CONFIGPILOT_GITHUB_CLIENT_SECRET="你的 GitHub OAuth Client Secret"
```

也可以临时通过终端环境变量启动：

```bash
export CONFIGPILOT_GITHUB_CLIENT_ID="你的 GitHub OAuth Client ID"
export CONFIGPILOT_GITHUB_CLIENT_SECRET="你的 GitHub OAuth Client Secret"
npm install
npm run tauri:dev
```

如果只想检查前端：

```bash
npm install
npm run dev
```

## 同步仓库结构

```text
zsh/
  .zshrc
  .zprofile
  .zshenv
  config-zsh/
ghostty/
  config
  ghostty/
manifest.json
```

## 安全说明

ConfigPilot 第一版不会同步 `.ssh`、Git 凭据、系统 Keychain、浏览器配置等高风险数据。GitHub token 通过系统安全存储保存，macOS 下会进入 Keychain。冲突发生时应用只生成冲突副本并提示选择，不会静默覆盖本地配置。

## 软著材料

软著申请相关文档位于：

```text
docs/software-copyright/
```

包含软件说明书、用户操作手册、系统设计说明书、主要功能模块说明和源代码目录说明。
