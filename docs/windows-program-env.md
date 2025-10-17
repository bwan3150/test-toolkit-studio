# Windows 构建环境配置

## 安装步骤

### 1. Node.js
- 下载：https://nodejs.org/
- 运行安装程序，勾选 **"Add to PATH"**

### 2. Rust
- 下载：https://www.rust-lang.org/tools/install
- 运行 `rustup-init.exe`
- 选择默认安装（输入 1 回车）

### 3. Visual Studio Build Tools（必需）
- 下载：https://visualstudio.microsoft.com/downloads/
- 找到 **"Build Tools for Visual Studio"** 下载
- 运行安装程序
- 勾选：**"使用 C++ 的桌面开发"** 或 **"C++ 生成工具"**
- 点击安装

### 4. Python
- 下载：https://www.python.org/downloads/
- 运行安装程序
- **必须勾选：** "Add Python to PATH"
- 点击 Install Now

### 5. uv（Python 包管理器）
打开命令提示符（CMD）运行：
```cmd
pip install uv
```

## 验证安装

运行环境检查脚本：
```cmd
check-env.bat
```

如果所有工具都显示 ✓，说明环境配置成功。

## 开始构建

```cmd
make-win.bat
```

## 输出位置

构建完成后，安装包在 `dist\` 目录：
- `Test Toolkit Studio Setup x.x.x.exe`
