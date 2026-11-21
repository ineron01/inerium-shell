use crate::device::{BuiltinStatus, DeviceManager};
use crate::preview::LivePreviewPipeline;
use crate::scrcpy::StreamingManager;
use crate::types::{CameraSessionInfo, CameraSource, Orientation};
use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, Button, CheckButton,
    Label, Orientation as GtkOrientation, Picture, PolicyType, ScrolledWindow,
    Spinner, ToggleButton,
};
use inerium_i18n::tr;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use tokio::sync::mpsc;

pub const INERIUM_CSS: &str = r#"
window {
    background-color: #1F1F28;
    color: #DCD7BA;
    font-family: -apple-system, BlinkMacSystemFont, "Cantarell", "Inter", sans-serif;
}

.main-container {
    padding: 10px;
}

.header-title {
    font-size: 15px;
    font-weight: 700;
    color: #DCD7BA;
    margin-bottom: 2px;
}

.header-subtitle {
    font-size: 11px;
    color: #727169;
    margin-bottom: 6px;
}

.card-box {
    background-color: #16161D;
    border: 1px solid #2A2A37;
    border-radius: 6px;
    padding: 6px 10px;
    margin-bottom: 6px;
}

.section-label {
    font-size: 10px;
    font-weight: 600;
    color: #938AA9;
    margin-bottom: 4px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
}

.segmented-btn {
    background-color: #223249;
    color: #DCD7BA;
    border: 1px solid #2D4F67;
    border-radius: 5px;
    padding: 4px 8px;
    font-size: 11px;
    font-weight: 500;
}

.segmented-btn:checked {
    background-color: #7E9CD8;
    color: #16161D;
    font-weight: 600;
}

.btn-primary-start {
    background-color: #76946A;
    color: #16161D;
    font-size: 13px;
    font-weight: 700;
    padding: 7px 18px;
    border-radius: 6px;
    border: none;
}

.btn-primary-start:hover {
    background-color: #87A987;
}

.btn-primary-stop {
    background-color: #C34043;
    color: #DCD7BA;
    font-size: 13px;
    font-weight: 700;
    padding: 7px 18px;
    border-radius: 6px;
    border: none;
}

.btn-primary-stop:hover {
    background-color: #E82424;
}

.btn-reset {
    background-color: #2A2A37;
    color: #E82424;
    font-size: 12px;
    font-weight: 600;
    padding: 7px 14px;
    border-radius: 6px;
    border: 1px solid #7E2A2A;
}

.btn-reset:hover {
    background-color: #7E2A2A;
    color: #DCD7BA;
}

.btn-reset:active {
    background-color: #C34043;
}

.btn-secondary {
    background-color: #2A2A37;
    color: #DCD7BA;
    font-size: 10px;
    font-weight: 500;
    padding: 4px 8px;
    border-radius: 5px;
    border: 1px solid #363646;
}

.btn-secondary:hover {
    background-color: #363646;
}

.preview-container {
    background-color: #0d0c0c;
    border: 1px solid #2A2A37;
    border-radius: 6px;
    margin-bottom: 6px;
}

.status-bar-text {
    font-size: 12px;
    color: #7E9CD8;
}

.status-warning {
    color: #E6C384;
}

.status-danger {
    color: #E82424;
}
"#;

#[derive(Debug)]
pub enum GuiEvent {
    DevicesRefreshed {
        builtin_hw_disabled: bool,
        external_desc: Result<String, String>,
    },
    StreamStarted(Result<CameraSessionInfo, String>),
    StreamStopped,
    StreamAborted(String),
    ResetCompleted(Result<(), String>),
    WifiClientDiscovered(Option<String>),
    TcpipEnabled(Result<(), String>),
}

fn spawn_stream_watchdog(tx: mpsc::Sender<GuiEvent>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(600)).await;
            let session = StreamingManager::get_current_session();
            if !session.is_streaming {
                break;
            }
            let mut failed = false;
            let mut reason = tr("Connection lost with device", "Потеряна связь с устройством");
            if let Some(pid) = session.pid {
                if !StreamingManager::is_pid_alive(pid) {
                    failed = true;
                    reason = tr("Video stream process exited", "Процесс видеопотока завершился");
                }
            }
            if !failed {
                for &aux in &session.aux_pids {
                    if aux > 0 && !StreamingManager::is_pid_alive(aux) {
                        failed = true;
                        reason = tr("Phone camera disconnected or discharged", "Камера телефона отключена или разрядилась");
                        break;
                    }
                }
            }
            if failed {
                let _ = StreamingManager::stop_stream().await;
                let _ = tx.send(GuiEvent::StreamAborted(reason.to_string())).await;
                break;
            }
        }
    });
}

pub fn run_gui() {
    let app = Application::builder()
        .application_id("org.inerium.camera")
        .build();

    app.connect_startup(|_| {
        if let Some(display) = gdk4::Display::default() {
            if let Some(home) = std::env::var_os("HOME") {
                let vars_path = std::path::PathBuf::from(home).join(".config/inerium/theme-vars.css");
                if let Ok(vars) = std::fs::read_to_string(&vars_path) {
                    let vars_provider = gtk4::CssProvider::new();
                    vars_provider.load_from_data(&vars);
                    gtk4::style_context_add_provider_for_display(
                        &display,
                        &vars_provider,
                        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
                    );
                }
            }
            let provider = gtk4::CssProvider::new();
            provider.load_from_data(INERIUM_CSS);
            gtk4::style_context_add_provider_for_display(
                &display,
                &provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
        }
    });

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(tr("Camera", "Камера"))
        .default_width(380)
        .default_height(500)
        .resizable(true)
        .build();

    let root_box = GtkBox::new(GtkOrientation::Vertical, 8);
    root_box.add_css_class("main-container");

    // Header
    let header_box = GtkBox::new(GtkOrientation::Vertical, 2);
    let title_label = Label::new(Some(tr("Camera", "Камера")));
    title_label.add_css_class("header-title");
    title_label.set_halign(Align::Start);

    let status_subtitle = Label::new(Some(tr("Initializing devices...", "Инициализация устройств...")));
    status_subtitle.add_css_class("header-subtitle");
    status_subtitle.set_halign(Align::Start);

    header_box.append(&title_label);
    header_box.append(&status_subtitle);
    root_box.append(&header_box);

    // Source Card
    let source_card = GtkBox::new(GtkOrientation::Vertical, 6);
    source_card.add_css_class("card-box");
    let source_title = Label::new(Some(tr("Video Source", "Источник видео")));
    source_title.add_css_class("section-label");
    source_title.set_halign(Align::Start);
    source_card.append(&source_title);

    let source_btn_box = GtkBox::new(GtkOrientation::Horizontal, 6);
    let btn_src_builtin = ToggleButton::with_label(tr("Built-in", "Встроенная"));
    let btn_src_back = ToggleButton::with_label(tr("Main", "Основная"));
    let btn_src_front = ToggleButton::with_label(tr("Front", "Фронтальная"));

    btn_src_builtin.set_tooltip_text(Some(tr("Built-in laptop webcam (Fn + F6)", "Встроенная веб-камера ноутбука (Fn + F6)")));
    btn_src_back.set_tooltip_text(Some(tr("Main rear smartphone camera", "Основная тыловая камера смартфона")));
    btn_src_front.set_tooltip_text(Some(tr("Front selfie smartphone camera", "Фронтальная селфи-камера смартфона")));

    btn_src_builtin.add_css_class("segmented-btn");
    btn_src_back.add_css_class("segmented-btn");
    btn_src_front.add_css_class("segmented-btn");

    btn_src_back.set_group(Some(&btn_src_builtin));
    btn_src_front.set_group(Some(&btn_src_builtin));
    btn_src_back.set_active(true);

    source_btn_box.append(&btn_src_builtin);
    source_btn_box.append(&btn_src_back);
    source_btn_box.append(&btn_src_front);
    source_card.append(&source_btn_box);
    root_box.append(&source_card);

    // Orientation Card
    let orient_card = GtkBox::new(GtkOrientation::Vertical, 6);
    orient_card.add_css_class("card-box");
    let orient_title = Label::new(Some(tr("Orientation", "Ориентация")));
    orient_title.add_css_class("section-label");
    orient_title.set_halign(Align::Start);
    orient_card.append(&orient_title);

    let orient_btn_box = GtkBox::new(GtkOrientation::Horizontal, 6);
    let btn_orient_land = ToggleButton::with_label(tr("16:9 (Landscape)", "16:9 (Альбомная)"));
    let btn_orient_port = ToggleButton::with_label(tr("9:16 (Portrait)", "9:16 (Портретная)"));

    btn_orient_land.add_css_class("segmented-btn");
    btn_orient_port.add_css_class("segmented-btn");
    btn_orient_port.set_group(Some(&btn_orient_land));
    btn_orient_land.set_active(true);

    orient_btn_box.append(&btn_orient_land);
    orient_btn_box.append(&btn_orient_port);
    orient_card.append(&orient_btn_box);
    root_box.append(&orient_card);

    // Extra Toggles Card
    let extra_card = GtkBox::new(GtkOrientation::Vertical, 6);
    extra_card.add_css_class("card-box");
    let extra_title = Label::new(Some(tr("Settings & Network", "Параметры и сеть")));
    extra_title.add_css_class("section-label");
    extra_title.set_halign(Align::Start);
    extra_card.append(&extra_title);

    let toggles_row = GtkBox::new(GtkOrientation::Horizontal, 16);
    let chk_torch = CheckButton::with_label(tr("Flashlight", "Вспышка (Фонарик)"));
    let chk_audio = CheckButton::with_label(tr("Microphone", "Микрофон"));

    toggles_row.append(&chk_torch);
    toggles_row.append(&chk_audio);
    extra_card.append(&toggles_row);

    let wifi_row = GtkBox::new(GtkOrientation::Horizontal, 6);
    let btn_scan_wifi = Button::with_label(tr("Wi-Fi Scan", "Поиск по Wi-Fi"));
    btn_scan_wifi.set_tooltip_text(Some(tr("Find phone on local Wi-Fi or laptop hotspot", "Найти телефон в локальной Wi-Fi сети или на точке доступа ноутбука")));
    btn_scan_wifi.add_css_class("btn-secondary");
    let btn_enable_tcpip = Button::with_label(tr("Wi-Fi via USB", "Wi-Fi через USB"));
    btn_enable_tcpip.set_tooltip_text(Some(tr("Enable ADB TCP/IP port 5555 over USB", "Включить режим ADB TCP/IP 5555 на USB")));
    btn_enable_tcpip.add_css_class("btn-secondary");

    wifi_row.append(&btn_scan_wifi);
    wifi_row.append(&btn_enable_tcpip);
    extra_card.append(&wifi_row);
    root_box.append(&extra_card);

    // Preview Container
    let preview_box = GtkBox::new(GtkOrientation::Vertical, 0);
    preview_box.add_css_class("preview-container");
    preview_box.set_hexpand(true);
    preview_box.set_vexpand(false);
    preview_box.set_halign(Align::Center);
    preview_box.set_valign(Align::Center);

    let picture = Picture::new();
    picture.set_can_shrink(true);
    picture.set_size_request(240, 135);

    let preview_placeholder = Label::new(Some(tr("Click «Start Streaming» to activate camera", "Нажмите «Запустить трансляцию» для активации камеры")));
    preview_placeholder.add_css_class("header-subtitle");
    preview_placeholder.set_margin_top(6);
    preview_placeholder.set_margin_bottom(6);

    preview_box.append(&picture);
    preview_box.append(&preview_placeholder);
    root_box.append(&preview_box);

    // Primary Action Button, Reset Button & Spinner (Docked at bottom)
    let action_box = GtkBox::new(GtkOrientation::Horizontal, 8);
    action_box.set_halign(Align::Center);
    action_box.set_margin_top(4);
    action_box.set_margin_bottom(2);

    let btn_action = Button::with_label(tr("Start Streaming", "Запустить трансляцию"));
    btn_action.add_css_class("btn-primary-start");

    let btn_reset = Button::with_label(tr("Reset", "Сброс"));
    btn_reset.set_tooltip_text(Some(tr("Emergency reset: kill all processes, disconnect ADB, and reset video bridge", "Аварийный сброс: завершить все процессы, отключить ADB и сбросить видеомост")));
    btn_reset.add_css_class("btn-reset");

    let spinner = Spinner::new();
    spinner.set_visible(false);

    action_box.append(&btn_action);
    action_box.append(&btn_reset);
    action_box.append(&spinner);

    // Bottom Status Bar (Docked)
    let status_bar_label = Label::new(Some(tr("Ready", "Готово к работе")));
    status_bar_label.add_css_class("status-bar-text");
    status_bar_label.set_halign(Align::Start);

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .propagate_natural_width(false)
        .propagate_natural_height(false)
        .vexpand(true)
        .hexpand(true)
        .min_content_width(320)
        .min_content_height(240)
        .child(&root_box)
        .build();

    let outer_box = GtkBox::new(GtkOrientation::Vertical, 4);
    outer_box.add_css_class("main-container");
    outer_box.append(&scrolled);
    outer_box.append(&action_box);
    outer_box.append(&status_bar_label);

    window.set_child(Some(&outer_box));
    window.present();

    // Channel for worker -> GTK main thread events
    let (tx, mut rx) = mpsc::channel::<GuiEvent>(32);

    // Initial session setup
    let init_session = StreamingManager::get_current_session();
    let current_source = Rc::new(Cell::new(init_session.source));
    let current_orientation = Rc::new(Cell::new(init_session.orientation));
    let is_streaming = Rc::new(Cell::new(init_session.is_streaming));
    let is_busy = Rc::new(Cell::new(false));
    let preview_pipeline = Rc::new(RefCell::new(LivePreviewPipeline::new()));

    match init_session.source {
        CameraSource::Builtin => btn_src_builtin.set_active(true),
        CameraSource::ExternalBack => btn_src_back.set_active(true),
        CameraSource::ExternalFront => btn_src_front.set_active(true),
    }

    match init_session.orientation {
        Orientation::Landscape => btn_orient_land.set_active(true),
        Orientation::Portrait => btn_orient_port.set_active(true),
    }

    chk_torch.set_active(init_session.torch);
    chk_audio.set_active(init_session.audio);

    if init_session.is_streaming {
        btn_action.set_label(tr("Stop Streaming", "Остановить трансляцию"));
        btn_action.remove_css_class("btn-primary-start");
        btn_action.add_css_class("btn-primary-stop");
        preview_placeholder.set_text(tr("Stream is active", "Трансляция активна"));
        if let Err(err) = preview_pipeline.borrow_mut().start(picture.clone()) {
            preview_placeholder.set_text(&format!("{}: {}", tr("Preview unavailable", "Превью недоступно"), err));
        }
        spawn_stream_watchdog(tx.clone());
    } else {
        tokio::spawn(async {
            let _ = StreamingManager::ensure_bridge_ready().await;
        });
    }

    // Spawn event dispatcher on the GTK main loop
    let subtitle_w = status_subtitle.clone();
    let status_bar_w = status_bar_label.clone();
    let btn_action_w = btn_action.clone();
    let btn_reset_w = btn_reset.clone();
    let btn_builtin_w = btn_src_builtin.clone();
    let btn_scan_wifi_w = btn_scan_wifi.clone();
    let spinner_w = spinner.clone();
    let picture_w = picture.clone();
    let placeholder_w = preview_placeholder.clone();

    let is_busy_w = is_busy.clone();
    let is_streaming_w = is_streaming.clone();
    let preview_pipeline_ev = preview_pipeline.clone();
    let tx_watch = tx.clone();

    // Centralized GTK event loop handler
    glib::spawn_future_local(async move {
        while let Some(event) = rx.recv().await {
            match event {
                GuiEvent::DevicesRefreshed {
                    builtin_hw_disabled,
                    external_desc,
                } => {
                    btn_builtin_w.set_sensitive(true);
                    if builtin_hw_disabled {
                        btn_builtin_w.set_tooltip_text(Some(tr("Built-in camera (disabled by Fn + F6 switch)", "Встроенная камера (аппаратно отключена Fn + F6)")));
                    } else {
                        btn_builtin_w.set_tooltip_text(Some(tr("Built-in laptop webcam", "Встроенная камера ноутбука")));
                    }

                    match external_desc {
                        Ok(desc) => {
                            subtitle_w.set_text(&format!("{}: {}", tr("Connected", "Подключено"), desc));
                            subtitle_w.remove_css_class("status-danger");
                            subtitle_w.remove_css_class("status-warning");
                        }
                        Err(_) => {
                            if builtin_hw_disabled {
                                subtitle_w.set_text(tr("External camera not connected. Built-in camera disabled (Fn + F6).", "Внешняя камера не подключена. Встроенная камера выключена (Fn + F6)."));
                                subtitle_w.add_css_class("status-warning");
                            } else {
                                subtitle_w.set_text(tr("External camera not connected. Built-in camera available.", "Внешняя камера не подключена. Доступна встроенная камера."));
                            }
                        }
                    }
                }
                GuiEvent::StreamStarted(res) => {
                    is_busy_w.set(false);
                    spinner_w.stop();
                    spinner_w.set_visible(false);
                    btn_action_w.set_sensitive(true);

                    match res {
                        Ok(info) => {
                            is_streaming_w.set(true);
                            btn_action_w.set_label(tr("Stop Streaming", "Остановить трансляцию"));
                            btn_action_w.remove_css_class("btn-primary-start");
                            btn_action_w.add_css_class("btn-primary-stop");

                            subtitle_w.set_text(&format!("{}: {}", tr("Connected", "Подключено"), info.conn_type.as_deref().unwrap_or(tr("Ready", "Готово"))));
                            status_bar_w.set_text(&format!("{} ({}, {})", tr("Stream is active", "Трансляция активна"), info.source, info.orientation));
                            placeholder_w.set_text(tr("Stream is active", "Трансляция активна"));

                            if !preview_pipeline_ev.borrow().is_running() {
                                if let Err(err) = preview_pipeline_ev.borrow_mut().start(picture_w.clone()) {
                                    placeholder_w.set_text(&format!("{}: {}", tr("Preview unavailable", "Превью недоступно"), err));
                                }
                            }
                            spawn_stream_watchdog(tx_watch.clone());
                        }
                        Err(err) => {
                            is_streaming_w.set(false);
                            btn_action_w.set_label(tr("Start Streaming", "Запустить трансляцию"));
                            btn_action_w.remove_css_class("btn-primary-stop");
                            btn_action_w.add_css_class("btn-primary-start");
                            status_bar_w.set_text(&format!("{}: {}", tr("Error", "Ошибка"), err));
                            placeholder_w.set_text(tr("Error starting camera", "Ошибка при запуске камеры"));
                            preview_pipeline_ev.borrow_mut().stop();
                            picture_w.set_paintable(None::<&gdk4::Paintable>);
                        }
                    }
                }
                GuiEvent::StreamStopped => {
                    is_busy_w.set(false);
                    is_streaming_w.set(false);
                    spinner_w.stop();
                    spinner_w.set_visible(false);
                    btn_action_w.set_sensitive(true);
                    btn_reset_w.set_sensitive(true);

                    btn_action_w.set_label(tr("Start Streaming", "Запустить трансляцию"));
                    btn_action_w.remove_css_class("btn-primary-stop");
                    btn_action_w.add_css_class("btn-primary-start");
                    placeholder_w.set_text(tr("Click «Start Streaming» to activate camera", "Нажмите «Запустить трансляцию» для активации камеры"));
                    status_bar_w.set_text(tr("Stream stopped", "Трансляция остановлена"));
                    preview_pipeline_ev.borrow_mut().stop();
                    picture_w.set_paintable(None::<&gdk4::Paintable>);
                }
                GuiEvent::StreamAborted(reason) => {
                    is_busy_w.set(false);
                    is_streaming_w.set(false);
                    spinner_w.stop();
                    spinner_w.set_visible(false);
                    btn_action_w.set_sensitive(true);
                    btn_reset_w.set_sensitive(true);

                    btn_action_w.set_label(tr("Start Streaming", "Запустить трансляцию"));
                    btn_action_w.remove_css_class("btn-primary-stop");
                    btn_action_w.add_css_class("btn-primary-start");
                    placeholder_w.set_text(tr("Camera stream interrupted", "Поток камеры прерван"));
                    subtitle_w.set_text(tr("Warning: device disconnected", "Внимание: устройство отключено"));
                    subtitle_w.add_css_class("status-danger");
                    status_bar_w.set_text(&format!("{}: {}", tr("Failure", "Сбой"), reason));
                    preview_pipeline_ev.borrow_mut().stop();
                    picture_w.set_paintable(None::<&gdk4::Paintable>);
                }
                GuiEvent::ResetCompleted(res) => {
                    is_busy_w.set(false);
                    is_streaming_w.set(false);
                    spinner_w.stop();
                    spinner_w.set_visible(false);
                    btn_action_w.set_sensitive(true);
                    btn_reset_w.set_sensitive(true);

                    btn_action_w.set_label(tr("Start Streaming", "Запустить трансляцию"));
                    btn_action_w.remove_css_class("btn-primary-stop");
                    btn_action_w.add_css_class("btn-primary-start");
                    placeholder_w.set_text(tr("Click «Start Streaming» to activate camera", "Нажмите «Запустить трансляцию» для активации камеры"));
                    preview_pipeline_ev.borrow_mut().stop();
                    picture_w.set_paintable(None::<&gdk4::Paintable>);

                    match res {
                        Ok(()) => {
                            subtitle_w.set_text(tr("Reset completed: processes and bridge cleared", "Сброс выполнен: процессы и мост очищены"));
                            subtitle_w.remove_css_class("status-danger");
                            subtitle_w.remove_css_class("status-warning");
                            status_bar_w.set_text(tr("Camera state and ADB completely reset", "Состояние камеры и ADB полностью сброшено"));
                        }
                        Err(e) => {
                            subtitle_w.set_text(tr("Error during reset", "Ошибка при сбросе"));
                            subtitle_w.add_css_class("status-danger");
                            status_bar_w.set_text(&format!("{}: {}", tr("Reset error", "Ошибка сброса"), e));
                        }
                    }
                }
                GuiEvent::WifiClientDiscovered(client_opt) => {
                    btn_scan_wifi_w.set_sensitive(true);
                    if !is_busy_w.get() {
                        spinner_w.stop();
                        spinner_w.set_visible(false);
                    }
                    if let Some(ip) = client_opt {
                        status_bar_w.set_text(&format!("{}: {}", tr("Found device", "Найдено устройство"), ip));
                        let net = if ip.starts_with("10.42.0.") { tr("Hotspot", "Точка доступа") } else { tr("Local network", "Локальная сеть") };
                        subtitle_w.set_text(&format!("{}: Wi-Fi ({} {})", tr("Connected", "Подключено"), net, ip));
                        subtitle_w.remove_css_class("status-danger");
                        subtitle_w.remove_css_class("status-warning");
                    } else {
                        status_bar_w.set_text(tr("Phone not found on Wi-Fi networks", "Телефон в сетях Wi-Fi не найден"));
                    }
                }
                GuiEvent::TcpipEnabled(res) => {
                    match res {
                        Ok(()) => {
                            status_bar_w.set_text(tr("TCP/IP 5555 mode enabled successfully. Cable can now be disconnected.", "Режим TCP/IP 5555 успешно включен. Теперь можно отключить кабель."));
                        }
                        Err(e) => {
                            status_bar_w.set_text(&format!("{}: {}", tr("Error", "Ошибка"), e));
                        }
                    }
                }
            }
        }
    });

    // Trigger initial device discovery
    let tx_init = tx.clone();
    tokio::spawn(async move {
        let builtin = DeviceManager::detect_builtin_webcam();
        let builtin_hw_disabled = matches!(builtin, BuiltinStatus::HardwareDisabled);
        let ext = DeviceManager::discover_external_device(None).await;
        let ext_desc = ext.map(|(_, desc)| desc);
        let _ = tx_init.send(GuiEvent::DevicesRefreshed {
            builtin_hw_disabled,
            external_desc: ext_desc,
        }).await;
    });

    // Helper closure to trigger stream restart or update
    let trigger_refresh = {
        let tx = tx.clone();
        let btn_action = btn_action.clone();
        let spinner = spinner.clone();
        let status_bar = status_bar_label.clone();
        let chk_torch = chk_torch.clone();
        let chk_audio = chk_audio.clone();
        let is_busy = is_busy.clone();
        let is_streaming = is_streaming.clone();

        move |source: CameraSource, orientation: Orientation| {
            if is_busy.get() {
                return;
            }
            if !is_streaming.get() {
                return;
            }

            // Keep preview SHM reader connected across source swaps because _bridge stays alive

            is_busy.set(true);
            spinner.set_visible(true);
            spinner.start();
            btn_action.set_sensitive(false);

            let torch = chk_torch.is_active();
            let audio = chk_audio.is_active();
            let tx_w = tx.clone();

            status_bar.set_text(tr("Updating configuration...", "Обновление конфигурации..."));
            tokio::spawn(async move {
                let res = StreamingManager::start_stream(source, orientation, torch, audio, None).await;
                let _ = tx_w.send(GuiEvent::StreamStarted(res)).await;
            });
        }
    };

    // Source button handlers
    {
        let chk_torch_c = chk_torch.clone();
        let trig = trigger_refresh.clone();
        let cur_src = current_source.clone();
        let cur_orient = current_orientation.clone();
        let status_bar_c = status_bar_label.clone();
        let subtitle_c = status_subtitle.clone();

        btn_src_builtin.connect_toggled(move |btn| {
            if !btn.is_active() {
                return;
            }

            cur_src.set(CameraSource::Builtin);
            chk_torch_c.set_sensitive(false);

            match DeviceManager::detect_builtin_webcam() {
                BuiltinStatus::Available { ref device_path, .. } => {
                    status_bar_c.set_text(&format!("{}: ({})", tr("Built-in camera ready", "Встроенная камера готова"), device_path));
                    subtitle_c.set_text(tr("Built-in camera connected", "Встроенная камера подключена"));
                    subtitle_c.remove_css_class("status-danger");
                    subtitle_c.remove_css_class("status-warning");
                }
                BuiltinStatus::HardwareDisabled | BuiltinStatus::NotFound => {
                    status_bar_c.set_text(tr("Built-in camera disabled (Fn + F6) or not found", "Встроенная камера отключена (Fn + F6) или не найдена"));
                    subtitle_c.set_text(tr("Built-in camera disabled (Fn + F6)", "Встроенная камера отключена (Fn + F6)"));
                    subtitle_c.add_css_class("status-warning");
                }
            }
            trig(CameraSource::Builtin, cur_orient.get());
        });
    }

    {
        let chk_torch_c = chk_torch.clone();
        let trig = trigger_refresh.clone();
        let cur_src = current_source.clone();
        let cur_orient = current_orientation.clone();
        let status_bar_c = status_bar_label.clone();

        btn_src_back.connect_toggled(move |btn| {
            if !btn.is_active() {
                return;
            }
            cur_src.set(CameraSource::ExternalBack);
            chk_torch_c.set_sensitive(true);
            status_bar_c.set_text(tr("Selected external (main) camera", "Выбрана внешняя (основная) камера"));
            trig(CameraSource::ExternalBack, cur_orient.get());
        });
    }

    {
        let chk_torch_c = chk_torch.clone();
        let trig = trigger_refresh.clone();
        let cur_src = current_source.clone();
        let cur_orient = current_orientation.clone();
        let status_bar_c = status_bar_label.clone();

        btn_src_front.connect_toggled(move |btn| {
            if !btn.is_active() {
                return;
            }
            cur_src.set(CameraSource::ExternalFront);
            chk_torch_c.set_sensitive(false);
            status_bar_c.set_text(tr("Selected external (front) camera", "Выбрана внешняя (фронтальная) камера"));
            trig(CameraSource::ExternalFront, cur_orient.get());
        });
    }

    // Orientation button handlers
    {
        let trig = trigger_refresh.clone();
        let cur_src = current_source.clone();
        let cur_orient = current_orientation.clone();
        let status_bar_c = status_bar_label.clone();

        btn_orient_land.connect_toggled(move |btn| {
            if !btn.is_active() {
                return;
            }
            cur_orient.set(Orientation::Landscape);
            status_bar_c.set_text(tr("Orientation: 16:9 (Landscape)", "Ориентация: 16:9 (Альбомная)"));
            trig(cur_src.get(), Orientation::Landscape);
        });
    }

    {
        let trig = trigger_refresh.clone();
        let cur_src = current_source.clone();
        let cur_orient = current_orientation.clone();
        let status_bar_c = status_bar_label.clone();

        btn_orient_port.connect_toggled(move |btn| {
            if !btn.is_active() {
                return;
            }
            cur_orient.set(Orientation::Portrait);
            status_bar_c.set_text(tr("Orientation: 9:16 (Portrait)", "Ориентация: 9:16 (Портретная)"));
            trig(cur_src.get(), Orientation::Portrait);
        });
    }

    // Toggle handlers
    {
        let trig = trigger_refresh.clone();
        let cur_src = current_source.clone();
        let cur_orient = current_orientation.clone();
        let is_stream = is_streaming.clone();
        chk_torch.connect_toggled(move |_| {
            if cur_src.get() == CameraSource::ExternalBack && is_stream.get() {
                trig(cur_src.get(), cur_orient.get());
            }
        });
    }

    {
        let trig = trigger_refresh.clone();
        let cur_src = current_source.clone();
        let cur_orient = current_orientation.clone();
        let is_stream = is_streaming.clone();
        chk_audio.connect_toggled(move |_| {
            if is_stream.get() {
                trig(cur_src.get(), cur_orient.get());
            }
        });
    }

    // Wi-Fi (LAN + hotspot) scan button
    {
        let tx_wifi = tx.clone();
        let status_bar = status_bar_label.clone();
        let btn_scan = btn_scan_wifi.clone();
        let spinner_scan = spinner.clone();

        btn_scan_wifi.connect_clicked(move |_| {
            btn_scan.set_sensitive(false);
            spinner_scan.set_visible(true);
            spinner_scan.start();
            status_bar.set_text(tr("Searching for phone on Wi-Fi networks...", "Поиск телефона в Wi-Fi сетях..."));
            let tx_w = tx_wifi.clone();
            tokio::spawn(async move {
                let client = DeviceManager::discover_wifi_device().await;
                let _ = tx_w.send(GuiEvent::WifiClientDiscovered(client)).await;
            });
        });
    }

    // Enable TCP/IP on USB button
    {
        let tx_tcp = tx.clone();
        let status_bar = status_bar_label.clone();

        btn_enable_tcpip.connect_clicked(move |_| {
            status_bar.set_text(tr("Enabling TCP/IP 5555 mode over USB...", "Включение режима TCP/IP 5555 на USB..."));
            let tx_w = tx_tcp.clone();
            tokio::spawn(async move {
                let res = DeviceManager::enable_tcpip_5555().await;
                let _ = tx_w.send(GuiEvent::TcpipEnabled(res)).await;
            });
        });
    }

    // Main action button (Start / Stop)
    {
        let tx_act = tx.clone();
        let btn_action_btn = btn_action.clone();
        let spinner_ref = spinner.clone();
        let status_bar = status_bar_label.clone();
        let chk_torch_c = chk_torch.clone();
        let chk_audio_c = chk_audio.clone();
        let cur_src = current_source.clone();
        let cur_orient = current_orientation.clone();
        let is_stream = is_streaming.clone();
        let is_busy_act = is_busy.clone();
        let preview_pipeline_act = preview_pipeline.clone();

        btn_action.connect_clicked(move |_| {
            if is_busy_act.get() {
                return;
            }

            let source = cur_src.get();
            let orientation = cur_orient.get();
            let streaming = is_stream.get();

            if !streaming && source == CameraSource::Builtin {
                if let BuiltinStatus::HardwareDisabled = DeviceManager::detect_builtin_webcam() {
                    status_bar.set_text(tr("Built-in camera disabled by hardware switch (Fn + F6)", "Встроенная камера отключена аппаратно (Fn + F6)"));
                    return;
                }
            }

            is_busy_act.set(true);
            spinner_ref.set_visible(true);
            spinner_ref.start();
            btn_action_btn.set_sensitive(false);

            let torch = chk_torch_c.is_active();
            let audio = chk_audio_c.is_active();
            let tx_w = tx_act.clone();

            if streaming {
                preview_pipeline_act.borrow_mut().stop();
                status_bar.set_text(tr("Stopping stream...", "Остановка трансляции..."));
                tokio::spawn(async move {
                    let _ = StreamingManager::stop_stream().await;
                    let _ = tx_w.send(GuiEvent::StreamStopped).await;
                });
            } else {
                status_bar.set_text(tr("Starting stream...", "Запуск трансляции..."));
                tokio::spawn(async move {
                    let res = StreamingManager::start_stream(source, orientation, torch, audio, None).await;
                    let _ = tx_w.send(GuiEvent::StreamStarted(res)).await;
                });
            }
        });
    }

    // Emergency Reset button handler
    {
        let tx_rst = tx.clone();
        let btn_action_ref = btn_action.clone();
        let btn_reset_ref = btn_reset.clone();
        let spinner_ref = spinner.clone();
        let status_bar = status_bar_label.clone();
        let is_busy_rst = is_busy.clone();
        let preview_pipeline_rst = preview_pipeline.clone();

        btn_reset.connect_clicked(move |_| {
            if is_busy_rst.get() {
                return;
            }

            is_busy_rst.set(true);
            spinner_ref.set_visible(true);
            spinner_ref.start();
            btn_action_ref.set_sensitive(false);
            btn_reset_ref.set_sensitive(false);
            preview_pipeline_rst.borrow_mut().stop();
            status_bar.set_text(tr("Performing emergency reset...", "Выполняется аварийный сброс..."));

            let tx_w = tx_rst.clone();
            tokio::spawn(async move {
                let res = StreamingManager::force_reset().await;
                let _ = tx_w.send(GuiEvent::ResetCompleted(res)).await;
            });
        });
    }
}
