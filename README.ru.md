# inerium-shell

[English](README.md) | [Русский](README.ru.md)

Универсальное окружение рабочего стола для CachyOS / Arch Linux, построенное вокруг **Hyprland (Lua API)**, **Waybar**, **Rofi**, **Mako**, **SwayOSD**, **Dolphin KIO** и нативного набора утилит на **Rust / GTK4** в фирменной цветовой палитре **Inerium Dragon**.

## Структура репозитория

- `Cargo.toml` — Rust workspace (`inerium-i18n`, `inerium-tools`, `inerium-shell`, `inerium-greeter`, `inerium-setup`, `dolphin-converter`)
- `standalone/inerium-polkit/` — Автономный Polkit-агент (форк [soteria](https://github.com/ImVaskel/soteria), Apache-2.0; `gtk4 0.9` / `relm4 0.9`, выделен отдельно во избежание конфликта `links = "gtk-4"` с `gtk4 0.11`)
- `config/` — Пользовательские конфигурации для создания симлинков (`hypr`, `waybar`, `rofi`, `mako`, `swayosd`, `fish`, `kitty`, `uwsm`, `systemd/user`, `qt5ct`, `qt6ct`, `gtk-3.0`, `gtk-4.0`)
- `local/` — Пользовательские скрипты (`~/.local/bin/`) и сервисные меню Dolphin KIO
- `system/` — Общесистемный `inerium-desktop-indexer`, шаблоны `greetd` (`--with-greeter`) и правило `sudoers.d`
- `assets/` — Обои по умолчанию (`assets/wallpapers/`) и фоны экрана входа (`assets/backgrounds/`)
- `pkglist-base.txt` — Базовый список пакетов Arch/CachyOS (минимальное чистое ядро)
- `bootstrap.sh` — Однострочный TTY-бутстраппер (устанавливает `git`, клонирует репозиторий, запускает `install.sh --with-greeter`)
- `install.sh` — Универсальный идемпотентный установщик (`--bin`, `--from-source`, `--with-greeter`, `--with-gdrive`, `--with-converter`, `--all-addons`)

## Способы установки и быстрый старт

### 1. Однострочный бутстрап (рекомендуется)

Работает одинаково как из **чистой TTY-консоли**, так и **внутри любого существующего графического окружения** (KDE Plasma, GNOME и др.):

**Короткий URL (через GitHub Pages):**
```bash
curl -sL ineron01.github.io/inerium-shell | bash
```

**Прямой URL GitHub Raw (альтернативный / резервный вариант):**
```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/ineron01/inerium-shell/main/bootstrap.sh)"
```

- **Интерактивный TUI:** автоматически определяет оборудование (GPU, батарею/шасси ноутбука, установленный дисплейный менеджер) и предлагает быстрый выбор компонентов.
- **Быстрые предсобранные бинарники (по умолчанию):** загружает компактные релизные архивы (~6 МБ) напрямую из GitHub Releases без долгой компиляции через Cargo.
- **Умный откат (Smart Fallback):** автоматически переключается на локальную сборку из исходников при несовместимости динамических библиотек.
- **Сохранение существующего окружения:** при установке поверх KDE/GNOME настраивает Inerium (`Hyprland (UWSM-Waybar)`) сессией по умолчанию в `greetd`, сохраняя KDE Plasma нетронутой и доступной для запуска.

### 2. Тихая / автоматическая установка (для скриптов и SSH)

```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/ineron01/inerium-shell/main/bootstrap.sh)" -- -y
```

### 3. Сборка из исходников (для разработчиков)

```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/ineron01/inerium-shell/main/bootstrap.sh)" -- --from-source
```

### 4. Локальный запуск из клонированного репозитория

```bash
./install.sh --dry-run
./install.sh --bin --with-greeter --with-gdrive
```

## Документация и руководство пользователя

- **[Руководство по горячим клавишам, жестам и Waybar](docs/keybindings.ru.md)** ([English version](docs/keybindings.md)) — Полное иллюстрированное руководство по управлению системой, тайлингу окон, шорткатам, жестам тачпада и скрытым возможностям панели Waybar.
- **[Руководство по дополнительным модулям](docs/addons.ru.md)** ([English version](docs/addons.md)) — Подробная документация по Inerium Camera, охлаждению MSI (`msi-ec` DKMS + регулятор кулеров) и noVPN.
- **[Настройка смартфона для Inerium Camera](docs/camera-setup.ru.md)** ([English version](docs/camera-setup.md)) — Пошаговая инструкция по настройке смартфона через ADB для работы в режиме веб-камеры.

## Дополнительные системные модули (Add-ons)

Inerium Shell включает опциональные расширения, которые можно выбрать в TUI установщика или включить через флаги:

- `--with-camera`: **Inerium Camera** — Использование Android-смартфона как 1080p веб-камеры (`/dev/video0`) с графическим интерфейсом GTK4, горячим переключением камер и поддержкой портретного/альбомного форматов.
- `--with-msi-ec`: **Охлаждение MSI** — Пропатченный DKMS-драйвер ядра `msi-ec` (включая патч для Bravo 15 C7VE) + системный демон регулировки кулеров CPU/GPU.
- `--with-novpn`: **noVPN Split-Tunneling** — Прямой запуск выбранных приложений через физический сетевой шлюз в обход активного VPN.
- `--all-addons`: Установка всех доступных дополнительных модулей одновременно.

## Первичная настройка (First-login setup)

Утилита `inerium-first-login` выполняет однократные шаги настройки (вход в учетные записи и т. д.) из каталога `~/.config/inerium/first-login.d/` в плавающем окне терминала при первом входе в Hyprland, запоминает выполненные шаги и повторно предлагает отложенные при следующем входе. В базовом репозитории шаги отсутствуют; персональные оверлеи размещают там исполняемые файлы (каждый должен возвращать код 0 при вызове с флагом `--check` после успешной настройки). Повторный запуск доступен в любое время из Rofi («Первичная настройка») или командой `inerium-first-login --reset`.

Специфичные аппаратные функции MSI (`msi-fan-regulator`) изолированы за Cargo-фичей `msi-ec` и по умолчанию отключены в универсальной сборке. Все панели и утилиты определяют возможности системы во время выполнения (`/sys/devices/platform/msi-ec`, `/sys/class/backlight`, `/sys/class/power_supply/`, `/sys/class/net/`, `envycontrol`) и скрывают недоступные элементы управления.
