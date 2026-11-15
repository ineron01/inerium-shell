use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

#[inline]
pub fn read_sysfs_u32(path: &Path) -> Option<u32> {
    let mut file = File::open(path).ok()?;
    let mut buf = [0u8; 32];
    let n = file.read(&mut buf).ok()?;
    std::str::from_utf8(&buf[..n]).ok()?.trim().parse().ok()
}

#[inline]
pub fn read_sysfs_i64_opt(path: &Path) -> Option<i64> {
    let mut file = File::open(path).ok()?;
    let mut buf = [0u8; 32];
    let n = file.read(&mut buf).ok()?;
    std::str::from_utf8(&buf[..n]).ok()?.trim().parse().ok()
}

#[inline]
pub fn read_sysfs_i64(path: &Path) -> i64 {
    read_sysfs_i64_opt(path).unwrap_or(0)
}

#[inline]
pub fn read_sysfs_string(path: &Path) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let mut buf = [0u8; 64];
    let n = file.read(&mut buf).ok()?;
    Some(std::str::from_utf8(&buf[..n]).ok()?.trim().to_string())
}

#[inline]
pub fn read_sysfs_str(path: &Path) -> String {
    read_sysfs_string(path).unwrap_or_default()
}

pub fn home_dir() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

pub fn config_dir() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".config"))
}

pub fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home_dir().join(".cache"))
}
