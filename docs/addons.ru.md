# Inerium Shell — Дополнительные модули и компоненты

В состав Inerium Shell входят опциональные системные модули, которые можно выбрать при установке в интерактивном TUI или через флаги командной строки (`--with-camera`, `--with-msi-ec`, `--with-novpn`, `--all-addons`).

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

- **Описание:** Пропатченный DKMS-драйвер ядра `msi-ec` и системный демон регулировки скорости кулеров (`msi-fan-regulator`).
- **Совместимость и особенности:**
  - В драйвер `msi-ec` добавлен идентификатор BIOS/EC прошивки **`158NIMS1.10F`** для ноутбука **MSI Bravo 15 C7VE (MS-158N)** (см. `hardware/msi-ec/PATCH-158NIMS1.10F.txt`).
  - Демон регулятора непрерывно считывает температуры CPU/GPU и регулирует обороты кулеров через стандартные регистры `msi-ec` в sysfs.
  - Предназначен для ноутбуков MSI, где штатное охлаждение в Linux не справляется или кулеры работают неэффективно.
  - *Примечание:* Регулятор опирается на стандартные адреса регистров `msi-ec`. Если у вас другая модель MSI, убедитесь, что версия прошивки вашего EC поддерживается в `msi-ec`, либо добавьте строку прошивки вашего BIOS в `hardware/msi-ec/msi-ec.c`.
- **Состав:**
  - `hardware/msi-ec/` (DKMS-модуль, собирается в `/usr/src/msi-ec-*/`)
  - Бинарник `msi-fan-regulator` (собирается из `crates/inerium-tools` с флагом `--features msi-ec`)
  - `/etc/systemd/system/msi-fan-regulator.service`
  - `/etc/udev/rules.d/99-msi-ec.rules`
  - `/etc/modules-load.d/msi-ec.conf`

---

## 3. noVPN — обход VPN для отдельных программ (`--with-novpn`)

- **Описание:** Легковесная утилита split-tunneling на базе изолированных сетевых пространств имен Linux (`ip netns`). Позволяет запускать выбранные приложения напрямую через физический шлюз провайдера, минуя любой глобальный VPN (AmneziaVPN, WireGuard, OpenVPN).
- **Состав:**
  - `/usr/local/bin/novpn`
  - `/usr/local/bin/novpn-exec`
  - `/usr/local/bin/novpn-up`
  - `/usr/local/bin/novpn-down`
  - `/etc/systemd/system/novpn.service`
- **Примеры использования:**
  - `novpn curl https://ifconfig.me` — покажет реальный IP-адрес провайдера.
  - `novpn google-chrome-stable` — браузер пойдет в сеть напрямую без VPN.
  - `novpn status` — статус связи и IP-адреса хоста (VPN) и изолированной сети.
