# dfm - Dotfile Manager

使用 bare git repository 管理 dotfiles，灵感来源于 [Atlassian 的 dotfiles 教程](https://www.atlassian.com/en/git/tutorials/dotfiles)。

## 功能

- `--init` - 在 `$HOME` 初始化一个新的 dotfile 仓库
- `--clone <url> [branch]` - 克隆已有的 dotfile 仓库
- `-H|--hide <dir> [no]` - 隐藏目录下的 dotfiles（仅 Windows）
- `lz|lazy|lazygit` - 在 dotfile 环境下启动 lazygit
- 其他命令直接传递给 git

## 从源码构建

需要 **Rust nightly** 以获得最小的二进制文件

```bash
git clone https://github.com/AzureZee/dotfile_manager.git
cd dotfile_manager
cargo +nightly install --path .
```

## 使用示例

### 初始化新的 dotfile 仓库

```bash
dfm --init
```

这会在 `$HOME/.cfg` 创建一个 bare git repository，
并在 `$HOME` 创建 `.gitignore` 文件。

### 克隆现有的 dotfile 仓库

```bash
dfm --clone https://github.com/owner/dotfiles.git

# 指定分支会在clone后自动checkout
dfm --clone https://github.com/owner/dotfiles.git linux
```

### 管理 dotfiles

```bash
dfm add .bashrc
dfm commit -m "update bashrc"
dfm push

# 使用 lazygit 可视化管理
dfm lazy
```

### 隐藏 dotfiles（仅 Windows）

```bash
dfm --hide ~     # 隐藏 home 下的 dotfiles
dfm --hide ~ no  # 取消隐藏
```
