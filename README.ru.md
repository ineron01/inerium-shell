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
├── hardware/                # Пропатченный DKMS-драйвер ядра msi-ec (Bravo 15 C7VE)
├── local/                   # Пользовательские скрипты (~/.local/bin/) и меню Dolphin KIO
├── scripts/install/         # Модульные стадии установки (00-helpers .. 70-summary)
├── system/                  # Системные udev-правила, greetd и юниты systemd
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

### 3. Сборка из исходников (для разработчиков)

```bash
bash -c "$(curl -fsSL https://raw.githubusercontent.com/ineron01/inerium-shell/main/bootstrap.sh)" -- --from-source
```

### 4. Локальный запуск из клонированного репозитория

```bash
./install.sh --dry-run
./install.sh --bin --with-greeter --with-gdrive
```

---

## Документация и руководства

- **[Руководство по горячим клавишам, жестам и Waybar](docs/keybindings.ru.md)** ([English version](docs/keybindings.md)) — Полное иллюстрированное руководство по управлению системой, тайлингу окон, шорткатам, жестам тачпада и скрытым возможностям панели Waybar.
- **[Руководство по дополнительным модулям](docs/addons.ru.md)** ([English version](docs/addons.md)) — Подробная документация по Inerium Camera, охлаждению MSI (`msi-ec` DKMS + регулятор кулеров) и noVPN.
- **[Настройка смартфона для Inerium Camera](docs/camera-setup.ru.md)** ([English version](docs/camera-setup.md)) — Пошаговая инструкция по настройке смартфона через ADB для работы в режиме веб-камеры.

---

## Дополнительные системные модули (Add-ons)

Inerium Shell включает опциональные расширения, которые можно выбрать в TUI установщика или включить через флаги:

- `--with-gdrive`: **Google Drive Integration** — Фоновая служба `rclone` для удобного доступа к Google Диску в файловом менеджере Dolphin.
- `--with-converter`: **Dolphin Converter** — Конвертер аудио, видео, документов и изображений из контекстного меню Dolphin с опциональным ИИ-вырезанием фона (`silueta.onnx`).
- `--with-camera`: **Inerium Camera** — Использование Android-смартфона как 1080p веб-камеры (`/dev/video0`) с графическим интерфейсом GTK4, горячим переключением камер и поддержкой портретного/альбомного форматов.
- `--with-msi-ec`: **Охлаждение MSI** — Драйвер ядра `msi-ec-dkms-git` + системный демон автоматической регулировки кулеров CPU/GPU (`msi-fan-regulator`).
- `--with-novpn`: **noVPN Split-Tunneling** — Прямой запуск выбранных приложений через физический сетевой шлюз в обход активного VPN.
- `--all-addons`: Установка всех доступных дополнительных модулей одновременно.

---

## Первичная настройка (`inerium-setup`)

При первом входе в сессию запускается графический мастер `inerium-setup` (GTK4):
- Калибровка мониторов, частоты обновления и масштабирования.
- Выбор обоев и стилей интерфейса.
- Авторизация в GitHub CLI и Google Drive во встроенных терминалах без лишних окон.

Повторный запуск доступен в любое время из лаунчера или командой `inerium-setup`.
