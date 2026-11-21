# Contributing to Inerium Shell

Thank you for your interest in contributing to Inerium Shell! We welcome issues, bug reports, and pull requests.

## 1. Development Prerequisites

Inerium Shell runs on Arch Linux / CachyOS. Ensure you have the core build toolchain and GTK4 / Layer-Shell runtime libraries installed:

```bash
sudo pacman -Syu --needed \
    base-devel git rust pkgconf clang mold \
    gtk4 gtk4-layer-shell vte4 \
    gstreamer gstreamer-vaapi gst-plugins-base gst-plugins-good \
    pango cairo gdk-pixbuf2 glib2 lua python
```

## 2. Repository Architecture

- `crates/`: Unified Rust workspace containing system daemons, IPC bridges, and GTK4/Relm4 layer-shell surfaces.
- `config/`: Desktop user configurations (`~/.config/` targets: Hyprland, Waybar, Rofi, Kitty, Fish, GTK, SwayOSD).
- `local/bin/`: User CLI tools (`~/.local/bin/` targets without `.sh` / `.py` suffixes).
- `system/`: System-level units, udev rules, sudoers templates, and modular add-ons (`camera`, `msi-cooling`, `novpn`).
- `scripts/install/`: Modular installer phases executed by `install.sh`.

## 3. Local Verification Protocol

Before submitting a Pull Request, run the full validation suite locally:

```bash
# 1. Rust workspace check, clippy lint, and tests
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# 2. Shell and Python syntax validation
bash -n *.sh scripts/install/*.sh
luac -p config/hypr/*.lua config/hypr/config/*.lua

for f in local/bin/*; do
    [[ -f "$f" ]] || continue
    if head -n 1 "$f" | grep -qE "bash|sh"; then
        bash -n "$f"
    elif head -n 1 "$f" | grep -q "python"; then
        python3 -m py_compile "$f"
    fi
done

# 3. Dry-run installer validation
./install.sh --dry-run
./install.sh --from-source --dry-run
```

## 4. Commit Conventions

We follow the [Conventional Commits](https://www.conventionalcommits.org/) specification:

- `feat(scope): ...` for new features or capabilities.
- `fix(scope): ...` for bug fixes.
- `refactor(scope): ...` for code reorganization without behavior changes.
- `chore(scope): ...` for dependencies, workflows, or metadata updates.
- `docs(scope): ...` for documentation updates.

Scopes should reflect the modified component (e.g., `shell`, `tools`, `polkit`, `install`, `hypr`, `waybar`).

## 5. Security & Privacy Guardrails

- Never commit personal credentials, access tokens, API keys, or private configs (`rclone.conf`, `*.token`, `amnezia*.conf`).
- Ensure all scripts and system utilities remain strictly user-agnostic: avoid hardcoding user names or home directory paths.
