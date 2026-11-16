#!/usr/bin/env python3
"""
inerium-shell/scripts/install/manifest.py
Installation manifest state manager, garbage pruner, and uninstaller.
"""

import argparse
import datetime
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path


def is_inerium_target(path_str: str, repo_dir: str) -> bool:
    """Check if a symlink target points inside the inerium/kanagawa repository or directory."""
    try:
        raw_target = os.readlink(path_str)
    except OSError:
        return False
    if "inerium" in raw_target.lower() or "kanagawa" in raw_target.lower():
        return True
    abs_target = os.path.abspath(os.path.join(os.path.dirname(path_str), raw_target))
    abs_repo = os.path.abspath(repo_dir)
    return abs_target.startswith(abs_repo)


def remove_file_or_link(path_str: str, sudo: bool = False, dry_run: bool = False) -> bool:
    """Safely remove a file or symlink (optionally using sudo)."""
    p = Path(path_str)
    if not p.is_symlink() and not p.exists():
        return False

    if dry_run:
        prefix = "sudo rm -rf" if sudo else "rm -rf"
        print(f"[DRY-RUN] {prefix} {path_str}")
        return True

    try:
        if sudo:
            subprocess.run(["sudo", "rm", "-rf", path_str], check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        else:
            if p.is_symlink() or p.is_file():
                p.unlink(missing_ok=True)
            elif p.is_dir():
                shutil.rmtree(path_str, ignore_errors=True)
        return True
    except Exception as e:
        print(f"[inerium-shell] Warning: failed to remove {path_str}: {e}", file=sys.stderr)
        return False


def load_manifest(manifest_path: str) -> dict:
    if os.path.isfile(manifest_path):
        try:
            with open(manifest_path, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception as e:
            print(f"[inerium-shell] Warning: could not parse {manifest_path}: {e}", file=sys.stderr)
    return {}


def read_list_file(file_path: str) -> list:
    if not os.path.isfile(file_path):
        return []
    with open(file_path, "r", encoding="utf-8") as f:
        return [line.strip() for line in f if line.strip()]


def read_tsv_file(file_path: str) -> list:
    if not os.path.isfile(file_path):
        return []
    res = []
    seen = set()
    with open(file_path, "r", encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or "\t" not in line:
                continue
            dst, src = line.split("\t", 1)
            dst, src = dst.strip(), src.strip()
            if dst not in seen:
                seen.add(dst)
                res.append({"path": dst, "target": src})
    return res


def action_save(args):
    temp_dir = args.temp_dir
    manifest_path = args.manifest
    repo_dir = args.repo_dir

    old_manifest = load_manifest(manifest_path)

    binaries = sorted(list(set(read_list_file(os.path.join(temp_dir, "binaries")))))
    templates = sorted(list(set(read_list_file(os.path.join(temp_dir, "templates")))))
    system_files = sorted(list(set(read_list_file(os.path.join(temp_dir, "system_files")))))
    user_units = sorted(list(set(read_list_file(os.path.join(temp_dir, "user_units")))))
    system_units = sorted(list(set(read_list_file(os.path.join(temp_dir, "system_units")))))
    addons = sorted(list(set(read_list_file(os.path.join(temp_dir, "addons")))))
    symlinks = read_tsv_file(os.path.join(temp_dir, "symlinks"))

    new_manifest = {
        "version": args.version,
        "commit": args.commit,
        "installed_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "install_mode": args.install_mode,
        "repo_dir": repo_dir,
        "addons": addons,
        "binaries": binaries,
        "symlinks": symlinks,
        "templates": templates,
        "system_files": system_files,
        "user_units": user_units,
        "system_units": system_units,
    }

    # Prune obsolete items if old manifest exists
    if old_manifest:
        prune_manifest_diff(old_manifest, new_manifest, repo_dir)

    # Save new manifest atomically
    os.makedirs(os.path.dirname(manifest_path), exist_ok=True)
    tmp_out = manifest_path + ".tmp"
    with open(tmp_out, "w", encoding="utf-8") as f:
        json.dump(new_manifest, f, indent=2, ensure_ascii=False)
        f.write("\n")
    os.replace(tmp_out, manifest_path)
    print(f"[inerium-shell] Updated installation manifest: {manifest_path}")


def prune_manifest_diff(old: dict, new: dict, repo_dir: str):
    """Compare old and new manifest, deleting removed symlinks, binaries, and system files."""
    new_symlink_paths = {item["path"] for item in new.get("symlinks", [])}
    new_binaries = set(new.get("binaries", []))
    new_system_files = set(new.get("system_files", []))
    new_user_units = set(new.get("user_units", []))
    new_system_units = set(new.get("system_units", []))

    # Obsolete symlinks
    for item in old.get("symlinks", []):
        p = item.get("path")
        if p and p not in new_symlink_paths:
            if os.path.islink(p):
                if is_inerium_target(p, repo_dir) or not os.path.exists(p):
                    print(f"[inerium-shell] Pruning obsolete symlink: {p}")
                    remove_file_or_link(p)

    # Obsolete user binaries
    for b in old.get("binaries", []):
        if b and b not in new_binaries:
            if os.path.isfile(b) or os.path.islink(b):
                print(f"[inerium-shell] Pruning obsolete user binary: {b}")
                remove_file_or_link(b)

    # Obsolete user units
    for u in old.get("user_units", []):
        if u and u not in new_user_units:
            print(f"[inerium-shell] Disabling and pruning obsolete user unit: {u}")
            subprocess.run(["systemctl", "--user", "disable", "--now", u], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            unit_file = os.path.expanduser(f"~/.config/systemd/user/{u}")
            remove_file_or_link(unit_file)

    # Obsolete system files
    for sf in old.get("system_files", []):
        if sf and sf not in new_system_files:
            print(f"[inerium-shell] Pruning obsolete system file: {sf}")
            remove_file_or_link(sf, sudo=True)

    # Obsolete system units
    for su in old.get("system_units", []):
        if su and su not in new_system_units:
            print(f"[inerium-shell] Disabling and pruning obsolete system unit: {su}")
            subprocess.run(["sudo", "systemctl", "disable", "--now", su], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            sys_unit_file = f"/etc/systemd/system/{su}"
            remove_file_or_link(sys_unit_file, sudo=True)


def action_clean_orphans(args):
    """Clean broken symlinks and known legacy files across inerium config directories."""
    repo_dir = args.repo_dir
    home = os.path.expanduser("~")

    # Directories to inspect for broken symlinks
    scan_dirs = [
        os.path.join(home, ".local/bin"),
        os.path.join(home, ".local/share/applications"),
        os.path.join(home, ".local/share/kio/servicemenus"),
        os.path.join(home, ".config/hypr"),
        os.path.join(home, ".config/hypr/config"),
        os.path.join(home, ".config/systemd/user"),
    ]

    for sdir in scan_dirs:
        if not os.path.isdir(sdir):
            continue
        for entry in os.scandir(sdir):
            if entry.is_symlink():
                # Check if dangling
                if not os.path.exists(entry.path):
                    if is_inerium_target(entry.path, repo_dir):
                        print(f"[inerium-shell] Removing broken inerium symlink: {entry.path}")
                        remove_file_or_link(entry.path)

    # Known legacy obsolete files that should never exist in ~/.config/hypr or ~/.local/bin
    known_legacy = [
        os.path.join(home, ".config/hypr/AGENTS.md"),
        os.path.join(home, ".config/hypr/hyprtoolkit.conf"),
        os.path.join(home, ".config/hypr/config/inputs.lua"),
        os.path.join(home, ".config/hypr/config/autostart_waybar.lua"),
        os.path.join(home, ".config/hypr/config/binds_waybar.lua"),
        os.path.join(home, ".config/hypr/config/binds_common.lua"),
        os.path.join(home, ".config/hypr/config/theme_settings.json"),
        os.path.join(home, ".local/bin/hypr-profile"),
        os.path.join(home, ".local/bin/inerium-hotspot.sh"),
        os.path.join(home, ".local/bin/inerium-popup-backup"),
        os.path.join(home, ".local/bin/inerium-switch-bar"),
        os.path.join(home, ".local/bin/inerium-switch-bar-backup"),
        os.path.join(home, ".local/bin/rofi-audio-sink"),
        os.path.join(home, ".local/bin/rofi-bluetooth"),
        os.path.join(home, ".local/bin/rofi-power-menu"),
        os.path.join(home, ".local/bin/rofi-power-menu-backup"),
        os.path.join(home, ".local/bin/rofi-wifi-menu"),
        os.path.join(home, ".local/bin/inerium-toggle-bluetooth.sh"),
        os.path.join(home, ".local/bin/inerium-toggle-network.sh"),
    ]

    for legacy in known_legacy:
        if os.path.islink(legacy) or os.path.isfile(legacy):
            print(f"[inerium-shell] Removing legacy file/link: {legacy}")
            remove_file_or_link(legacy)


def action_uninstall(args):
    manifest_path = args.manifest
    repo_dir = args.repo_dir
    dry_run = args.dry_run

    manifest = load_manifest(manifest_path)
    if not manifest:
        print(f"[inerium-shell] No manifest found at {manifest_path}. Running legacy fallback cleanup.")
    else:
        print(f"[inerium-shell] Loaded manifest for version {manifest.get('version', 'unknown')}")

    # 1. Stop and disable system services
    for su in manifest.get("system_units", []):
        print(f"[inerium-shell] Stopping and disabling system service: {su}")
        if not dry_run:
            subprocess.run(["sudo", "systemctl", "disable", "--now", su], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            remove_file_or_link(f"/etc/systemd/system/{su}", sudo=True)

    # 2. Stop and disable user services
    for uu in manifest.get("user_units", []):
        print(f"[inerium-shell] Stopping and disabling user service: {uu}")
        if not dry_run:
            subprocess.run(["systemctl", "--user", "disable", "--now", uu], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            unit_file = os.path.expanduser(f"~/.config/systemd/user/{uu}")
            remove_file_or_link(unit_file)

    # 3. Remove symlinks
    for item in manifest.get("symlinks", []):
        p = item.get("path")
        if p and (os.path.islink(p) or not os.path.exists(p)):
            print(f"[inerium-shell] Removing symlink: {p}")
            remove_file_or_link(p, dry_run=dry_run)

    # 4. Remove user binaries
    for b in manifest.get("binaries", []):
        print(f"[inerium-shell] Removing user binary: {b}")
        remove_file_or_link(b, dry_run=dry_run)

    # Remove set-wallpaper symlink
    remove_file_or_link(os.path.expanduser("~/.local/bin/set-wallpaper"), dry_run=dry_run)

    # 5. Remove rendered templates
    for tpl in manifest.get("templates", []):
        print(f"[inerium-shell] Removing generated configuration: {tpl}")
        remove_file_or_link(tpl, dry_run=dry_run)

    # 6. Remove system files
    for sf in manifest.get("system_files", []):
        print(f"[inerium-shell] Removing system file: {sf}")
        remove_file_or_link(sf, sudo=True, dry_run=dry_run)

    # 7. Clean broken links and legacy orphans
    action_clean_orphans(args)

    # 8. Remove state files
    state_dir = os.path.dirname(manifest_path)
    setup_done = os.path.join(state_dir, "setup.done")
    remove_file_or_link(setup_done, dry_run=dry_run)
    remove_file_or_link(manifest_path, dry_run=dry_run)
    print("[inerium-shell] Uninstallation complete.")


def main():
    parser = argparse.ArgumentParser(description="Inerium Shell Manifest & Cleanup Engine")
    subparsers = parser.add_subparsers(dest="command", required=True)

    # Save
    p_save = subparsers.add_parser("save")
    p_save.add_argument("--temp-dir", required=True)
    p_save.add_argument("--manifest", required=True)
    p_save.add_argument("--repo-dir", required=True)
    p_save.add_argument("--version", default="unknown")
    p_save.add_argument("--commit", default="unknown")
    p_save.add_argument("--install-mode", default="bin")

    # Clean orphans
    p_clean = subparsers.add_parser("clean-orphans")
    p_clean.add_argument("--repo-dir", required=True)

    # Uninstall
    p_uninst = subparsers.add_parser("uninstall")
    p_uninst.add_argument("--manifest", required=True)
    p_uninst.add_argument("--repo-dir", required=True)
    p_uninst.add_argument("--dry-run", type=int, default=0)

    args = parser.parse_args()

    if args.command == "save":
        action_save(args)
    elif args.command == "clean-orphans":
        action_clean_orphans(args)
    elif args.command == "uninstall":
        action_uninstall(args)


if __name__ == "__main__":
    main()
