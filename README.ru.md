# Inerium Shell

<p align="center">
  <a href="https://github.com/ineron01/inerium-shell/actions/workflows/ci.yml"><img src="https://github.com/ineron01/inerium-shell/actions/workflows/ci.yml/badge.svg" alt="CI Quality Gate"></a>
  <a href="https://github.com/ineron01/inerium-shell/releases/latest"><img src="https://img.shields.io/github/v/release/ineron01/inerium-shell?color=blue&label=Release" alt="Latest Release"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-GPL--3.0--or--later-green.svg" alt="Лицензия: GPL-3.0"></a>
  <a href="https://archlinux.org"><img src="https://img.shields.io/badge/Arch%20Linux%20%2F%20CachyOS-Поддерживается-1793d1?logo=archlinux" alt="Arch Linux / CachyOS"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/Rust-2024%20Edition-dea584?logo=rust" alt="Rust 2024 Edition"></a>
</p>

[English](README.md) | [Русский](README.ru.md)

Универсальное окружение рабочего стола для CachyOS / Arch Linux, построенное вокруг **Hyprland (Lua API)**, **Waybar**, **Rofi**, **Mako**, **SwayOSD**, **Dolphin KIO** и нативного набора утилит на **Rust / GTK4** в фирменной цветовой палитре **Inerium Dragon**.

![Витрина Inerium Desktop](assets/preview/desktop.png)

---

## Структура репозитория

```
inerium-shell/
├── assets/                  # Обои по умолчанию, снимки витрины и фоны экрана входа
├── config/                  # Конфигурации для создания симлинков в ~/.config/*
├── crates/                  # Единый воркспейс утилит на Rust
│   ├── dolphin-converter/   # Конвертер медиа и документов для Dolphin с AI-удалением фона
│   ├── inerium-camera/      # Менеджер виртуальной веб-камеры (Android ADB/Wi-Fi в /dev/video0)
│   ├── inerium-greeter/     # Экран входа greetd на GTK4 с выбором сессий
│   ├── inerium-hw/          # Модуль автоопределения железа и телеметрии sysfs
│   ├── inerium-i18n/        # Каталог двуязычной локализации (RU / EN)
│   ├── inerium-polkit/      # Нативный Polkit-агент аутентификации на GTK4/Relm4
│   ├── inerium-setup/       # Графический мастер настройки экранов и авторизаций
│   ├── inerium-shell/       # Центр управления и интерактивные Relm4-попапы
│   └── inerium-tools/       # Демоны телеметрии Waybar, IPC-мосты и контроль кулеров
├── docs/                    # Иллюстрированные руководства, горячие клавиши и инструкции
├── first-login.d/           # Шаги первичной авторизации (GitHub CLI, Google Drive)
├── local/                   # Пользовательские скрипты (~/.local/bin/) и меню Dolphin KIO
├── scripts/install/         # Модульные стадии установки (00-helpers .. 70-summary)
├── system/                  # Системные udev-правила, greetd и юниты systemd
├── themes/                  # Базовые цветовые палитры (Kanagawa Dragon, Inerium Light) и пул themes/extra/
├── bootstrap.sh             # Однострочный TTY-бутстраппер
└── install.sh               # Универсальный оркестратор установки (--bin, --from-source)
```

---

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

### 3. Сборка из исходников

Клонирует репозиторий в `~/inerium-shell` (видимая папка в домашней директории) и компилирует крейты через Cargo:

```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/ineron01/inerium-shell/main/bootstrap.sh)" -- --from-source
```

> **Расположение репозитория и пути установки:**
> - Стандартная установка бинарников выполняется в `$XDG_DATA_HOME/inerium-shell` (по умолчанию `~/.local/share/inerium-shell`, скрытая папка — не захламляет `$HOME` и защищена от случайного удаления).
> - Сборка из исходников размещает репозиторий в `~/inerium-shell` (видимая директория в `$HOME` для удобного просмотра и редактирования кода).
> - Файлы конфигурации: Устанавливаются как реальные файлы прямо в `$XDG_CONFIG_HOME` (`~/.config/{hypr,waybar,kitty,rofi,mako,uwsm,gtk-3.0,gtk-4.0}`) и регистрируются в манифесте (`~/.local/state/inerium/install-manifest`), избавляя систему от хрупких симлинков каталогов. Пользовательские настройки сохраняются при обновлениях; для обновления конфигов используется флаг `--update-configs` с автоматическим созданием бэкапов `.bak.<timestamp>`.
> - Темы и палитры: Базовые темы размещаются в `~/.config/inerium/themes/` (`kanagawa-dragon.toml`, `inerium-light.toml`, `template.toml.example`), а каталог дополнительных тем доступен в `~/.local/share/inerium-shell/themes/extra/`.
> - Бинарные файлы: Пользовательские утилиты ставятся в `~/.local/bin/`, а экран входа — в `/usr/local/bin/inerium-greeter`.
> - Путь к репозиторию можно переопределить переменной: `INERIUM_SHELL_DIR=/любой/путь`.

### 4. Локальный запуск из клонированного репозитория

```bash
git clone https://github.com/ineron01/inerium-shell.git ~/inerium-shell
cd ~/inerium-shell
./install.sh --bin --with-greeter --with-gdrive
```

---

## Документация и руководства

- **[Руководство по горячим клавишам, жестам и Waybar](docs/keybindings.ru.md)** ([English version](docs/keybindings.md)) — Полное иллюстрированное руководство по управлению системой, тайлингу окон, шорткатам, жестам тачпада и скрытым возможностям панели Waybar.
- **[Руководство по дополнительным модулям](docs/addons.ru.md)** ([English version](docs/addons.md)) — Подробная документация по Inerium Camera, охлаждению MSI (`msi-ec` DKMS + регулятор кулеров) и noVPN.
- **[Настройка смартфона для Inerium Camera](docs/camera-setup.ru.md)** ([English version](docs/camera-setup.md)) — Пошаговая инструкция по настройке смартфона через ADB для работы в режиме веб-камеры.

---

## Дополнительные системные модули и параметры установщика

Inerium Shell включает опциональные расширения, которые можно выбрать в TUI установщика или включить через флаги командной строки:

### Флаги модулей (Add-ons)
- `--with-gdrive`: **Google Drive Integration** — Фоновая служба `rclone` для удобного доступа к Google Диску в файловом менеджере Dolphin.
- `--with-converter`: **Dolphin Converter** — Конвертер аудио, видео, документов и изображений из контекстного меню Dolphin с опциональным ИИ-вырезанием фона (`silueta.onnx`).
- `--with-camera`: **Inerium Camera** — Использование Android-смартфона как 1080p веб-камеры (`/dev/video0`) с графическим интерфейсом GTK4, горячим переключением камер и поддержкой портретного/альбомного форматов.
- `--with-msi-ec`: **Охлаждение MSI** — Драйвер ядра `msi-ec-dkms-git` + системный демон автоматической регулировки кулеров CPU/GPU (`msi-fan-regulator`).
- `--with-novpn`: **noVPN Split-Tunneling** — Прямой запуск выбранных приложений через физический сетевой шлюз в обход активного VPN.
- `--all-addons`: Установка всех доступных дополнительных модулей одновременно.

### Флаги конфигурации и управления
- `--bin`: Установка готовых скомпилированных бинарников из GitHub Releases (по умолчанию).
- `--from-source`: Локальная компиляция Rust-воркспейса через Cargo.
- `--with-greeter`: Установка и настройка дисплейного менеджера `greetd` + экрана входа `inerium-greeter`.
- `--lang [en|ru]`: Выбор языка установщика и базовой локали системы.
- `--workspaces N`: Число виртуальных рабочих столов на монитор (1-9, по умолчанию: 3).
- `--fetch-model`: Загрузка ИИ-модели удаления фона `silueta.onnx` (~43 МБ).
- `--update-configs`: Обновление конфигурационных файлов с сохранением измененных пользователем файлов в бэкапы `.bak.<timestamp>`.
- `--check-abi`: Проверка корректности зависимостей динамических библиотек (`ldd`) для всех установленных бинарников.
- `--prune`: Очистка устаревших файлов относительно предыдущего манифеста установки.
- `--dry-run`: Вывод планируемых действий без модификации системы.
- `--non-interactive`, `-y`: Автоматическая установка без интерактивных диалогов TUI.
- `--uninstall`: **Чистое удаление** — Отключает зарегистрированные службы, удаляет все разложенные симлинки, бинарники и шаблоны по манифесту установки (`~/.local/state/inerium/install-manifest`), а также удаляет `~/.local/share/inerium-shell` (не затрагивая рабочие каталоги разработчика).

---

## Первичная настройка (`inerium-setup`)

При первом входе в сессию запускается графический мастер `inerium-setup` (GTK4):
- Калибровка мониторов, частоты обновления и масштабирования.
- Выбор обоев и стилей интерфейса.
- Авторизация в GitHub CLI и Google Drive во встроенных терминалах без лишних окон.

Повторный запуск доступен в любое время из лаунчера или командой `inerium-setup`.


---

## Темы оформления и кастомизация (Theme Engine)

Inerium Desktop включает декларативный движок и компилятор тем оформления (`inerium-theme`), работающий по принципу Single Source of Truth.

### Базовые темы из коробки
- **Inerium Light (`inerium-light`)**: Чистая современная нейтральная светлая тема с высокой контрастностью, белыми карточками (`#ffffff`), глубоким слейт-текстом (`#1e293b`) и стандартной дневной температурой дисплея 6500K.
- **Kanagawa Dragon (`kanagawa-dragon`)**: Тёмная палитра на основе японских чернил суми-э с тёплыми тонами и мягкими акцентами.

### Адаптивный Центр Управления
- **2 темы (по умолчанию из коробки)**: Клик по плитке **«Тема»** в Центре Управления мгновенно переключает систему между светлым и тёмным оформлением.
- **>2 тем (при добавлении пользовательских тем)**: Как только в каталог `~/.config/inerium/themes/` добавляется хотя бы один файл, клик по плитке автоматически открывает интерактивную **Галерею тем** с карточками, именами авторов, цветовыми свотчами и индикатором активности. При этом правый клик (ПКМ) сохраняет возможность быстрого тоггла dark ↔ light.

### Создание пользовательской темы
Создайте TOML-файл темы в каталоге `~/.config/inerium/themes/<id>.toml`:

```toml
[theme]
id = "my-custom-theme"
name = "My Custom Theme"
author = "Ваше Имя"
upstream = "https://github.com/your-username/my-theme"
variant = "dark" # "dark" или "light"
license = "MIT"
description = "Краткое описание вашей палитры"

[palette]
bg_canvas         = "rgba(24, 22, 22, 0.97)"
bg_surface        = "#221f1f"
bg_surface_alt    = "#1c1a1a"
bg_surface_hover  = "#2d2a2a"
bg_surface_active = "#363434"

border_subtle     = "rgba(197, 201, 197, 0.10)"
border_active     = "rgba(197, 201, 197, 0.25)"

text_primary      = "#c5c9c5"
text_heading      = "#e6e2cc"
text_muted        = "#737c73"
text_subtle       = "#a6a69c"
text_inverted     = "#181616"

accent_green      = "#87a987"
accent_green_bg   = "rgba(135, 169, 135, 0.16)"
accent_green_border = "rgba(135, 169, 135, 0.35)"

accent_blue       = "#8ba4b0"
accent_blue_bg    = "rgba(139, 164, 176, 0.16)"
accent_blue_border = "rgba(139, 164, 176, 0.38)"

accent_amber      = "#c4b28a"
accent_amber_bg   = "rgba(196, 178, 138, 0.15)"
accent_amber_border = "rgba(196, 178, 138, 0.38)"

accent_red        = "#c4746e"
accent_red_bg     = "rgba(196, 116, 110, 0.15)"
accent_red_border = "rgba(196, 116, 110, 0.38)"

accent_aqua       = "#8ea4a2"
accent_aqua_bg    = "rgba(142, 164, 162, 0.16)"
accent_aqua_border = "rgba(142, 164, 162, 0.38)"

accent_purple     = "#8992a7"
accent_purple_bg  = "rgba(137, 146, 167, 0.16)"
accent_purple_border = "rgba(137, 146, 167, 0.38)"

shadow            = "rgba(0, 0, 0, 0.85)"
modal_backdrop    = "rgba(18, 16, 16, 0.48)"
slider_trough     = "rgba(197, 201, 197, 0.10)"
slider_thumb      = "#e6e2cc"

[display]
temperature = 6500 # Цветовая температура экрана в Кельвинах (6500 — нейтральный дневной, 5500 — теплая бумага)
```

### Каталог дополнительных тем (`themes/extra/`)
Помимо двух основных базовых тем, Inerium содержит коллекцию популярных палитр сообщества, доступных для активации в один клик в мастере `inerium-setup` или через `inerium-theme install <id>` (что навсегда переносит палитру в `~/.config/inerium/themes/`):
- **Kanagawa Lotus (`kanagawa-lotus`)**: Тёплая палитра состаренной японской бумаги (автор rebelot) с автоматическим переключением дисплея в мягкие 5500K.
- **Kanagawa Wave (`kanagawa-wave`)**: Классическая тёмная палитра морских глубин (автор rebelot).
- **Tokyo Night (`tokyo-night`)**: Неоновая тёмная киберпанк-палитра (автор folke).
- **Catppuccin Mocha (`catppuccin-mocha`)**: Тёмная пастельная палитра.
- **Catppuccin Latte (`catppuccin-latte`)**: Светлая пастельная палитра.
- **Nord (`nord`)**: Спокойная арктическая сине-холодная палитра.
- **Gruvbox Dark (`gruvbox-dark`)**: Тёплая ретро-палитра в земляных тонах.
- **Dracula (`dracula`)**: Контрастная палитра в фиолетовых тонах.

Шаблон для создания собственной темы доступен в `~/.config/inerium/themes/template.toml.example`.

### Команды CLI
- `inerium-theme list` — Показать активные темы (`--all` — показать все включая дополнительные, `--json` — машинный вывод).
- `inerium-theme set <id>` — Применить тему по её идентификатору.
- `inerium-theme install <id>` — Перенести тему из дополнительных в постоянные (`~/.config/inerium/themes/`) и применить.
- `inerium-theme toggle` — Быстрое переключение между светлой и тёмной темой.
- `inerium-theme status` — Вывести метаданные текущей активной темы в формате JSON.

### Пользовательские хуки (`~/.config/inerium/hooks.d/`)
Любой исполняемый файл или скрипт в `~/.config/inerium/hooks.d/` автоматически запускается при смене темы и получает вариант (`dark` или `light`) и идентификатор темы первым и вторым аргументами.
