# inerium-i18n

Bilingual (Russian / English) localization catalog and translation macros for Inerium Shell.

## Usage

Provides lightweight zero-overhead localization strings across all Inerium Rust crates without heavy gettext runtime dependencies:

```rust
use inerium_i18n::{tr, tr_plural, tr_fmt, Lang};

// Simple static translation based on active language (detects $HOME/.config/locale.conf / env)
println!("{}", tr("Charging", "Зарядка"));

// Formatted string
let msg = tr_fmt!("Battery at {}%", "Батарея: {}%", 85);

// Pluralization
let count = 5;
let label = tr_plural(count, "file", "files", "файл", "файла", "файлов");

// Query or manually set active language
let current = Lang::get();
Lang::set(Lang::Ru);
```
