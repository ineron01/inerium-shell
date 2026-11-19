# Inerium Shell — Дополнительные модули и компоненты

В состав Inerium Shell входят опциональные системные модули, которые можно выбрать при установке в интерактивном TUI или через флаги командной строки (`--with-gdrive`, `--with-converter`, `--with-camera`, `--with-msi-ec`, `--with-novpn`, `--all-addons`).

---

## 1. Inerium Camera (`--with-camera`)

- **Описание:** Нативный Rust/GTK4 менеджер и стример виртуальной веб-камеры. Превращает Android-смартфон в веб-камеру высокого разрешения (1080p @ 30fps) для Linux по USB (ADB) или Wi-Fi/точке доступа.
- **Состав:**
  - `crates/inerium-camera` -> `~/.local/bin/inerium-camera` (алиас `phone-webcam`)
  - `~/.config/systemd/user/inerium-camera-bridge.service`
  - `/etc/udev/rules.d/99-phone-webcam.rules`
  - `/etc/modprobe.d/v4l2loopback.conf`
- **Ключевые возможности:**
  - Создает постоянный узел `/dev/video0` (`PhoneWebcam`), который не закрывается при остановке потока (транслирует студийный черный цвет), защищая звонки в Discord, Telegram, Chrome и OBS от вылетов.
  - Горячее переключение между основной камерой, фронталкой и камерой ноутбука прямо во время созвона.
  - Поддержка форматов 16:9 и портретного 9:16 с автоматическими черными полосами без растягивания кадра.
- **Инструкция по настройке:** См. [Инструкция по подключению смартфона](camera-setup.ru.md).

---

## 2. Модуль охлаждения MSI (`--with-msi-ec`)

- **Описание:** Официальный DKMS-драйвер ядра `msi-ec` (пакет `msi-ec-dkms-git` из AUR) и системный демон регулировки скорости кулеров (`msi-fan-regulator`).
- **Совместимость и особенности:**
  - Начиная с версии 0.18 (PR #773), `msi-ec` поддерживает семейство Bravo 15 C7VE / C7V / C7UCX из коробки благодаря сопоставлению префикса прошивки EC (`158NIMS1.1`).
  - Демон регулятора непрерывно считывает температуры CPU/GPU и регулирует обороты кулеров через стандартные регистры `msi-ec` в sysfs.
  - Предназначен для ноутбуков MSI, где штатное охлаждение в Linux не справляется или кулеры работают неэффективно.
- **Состав:**
  - Пакет `msi-ec-dkms-git` (AUR)
  - Бинарник `msi-fan-regulator` (собирается из `crates/inerium-tools` с флагом `--features msi-ec`)
  - `/etc/systemd/system/msi-fan-regulator.service`
  - `/etc/udev/rules.d/99-msi-ec.rules`
  - `/etc/modules-load.d/msi-ec.conf`

---

## 3. noVPN — обход VPN для отдельных программ (`--with-novpn`)

- **Описание:** Легковесная утилита split-tunneling на базе изолированных сетевых пространств имен Linux (`ip netns`). Позволяет запускать выбранные приложения напрямую через физический шлюз провайдера, минуя любой глобальный VPN (AmneziaVPN, WireGuard, OpenVPN).
- **Состав:**
  - `/usr/local/bin/novpn`
  - `/usr/local/bin/novpn-desktop`
  - `/usr/local/bin/novpn-exec`
  - `/usr/local/bin/novpn-up`
  - `/usr/local/bin/novpn-down`
  - `/etc/systemd/system/novpn.service`
- **Примеры использования:**
  - `novpn make-desktop` — универсальный генератор ярлыка прямого доступа для любого установленного браузера (автообнаружение XDG).
  - `novpn curl https://ifconfig.me` — покажет реальный IP-адрес провайдера.
  - `novpn status` — статус связи и IP-адреса хоста (VPN) и изолированной сети.

---

## 4. Интеграция с Google Drive (`--with-gdrive`)

- **Описание:** Прямая фоновая синхронизация и монтирование Google Диска в файловый менеджер Dolphin через `rclone`.
- **Состав:**
  - `~/.config/rclone/filter.txt`
  - `~/.config/systemd/user/gdrive.service`
  - `~/.config/inerium/first-login.d/20-gdrive`
- **Ключевые возможности:**
  - Монтирует Google Drive в директорию `~/GoogleDrive` с VFS-кэшем 10 ГБ и автоматическим переподключением при сбоях сети.
  - Интерактивный терминальный виджет авторизации в мастере настройки `inerium-setup`.

---

## 5. Универсальный конвертер Dolphin (`--with-converter`)

- **Описание:** Высокопроизводительный нативный Rust-конвертер, интегрированный прямо в контекстное меню Dolphin KIO.
- **Состав:**
  - `crates/dolphin-converter` -> `~/.local/bin/dolphin-convert`
  - `~/.local/share/kio/servicemenus/converter-*.desktop`
  - Опциональная нейросетевая модель: `~/.local/share/inerium/models/silueta.onnx`
- **Ключевые возможности:**
  - Быстрая конвертация офисных документов (PDF, DOCX, ODT через LibreOffice и Pandoc), медиафайлов (MP4, MP3, WAV, WEBP через FFmpeg и ImageMagick).
  - Параллельная обработка очередей файлов на пуле потоков Rayon.
  - Опциональное локальное ИИ-удаление фона с изображений через ONNX Runtime (`silueta.onnx`).
