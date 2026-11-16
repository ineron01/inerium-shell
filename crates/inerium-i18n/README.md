# inerium-i18n

Bilingual (Russian / English) localization catalog and translation macros for Inerium Shell.

## Usage

Provides lightweight zero-overhead localization strings across all Inerium Rust crates without heavy gettext runtime dependencies:

```rust
use inerium_i18n::{t, Locale};

let locale = Locale::from_env();
println!("{}", t!(locale, "Battery Charging", "Зарядка батареи"));
```
