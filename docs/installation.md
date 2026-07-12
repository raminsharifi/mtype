# Installation

[Documentation index](README.md) · [CLI reference](cli-reference.md) · [Troubleshooting](troubleshooting.md)

## Supported release binaries

The GitHub release workflow currently publishes these executables:

| Platform | Architecture | Asset |
| --- | --- | --- |
| macOS | Apple Silicon / ARM64 | `mtype-macos-arm64` |
| Linux | x86_64 | `mtype-linux-x86_64` |

There is no prebuilt Intel macOS binary. Other platforms can build from source
when Rust and the platform dependencies required by mtype's crates are
available.

Release binaries are single files. The browser dashboard HTML, CSS, JavaScript,
English word lists, and English quotes are compiled into the executable, so no
runtime asset folder is required.

## Install the macOS Apple Silicon release

These commands download the newest published ARM64 binary, make it executable,
remove the quarantine attribute added by macOS, and install it on the system
PATH:

```sh
curl -L -o mtype https://github.com/raminsharifi/mtype/releases/latest/download/mtype-macos-arm64
chmod +x mtype
xattr -d com.apple.quarantine mtype
sudo mv mtype /usr/local/bin/mtype
mtype --version
```

### Why `xattr` is needed

The release binary is not code signed or notarized. Browsers normally mark a
download with `com.apple.quarantine`; Gatekeeper may then refuse the first
launch. Removing that attribute allows the executable to run. If you prefer a
GUI flow, find the binary in Finder, right-click it, choose Open, and confirm.

`xattr` can print `No such xattr` when the file was not quarantined. In that
case, continue with the remaining commands.

### Install without `sudo`

Use a user-owned binary directory:

```sh
mkdir -p "$HOME/.local/bin"
mv mtype "$HOME/.local/bin/mtype"
```

Add it to zsh:

```sh
echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.zshrc"
source "$HOME/.zshrc"
mtype --version
```

## Install the Linux x86_64 release

```sh
curl -L -o mtype https://github.com/raminsharifi/mtype/releases/latest/download/mtype-linux-x86_64
chmod +x mtype
sudo mv mtype /usr/local/bin/mtype
mtype --version
```

For a user-only installation, move the binary to `~/.local/bin` and ensure that
directory appears in PATH:

```sh
mkdir -p "$HOME/.local/bin"
mv mtype "$HOME/.local/bin/mtype"
export PATH="$HOME/.local/bin:$PATH"
mtype --version
```

Place the PATH export in the startup file for your shell to make it persistent.
Typical files are `~/.bashrc`, `~/.zshrc`, or the configuration file used by
your shell.

## Install from source

### 1. Install Rust

The standard Rust installer is rustup:

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Follow the installer prompts, then open a new terminal or load Cargo's
environment file:

```sh
source "$HOME/.cargo/env"
rustc --version
cargo --version
```

### 2. Install mtype from GitHub

```sh
cargo install --git https://github.com/raminsharifi/mtype
```

Cargo places the executable at `~/.cargo/bin/mtype`. Rustup normally adds that
folder to PATH. Verify both the executable and CLI parser:

```sh
mtype --version
mtype --help
```

### Build a checked-out repository

```sh
git clone https://github.com/raminsharifi/mtype
cd mtype
cargo build --release
./target/release/mtype --version
./target/release/mtype
```

The optimized executable is `target/release/mtype`. `cargo run --release` builds
and launches it in one command.

## Update mtype

### Prebuilt installation

Repeat the platform download commands and replace the installed executable. The
binary and user data are separate; replacing the executable does not remove
config, results, analytics, presets, themes, or synced content.

### Cargo installation

```sh
cargo install --git https://github.com/raminsharifi/mtype --force
```

## Uninstall mtype

Remove the executable from whichever directory was used:

```sh
sudo rm /usr/local/bin/mtype
```

or:

```sh
rm "$HOME/.local/bin/mtype"
```

For a Cargo installation:

```sh
cargo uninstall mtype
```

Uninstalling the executable does not delete user data. See
[Data and privacy](data-and-privacy.md) for the platform paths and reset rules.

## Post-installation checks

```sh
mtype --version
mtype --help
mtype
```

If any check fails, use the relevant section of
[Troubleshooting](troubleshooting.md). Continue with [Usage](usage.md) after the
default test opens successfully.
