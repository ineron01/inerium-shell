use crate::device::DeviceManager;
use crate::scrcpy::StreamingManager;
use crate::types::{CameraSource, Orientation};

#[derive(Debug)]
pub enum Commands {
    Gui,
    Start {
        source: String,
        orientation: String,
        torch: bool,
        audio: bool,
        ip: Option<String>,
    },
    Stop,
    Reset,
    Toggle,
    Status,
    Preview,
    WifiScan,
    InternalBridge {
        v4l2: Option<String>,
    },
}

pub fn parse_cli_args() -> Commands {
    let args: Vec<String> = std::env::args().collect();
    if args.len() <= 1 {
        return Commands::Gui;
    }

    let mut source = "back".to_string();
    let mut orientation = "16:9".to_string();
    let mut torch = false;
    let mut audio = false;
    let mut ip = None;
    let mut is_start = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "gui" => return Commands::Gui,
            "stop" => return Commands::Stop,
            "reset" | "--reset" => return Commands::Reset,
            "toggle" => return Commands::Toggle,
            "status" => return Commands::Status,
            "preview" => return Commands::Preview,
            "wifi-scan" => return Commands::WifiScan,
            "_bridge" => {
                let mut v4l2 = None;
                let mut j = i + 1;
                while j < args.len() {
                    if args[j] == "--v4l2" && j + 1 < args.len() {
                        v4l2 = Some(args[j + 1].clone());
                        break;
                    }
                    j += 1;
                }
                return Commands::InternalBridge { v4l2 };
            }
            "start" => {
                is_start = true;
            }
            "-f" | "--front" => {
                is_start = true;
                source = "front".to_string();
            }
            "-b" | "--back" => {
                is_start = true;
                source = "back".to_string();
            }
            "-l" | "--builtin" | "--laptop" => {
                is_start = true;
                source = "builtin".to_string();
            }
            "-t" | "--torch" => {
                is_start = true;
                torch = true;
            }
            "-m" | "-a" | "--audio" | "--mic" => {
                audio = true;
            }
            "-s" | "--source" => {
                if i + 1 < args.len() {
                    i += 1;
                    source = args[i].clone();
                    is_start = true;
                }
            }
            "-o" | "--orientation" => {
                if i + 1 < args.len() {
                    i += 1;
                    orientation = args[i].clone();
                }
            }
            "--ip" => {
                if i + 1 < args.len() {
                    i += 1;
                    ip = Some(args[i].clone());
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }

    if is_start {
        Commands::Start {
            source,
            orientation,
            torch,
            audio,
            ip,
        }
    } else {
        Commands::Gui
    }
}

fn print_help() {
    println!(r#"inerium-camera — Minimalist, zero-freeze camera streamer and manager

ИСПОЛЬЗОВАНИЕ:
  inerium-camera [КОМАНДА | ФЛАГИ]

КОМАНДЫ:
  gui            Запустить графический интерфейс GTK4 (по умолчанию)
  start          Запустить трансляцию видеопотока
  stop           Остановить трансляцию и освободить устройства
  reset          Аварийный сброс всех процессов, ADB и перезапуск моста
  toggle         Быстрое переключение (старт / стоп)
  status         Показать статус сессии и устройств
  preview        Открыть плавающее PIP-окно превью
  wifi-scan      Поиск телефона по Wi-Fi (локальная сеть и точка доступа 10.42.0.x) и подключение ADB

ПАРАМЕТРЫ СТАРТА:
  -s, --source <builtin|back|front>   Выбор камеры (встроенная ноутбука, внешняя основная/фронтальная)
  -b, --back                          Быстрый выбор основной камеры
  -f, --front                         Быстрый выбор фронтальной камеры
  -l, --builtin                       Быстрый выбор встроенной камеры ноутбука
  -o, --orientation <16:9|9:16>       Соотношение сторон (альбомная / портретная)
  -t, --torch                         Включить подсветку / вспышку (основная камера)
  -m, --audio                         Передавать звук микрофона
  --ip <IP>                           Ручной IP-адрес для подключения по Wi-Fi
"#);
}

pub async fn handle_cli(cmd: Commands) {
    match cmd {
        Commands::Gui => {
            crate::gui::run_gui();
        }
        Commands::Start {
            source,
            orientation,
            torch,
            audio,
            ip,
        } => {
            let src = match source.to_lowercase().as_str() {
                "builtin" | "laptop" | "встроенная" => CameraSource::Builtin,
                "front" | "фронтальная" => CameraSource::ExternalFront,
                _ => CameraSource::ExternalBack,
            };

            let orient = match orientation.as_str() {
                "9:16" | "portrait" | "вертикальная" => Orientation::Portrait,
                _ => Orientation::Landscape,
            };

            println!("Запуск трансляции камеры ({})...", src.label());
            match StreamingManager::start_stream(src, orient, torch, audio, ip.as_deref()).await {
                Ok(info) => {
                    println!("Трансляция успешно запущена:");
                    println!("  Источник: {}", info.source.label());
                    println!("  Ориентация: {}", info.orientation.label());
                    println!("  Устройство: {}", info.conn_type.as_deref().unwrap_or(""));
                    if let Some(dev) = info.v4l2_device {
                        println!("  V4L2 узел: {}", dev);
                    }
                }
                Err(e) => {
                    eprintln!("Ошибка запуска: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Stop => {
            println!("Остановка трансляции...");
            if let Err(e) = StreamingManager::stop_stream().await {
                eprintln!("Ошибка: {}", e);
                std::process::exit(1);
            }
            println!("Трансляция остановлена, ресурсы освобождены.");
        }
        Commands::Reset => {
            println!("Выполняется аварийный сброс камеры и процессов...");
            if let Err(e) = StreamingManager::force_reset().await {
                eprintln!("Ошибка сброса: {}", e);
                std::process::exit(1);
            }
            println!("Все процессы завершены, ADB отключен, видеомост сброшен в исходное состояние.");
        }
        Commands::Toggle => {
            let session = StreamingManager::get_current_session();
            if session.is_streaming {
                println!("Остановка активной трансляции...");
                let _ = StreamingManager::stop_stream().await;
                println!("Камера выключена.");
            } else {
                println!("Запуск трансляции...");
                match StreamingManager::start_stream(
                    CameraSource::ExternalBack,
                    Orientation::Landscape,
                    false,
                    false,
                    None,
                )
                .await
                {
                    Ok(info) => println!("Трансляция активна ({})", info.source.label()),
                    Err(e) => {
                        eprintln!("Ошибка: {}", e);
                        std::process::exit(1);
                    }
                }
            }
        }
        Commands::Status => {
            let session = StreamingManager::get_current_session();
            if session.is_streaming {
                println!("Статус: АКТИВНА");
                println!("Источник: {}", session.source.label());
                println!("Ориентация: {}", session.orientation.label());
                println!("Подключение: {}", session.conn_type.as_deref().unwrap_or("N/A"));
                println!("V4L2: {}", session.v4l2_device.as_deref().unwrap_or("N/A"));
                if let Some(pid) = session.pid {
                    println!("PID: {}", pid);
                }
            } else {
                println!("Статус: НЕ АКТИВНА");
                let builtin = DeviceManager::detect_builtin_webcam();
                println!("Встроенная камера: {:?}", builtin);
            }
        }
        Commands::Preview => {
            println!("Запуск окна превью...");
            let _ = std::process::Command::new("gst-launch-1.0")
                .args([
                    "-q",
                    "shmsrc",
                    &format!("socket-path={}", crate::scrcpy::PREVIEW_SHM_SOCKET),
                    "is-live=true",
                    "do-timestamp=true",
                    "!",
                    "video/x-raw,format=I420,width=1920,height=1080,framerate=30/1",
                    "!",
                    "videoconvert",
                    "!",
                    "autovideosink",
                ])
                .spawn();
        }
        Commands::InternalBridge { v4l2 } => {
            if let Err(e) = crate::bridge::run_bridge(v4l2).await {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
        Commands::WifiScan => {
            println!("Поиск телефона по Wi-Fi (локальная сеть и точка доступа)...");
            if let Some(target) = DeviceManager::discover_wifi_device().await {
                println!("Подключено устройство: {}", target);
            } else {
                println!("Телефон с ADB TCP/IP (порт 5555) в сетях Wi-Fi не найден.");
            }
        }
    }
}
