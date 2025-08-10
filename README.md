# th - Teleport Helper

A modern CLI tool for managing Teleport logins and cloud resources, written in Rust.

## ✨ Features

- 🔐 **AWS Login** - Interactive role selection and elevation
- ☸️ **Kubernetes Access** - Cluster login with privilege escalation  
- 🗄️ **Database Connections** - RDS and MongoDB with proxy tunnels
- 🏗️ **Terraform Integration** - Quick Terragrunt authentication
- 📦 **Auto-Updates** - Seamless background updates from GitHub
- 🎨 **Beautiful UI** - Styled terminal interface with animations

## 🚀 Installation

### macOS & Linux (Bash)
```bash
curl -sSL https://raw.githubusercontent.com/nikomain/th-rust/main/install.sh | bash
```

### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/nikomain/th-rust/main/install.ps1 | iex
```

### Manual Installation
Download the appropriate binary from [releases](https://github.com/nikomain/th-rust/releases):
- **macOS ARM64**: `th-aarch64-apple-darwin`
- **macOS Intel**: `th-x86_64-apple-darwin`  
- **Linux x64**: `th-x86_64-unknown-linux-gnu`
- **Linux ARM64**: `th-aarch64-unknown-linux-gnu`
- **Windows x64**: `th-x86_64-pc-windows-msvc.exe`
- **Windows ARM64**: `th-aarch64-pc-windows-msvc.exe`

## 📖 Usage

### Basic Commands
```bash
th          # Show help
th a        # AWS login (interactive)
th k        # Kubernetes login  
th d        # Database connections
th t        # Terraform login
th l        # Logout/cleanup
```

### AWS Examples
```bash
th a                    # Interactive AWS login
th a dev                # Quick login to dev environment
th a prod s             # Login to prod with sudo role
```

### Kubernetes Examples  
```bash
th k                    # Interactive cluster selection
th k staging            # Quick login to staging cluster
```

### Database Examples
```bash
th d                    # Interactive database selection
th d prod-db            # Connect to specific database
```

### Updates
```bash
th update              # Update to latest version
th changelog           # View recent changes
```

## 🔧 Setup

### Shell Integration (macOS/Linux)
Add to your `~/.zshrc` or `~/.bash_profile`:
```bash
export PATH="$HOME/.local/bin:$PATH"
source $HOME/.local/bin/th.sh
```

### Windows
The installer automatically adds `th` to your PATH. Restart your terminal after installation.

## 🏗️ Development

### Building from Source
```bash
git clone https://github.com/nikomain/th-rust.git
cd th-rust
cargo build --release
```

### Cross-compilation Targets
- `x86_64-apple-darwin` (macOS Intel)
- `aarch64-apple-darwin` (macOS ARM)
- `x86_64-unknown-linux-gnu` (Linux x64)
- `aarch64-unknown-linux-gnu` (Linux ARM64)
- `x86_64-pc-windows-msvc` (Windows x64)
- `aarch64-pc-windows-msvc` (Windows ARM64)

## 📋 Requirements

- **Teleport CLI** (`tsh`) - Must be installed and configured
- **Network Access** - To Teleport proxy and GitHub (for updates)

## 🔄 Migration from Bash Version

`th` maintains complete compatibility with the original bash implementation:
- All commands work identically (`th a dev s`, `th k staging`, etc.)
- Credential sourcing works the same way
- Same configuration files and environment variables

## 📦 Auto-Updates

`th` automatically checks for updates daily and displays notifications after command execution. Updates are:
- ✅ **Non-intrusive** - Notifications appear at the end of commands
- ✅ **Secure** - Downloads verified releases from GitHub
- ✅ **Atomic** - Updates complete successfully or roll back
- ✅ **Optional** - You control when to update

## 🐛 Troubleshooting

### Common Issues
- **"tsh not found"** - Install Teleport CLI
- **"Permission denied"** - Run with appropriate permissions
- **"No updates available"** - You're on the latest version

### Getting Help
- Run `th` for built-in help
- Check [issues](https://github.com/nikomain/th-rust/issues) for known problems
- Submit bug reports with `th version` output

## 📄 License

MIT License - see LICENSE file for details.