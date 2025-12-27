# cloudflare-speed-cli

[![Rust](https://img.shields.io/badge/rust-1.81+-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-GPLv3-blue.svg)](LICENSE)

A CLI tool that displays network speed test results from Cloudflare's speed test service in a TUI interface.

## Features

- **Interactive TUI**: Real-time charts and statistics similar to `btop`
- **Speed Tests**: Measures download/upload throughput, idle latency, and loaded latency
- **History**: View and manage past test results
- **Export**: Save results as JSON
- **Text/JSON Modes**: Headless operation for scripting

## Installation

### Linux Package Managers

#### Debian/Ubuntu (DEB)

```bash
# Download the .deb package from GitHub Releases
wget https://github.com/kavehtehrani/cloudflare-speed-cli/releases/download/v0.1.0/cloudflare-speed-cli_0.1.0_amd64.deb

# Install using apt/dpkg
sudo apt install ./cloudflare-speed-cli_0.1.0_amd64.deb
# or
sudo dpkg -i cloudflare-speed-cli_0.1.0_amd64.deb
```

#### Fedora/RHEL/CentOS (RPM)

```bash
# Download the .rpm package from GitHub Releases
wget https://github.com/kavehtehrani/cloudflare-speed-cli/releases/download/v0.1.0/cloudflare-speed-cli-0.1.0-1.x86_64.rpm

# Install using dnf/yum
sudo dnf install ./cloudflare-speed-cli-0.1.0-1.x86_64.rpm
# or
sudo yum install ./cloudflare-speed-cli-0.1.0-1.x86_64.rpm
```

### macOS

#### Homebrew

```bash
# Install directly from the main repository
brew install kavehtehrani/cloudflare-speed-cli/cloudflare-speed-cli
```

The Homebrew formula is automatically updated when new releases are published.

### Direct Download (All Platforms)

Download pre-built binaries from [GitHub Releases](https://github.com/kavehtehrani/cloudflare-speed-cli/releases):

```bash
# Linux (x86_64)
wget https://github.com/kavehtehrani/cloudflare-speed-cli/releases/download/v0.1.0/cloudflare-speed-cli-x86_64-unknown-linux-gnu-v0.1.0.tar.gz
tar -xzf cloudflare-speed-cli-x86_64-unknown-linux-gnu-v0.1.0.tar.gz
sudo mv cloudflare-speed-cli/usr/local/bin/

# macOS (Intel)
wget https://github.com/kavehtehrani/cloudflare-speed-cli/releases/download/v0.1.0/cloudflare-speed-cli-x86_64-apple-darwin-v0.1.0.tar.gz
tar -xzf cloudflare-speed-cli-x86_64-apple-darwin-v0.1.0.tar.gz
sudo mv cloudflare-speed-cli/usr/local/bin/

# macOS (Apple Silicon)
wget https://github.com/kavehtehrani/cloudflare-speed-cli/releases/download/v0.1.0/cloudflare-speed-cli-aarch64-apple-darwin-v0.1.0.tar.gz
tar -xzf cloudflare-speed-cli-aarch64-apple-darwin-v0.1.0.tar.gz
sudo mv cloudflare-speed-cli/usr/local/bin/
```

### From Source (Cargo)

```bash
cargo install --git https://github.com/kavehtehrani/cloudflare-speed-cli --features tui
```

### AppImage (Universal Linux - No Installation Required)

Download the AppImage from [GitHub Releases](https://github.com/kavehtehrani/cloudflare-speed-cli/releases), make it executable, and run:

```bash
chmod +x cloudflare-speed-cli-x86_64.AppImage
./cloudflare-speed-cli-x86_64.AppImage
```

## Usage

Run with the TUI (default):

```bash
cloudflare-speed-cli
```

Text output mode:

```bash
cloudflare-speed-cli --text
```

JSON output mode:

```bash
cloudflare-speed-cli --json
```

## TUI Controls

### Dashboard Tab
- `q` / `Ctrl-C`: Quit
- `r`: Rerun test
- `p`: Pause/Resume
- `s`: Save JSON to auto-save location
- `a`: Toggle auto-save
- `tab`: Switch tabs (Dashboard, History, Help)
- `?`: Show help

### History Tab
- `↑/↓` or `j/k`: Navigate through historical runs
- `e`: Export selected run as JSON (to current directory)
- `c`: Export selected run as CSV (to current directory)
- `y`: Copy last exported file path to clipboard
- `d`: Delete selected history item
- `tab`: Switch tabs

### Export Options

- **`s` (Save JSON)**: On dashboard, saves the current/last run to the default auto-save location
- **`e` (Export JSON)**: In history tab, exports the selected historical run as JSON with full absolute path shown
- **`c` (Export CSV)**: In history tab, exports the selected historical run as CSV with full absolute path shown
- **`y` (Yank/Copy)**: In history tab, copies the last exported file's absolute path to clipboard

The exported files are saved to the current working directory with filenames based on the test timestamp and measurement ID. The full absolute path is displayed and can be copied to clipboard.

## Source

Uses endpoints from https://speed.cloudflare.com/
