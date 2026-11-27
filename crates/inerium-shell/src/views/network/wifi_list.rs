use std::collections::HashMap;

use gdk4::BUTTON_SECONDARY;
use gtk4::prelude::*;
use gtk4::{Button, Label, Orientation};
use inerium_i18n::{tr, tr_fmt};
use relm4::ComponentSender;

use super::auth_form::{build_auth_form, AuthFormState};
use super::backend::{SavedConn, WifiAp};
use super::{NetworkModel, NetworkMsg};
use crate::hw;

#[allow(clippy::too_many_arguments)]
pub fn populate_wifi_list(
    list_box: &gtk4::Box,
    networks: &[WifiAp],
    saved_conns: &HashMap<String, SavedConn>,
    active_details: Option<&hw::network::ActiveWifiDetails>,
    connectivity: hw::NetworkConnectivity,
    expanded_ssid: Option<&str>,
    auth_state: Option<&AuthFormState>,
    hotspot_active: bool,
    wifi_enabled: bool,
    is_scanning: bool,
    sender: &ComponentSender<NetworkModel>,
) -> Option<gtk4::Box> {
    while let Some(child) = list_box.first_child() {
        list_box.remove(&child);
    }

    if hotspot_active {
        let card = gtk4::Box::new(Orientation::Vertical, 10);
        card.add_css_class("cc-card");
        card.set_margin_top(8);
        card.set_margin_bottom(8);

        let top_row = gtk4::Box::new(Orientation::Horizontal, 8);
        let ico = Label::new(Some("󱜠"));
        ico.add_css_class("card-icon");
        top_row.append(&ico);

        let title_box = gtk4::Box::new(Orientation::Vertical, 2);
        title_box.set_hexpand(true);
        let title = Label::new(Some(tr("Hotspot Active", "Занято точкой доступа")));
        title.add_css_class("card-title");
        title.set_xalign(0.0);
        title_box.append(&title);

        let sub = Label::new(Some(tr(
            "Wi-Fi Hotspot is active",
            "Раздача Wi-Fi (точка доступа)",
        )));
        sub.add_css_class("card-sub");
        sub.set_xalign(0.0);
        title_box.append(&sub);
        top_row.append(&title_box);
        card.append(&top_row);

        let desc = Label::new(Some(tr(
            "The wireless adapter is currently broadcasting Wi-Fi (AP mode).\n\nTo scan and connect to other Wi-Fi networks, turn off the hotspot first.",
            "Беспроводной модуль сейчас работает в режиме раздачи интернета (AP).\n\nДля сканирования и подключения к другим Wi-Fi сетям сначала выключите точку доступа.",
        )));
        desc.add_css_class("cc-meta");
        desc.set_wrap(true);
        desc.set_xalign(0.0);
        card.append(&desc);

        let stop_btn = Button::with_label(tr("󱜡  Stop Hotspot", "󱜡  Выключить точку доступа"));
        stop_btn.add_css_class("cc-chip");
        stop_btn.add_css_class("on");
        let s = sender.clone();
        stop_btn.connect_clicked(move |b| {
            b.set_sensitive(false);
            b.set_label(tr("󰑐 Stopping...", "󰑐 Выключение..."));
            s.input(NetworkMsg::StopHotspot);
        });
        card.append(&stop_btn);

        list_box.append(&card);
        return None;
    }

    if networks.is_empty() {
        let empty_lbl = Label::new(Some(if is_scanning {
            tr(
                "󰑐 Searching for Wi-Fi networks...",
                "󰑐 Поиск беспроводных сетей...",
            )
        } else if wifi_enabled {
            tr(
                "No networks found. Click «Refresh»",
                "Сети не найдены. Нажмите «Обновить»",
            )
        } else {
            tr("Wi-Fi is turned off", "Wi-Fi выключен")
        }));
        empty_lbl.add_css_class("cc-meta");
        empty_lbl.set_margin_top(20);
        list_box.append(&empty_lbl);
        return None;
    }

    let mut expanded_card = None;

    for net in networks {
        let card = gtk4::Box::new(Orientation::Vertical, 0);
        card.add_css_class("card-item");
        if net.is_active {
            card.add_css_class("active");
        }
        let is_exp = expanded_ssid == Some(net.ssid.as_str());
        if is_exp {
            card.add_css_class("expanded");
            expanded_card = Some(card.clone());
        }

        // Header row
        let header_row = gtk4::Box::new(Orientation::Horizontal, 8);

        // Clickable main content (icon + text + badge)
        let main_box = gtk4::Box::new(Orientation::Horizontal, 8);
        main_box.set_hexpand(true);
        main_box.set_cursor_from_name(Some("pointer"));

        let sig_ico = if net.signal > 75 {
            "󰤨"
        } else if net.signal > 50 {
            "󰤥"
        } else if net.signal > 25 {
            "󰤢"
        } else {
            "󰤟"
        };
        let ico = Label::new(Some(sig_ico));
        ico.add_css_class("card-icon");
        if net.is_connecting {
            ico.add_css_class("connecting");
        } else if net.is_active {
            ico.add_css_class("active");
        } else if net.signal > 75 {
            ico.add_css_class("signal-high");
        } else if net.signal > 35 {
            ico.add_css_class("signal-mid");
        } else {
            ico.add_css_class("signal-low");
        }
        main_box.append(&ico);

        let txt = gtk4::Box::new(Orientation::Vertical, 1);
        txt.set_hexpand(true);
        let name_lbl = Label::new(Some(&net.ssid));
        name_lbl.add_css_class("card-title");
        name_lbl.set_xalign(0.0);
        name_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        txt.append(&name_lbl);

        let sec_str = if net.is_connecting {
            if let Some(details) = active_details {
                if !details.band.is_empty() {
                    tr_fmt!(
                        "Acquiring IP address • {band}...",
                        "Получение IP-адреса • {band}...",
                        band = details.band
                    )
                } else {
                    tr("Connecting...", "Подключение...").to_string()
                }
            } else {
                tr("Connecting...", "Подключение...").to_string()
            }
        } else if net.is_active {
            if connectivity == hw::NetworkConnectivity::Portal {
                if let Some(details) = active_details
                    && !details.ip.is_empty()
                {
                    tr_fmt!(
                        "Sign-in required • {ip}",
                        "Требуется вход • {ip}",
                        ip = details.ip
                    )
                } else {
                    tr("Sign-in required", "Требуется вход").to_string()
                }
            } else if let Some(details) = active_details {
                if !details.ip.is_empty() && !details.band.is_empty() {
                    tr_fmt!(
                        "Connected • {ip} ({band})",
                        "Подключено • {ip} ({band})",
                        ip = details.ip,
                        band = details.band
                    )
                } else if !details.ip.is_empty() {
                    tr_fmt!(
                        "Connected • {ip}",
                        "Подключено • {ip}",
                        ip = details.ip
                    )
                } else {
                    tr("Connected", "Подключено").to_string()
                }
            } else {
                tr("Connected", "Подключено").to_string()
            }
        } else if net.security.is_empty() {
            tr("Open network", "Открытая сеть").to_string()
        } else {
            net.security.clone()
        };
        let sec_lbl = Label::new(Some(&sec_str));
        sec_lbl.add_css_class("card-sub");
        if !net.is_active && !net.is_connecting && net.security.is_empty() {
            sec_lbl.add_css_class("warning");
        }
        sec_lbl.set_xalign(0.0);
        sec_lbl.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        txt.append(&sec_lbl);
        main_box.append(&txt);

        // Badge: Active ("󰄬 В сети"), Connecting ("󰑐 Подключение..."), or Saved ("Сохранено")
        let badge = Label::new(None);
        badge.add_css_class("card-badge");
        if net.is_connecting {
            badge.set_label(tr("󰑐  Connecting...", "󰑐  Подключение..."));
            badge.add_css_class("connecting");
            badge.set_visible(true);
        } else if net.is_active {
            if connectivity == hw::NetworkConnectivity::Portal {
                badge.set_label(tr("󰖪  Sign in", "󰖪  Войти"));
                badge.add_css_class("connecting");
            } else {
                badge.set_label(tr("󰄬  Connected", "󰄬  В сети"));
                badge.add_css_class("active");
            }
            badge.set_visible(true);
        } else if net.is_saved {
            badge.set_label(tr("Saved", "Сохранено"));
            badge.add_css_class("saved");
            badge.set_visible(true);
        } else {
            badge.set_visible(false);
        }
        main_box.append(&badge);

        header_row.append(&main_box);

        // Chevron expand button on the right inside the card
        let chevron_btn = Button::new();
        chevron_btn.add_css_class("card-chevron");
        let chev_lbl = Label::new(Some(if is_exp { "󰅃" } else { "󰅀" }));
        chevron_btn.set_child(Some(&chev_lbl));
        chevron_btn.set_cursor_from_name(Some("pointer"));
        chevron_btn.set_tooltip_text(Some(tr("Quick Actions", "Быстрые действия")));

        let s_chev = sender.clone();
        let ssid_chev = net.ssid.clone();
        chevron_btn.connect_clicked(move |_| {
            s_chev.input(NetworkMsg::ToggleExpandAp(ssid_chev.clone()));
        });
        header_row.append(&chevron_btn);

        card.append(&header_row);

        // Tooltip on the entire card
        let tooltip_text = if net.is_active {
            let sec_desc = if net.security.is_empty() {
                tr("Open", "Открытая").to_string()
            } else {
                net.security.clone()
            };
            let mut tip = format!(
                "SSID: {}\n{}: {}",
                net.ssid,
                tr("Security", "Безопасность"),
                sec_desc
            );
            if let Some(d) = active_details {
                if !d.ip.is_empty() {
                    tip.push_str(&format!("\nIP: {}", d.ip));
                }
                if !d.gateway.is_empty() {
                    tip.push_str(&format!("\n{}: {}", tr("Gateway", "Шлюз"), d.gateway));
                }
                if d.freq_mhz > 0 {
                    tip.push_str(&format!(
                        "\n{}: {} MHz ({})",
                        tr("Frequency", "Частота"),
                        d.freq_mhz,
                        d.band
                    ));
                }
                if d.signal_dbm != 0 {
                    tip.push_str(&format!(
                        "\n{}: {} dBm ({}%)",
                        tr("Signal", "Сигнал"),
                        d.signal_dbm,
                        net.signal
                    ));
                }
            }
            tip
        } else {
            let sec_desc = if net.security.is_empty() {
                tr("Open", "Открытая").to_string()
            } else {
                net.security.clone()
            };
            format!(
                "SSID: {}\n{}: {}\n{}: {}%",
                net.ssid,
                tr("Security", "Безопасность"),
                sec_desc,
                tr("Signal", "Сигнал"),
                net.signal
            )
        };
        card.set_tooltip_text(Some(&tooltip_text));

        // Primary click on main_box -> connect / disconnect / password prompt
        let s_click = sender.clone();
        let net_item = net.clone();
        let b_ui = badge.clone();
        let main_gesture = gtk4::GestureClick::new();
        main_gesture.set_button(gdk4::BUTTON_PRIMARY);
        main_gesture.connect_released(move |_, _, _, _| {
            if net_item.is_active {
                b_ui.set_label(tr("󰑐 Disconnecting...", "󰑐 Отключение..."));
                b_ui.set_visible(true);
            } else if net_item.is_saved
                || net_item.security.is_empty()
                || net_item.security.contains("--")
            {
                b_ui.set_label(tr("󰑐 Connecting...", "󰑐 Подключение..."));
                b_ui.set_visible(true);
            }
            s_click.input(NetworkMsg::SelectAp(net_item.clone()));
        });
        main_box.add_controller(main_gesture);

        // Right-click gesture (ПКМ) on the entire card -> toggle expand
        let s_right = sender.clone();
        let ssid_right = net.ssid.clone();
        let right_gesture = gtk4::GestureClick::new();
        right_gesture.set_button(BUTTON_SECONDARY);
        right_gesture.connect_pressed(move |_, _, _, _| {
            s_right.input(NetworkMsg::ToggleExpandAp(ssid_right.clone()));
        });
        card.add_controller(right_gesture);

        // Integrated Accordion Drawer INSIDE the card (for ALL networks!)
        if is_exp {
            let sep = gtk4::Separator::new(Orientation::Horizontal);
            sep.add_css_class("card-drawer-sep");
            card.append(&sep);

            if let Some(conn) = saved_conns.get(&net.ssid) {
                let drawer = gtk4::Box::new(Orientation::Horizontal, 6);
                drawer.add_css_class("card-drawer-inner");
                drawer.set_hexpand(true);
                drawer.set_homogeneous(true);

                // 1. Autoconnect chip
                let auto_btn = Button::new();
                auto_btn.add_css_class("drawer-chip");
                auto_btn.set_hexpand(true);
                if conn.autoconnect {
                    auto_btn.add_css_class("on");
                    auto_btn.set_label(tr("󰑐  Auto: On", "󰑐  Авто: Вкл"));
                } else {
                    auto_btn.set_label(tr("󰑐  Auto: Off", "󰑐  Авто: Выкл"));
                }
                auto_btn.set_tooltip_text(Some(tr(
                    "Toggle automatic connection",
                    "Автоматически подключаться к сети",
                )));
                let s_auto = sender.clone();
                let u_auto = conn.uuid.clone();
                let cur_auto = conn.autoconnect;
                let b_auto = auto_btn.clone();
                auto_btn.connect_clicked(move |_| {
                    b_auto.set_sensitive(false);
                    if cur_auto {
                        b_auto.remove_css_class("on");
                        b_auto.set_label(tr("󰑐  Auto: Off", "󰑐  Авто: Выкл"));
                    } else {
                        b_auto.add_css_class("on");
                        b_auto.set_label(tr("󰑐  Auto: On", "󰑐  Авто: Вкл"));
                    }
                    s_auto.input(NetworkMsg::ToggleAutoconnect(u_auto.clone(), !cur_auto));
                });
                drawer.append(&auto_btn);

                // 2. Forget network chip
                let forget_btn = Button::with_label(tr("󰆴  Forget", "󰆴  Забыть"));
                forget_btn.add_css_class("drawer-chip");
                forget_btn.add_css_class("danger");
                forget_btn.set_hexpand(true);
                forget_btn.set_tooltip_text(Some(tr(
                    "Delete saved Wi-Fi profile and credentials",
                    "Удалить сохранённый профиль и пароль сети",
                )));
                let s_forget = sender.clone();
                let u_forget = conn.uuid.clone();
                let b_forget = forget_btn.clone();
                forget_btn.connect_clicked(move |_| {
                    b_forget.set_sensitive(false);
                    b_forget.set_label(tr("󰑐  Forgetting...", "󰑐  Удаление..."));
                    s_forget.input(NetworkMsg::ForgetSavedNetwork(u_forget.clone()));
                });
                drawer.append(&forget_btn);

                // 3. Connect / Disconnect chip
                if net.is_active {
                    if connectivity.needs_portal_login() {
                        let portal_btn = Button::with_label(tr("󰖟  Sign in", "󰖟  Войти"));
                        portal_btn.add_css_class("drawer-chip");
                        portal_btn.add_css_class("on");
                        portal_btn.set_hexpand(true);
                        portal_btn.set_tooltip_text(Some(tr(
                            "Open network sign-in page in browser",
                            "Открыть страницу авторизации в браузере",
                        )));
                        let s_portal = sender.clone();
                        portal_btn.connect_clicked(move |_| {
                            s_portal.input(NetworkMsg::OpenCaptivePortal);
                        });
                        drawer.append(&portal_btn);
                    }

                    let disc_btn = Button::with_label(tr("󱚼  Disconnect", "󱚼  Отключиться"));
                    disc_btn.add_css_class("drawer-chip");
                    disc_btn.set_hexpand(true);
                    let s_disc = sender.clone();
                    let net_disc = net.clone();
                    let b_disc = disc_btn.clone();
                    disc_btn.connect_clicked(move |_| {
                        b_disc.set_sensitive(false);
                        b_disc.set_label(tr("󰑐  Disconnecting...", "󰑐  Отключение..."));
                        s_disc.input(NetworkMsg::SelectAp(net_disc.clone()));
                    });
                    drawer.append(&disc_btn);
                } else {
                    let conn_btn = Button::with_label(tr("󰄬  Connect", "󰄬  Подключить"));
                    conn_btn.add_css_class("drawer-chip");
                    conn_btn.set_hexpand(true);
                    let s_c = sender.clone();
                    let net_c = net.clone();
                    let b_conn = conn_btn.clone();
                    conn_btn.connect_clicked(move |_| {
                        b_conn.set_sensitive(false);
                        b_conn.set_label(tr("󰑐  Connecting...", "󰑐  Подключение..."));
                        s_c.input(NetworkMsg::SelectAp(net_c.clone()));
                    });
                    drawer.append(&conn_btn);
                }
                card.append(&drawer);

                if let Some(auth) = auth_state
                    && auth.ssid == net.ssid
                    && let Some(ref err) = auth.error_msg
                {
                    let err_lbl = gtk4::Label::new(Some(err));
                    err_lbl.add_css_class("auth-error");
                    err_lbl.set_wrap(true);
                    card.append(&err_lbl);
                }
            } else if net.security.is_empty() || net.security.contains("--") {
                // Open network actions
                let drawer = gtk4::Box::new(Orientation::Horizontal, 6);
                drawer.add_css_class("card-drawer-inner");
                drawer.set_hexpand(true);
                drawer.set_homogeneous(true);

                if net.is_active {
                    if connectivity.needs_portal_login() {
                        let portal_btn = Button::with_label(tr("󰖟  Sign in", "󰖟  Войти"));
                        portal_btn.add_css_class("drawer-chip");
                        portal_btn.add_css_class("on");
                        portal_btn.set_hexpand(true);
                        portal_btn.set_tooltip_text(Some(tr(
                            "Open network sign-in page in browser",
                            "Открыть страницу авторизации в браузере",
                        )));
                        let s_portal = sender.clone();
                        portal_btn.connect_clicked(move |_| {
                            s_portal.input(NetworkMsg::OpenCaptivePortal);
                        });
                        drawer.append(&portal_btn);
                    }

                    let disc_btn = Button::with_label(tr("󱚼  Disconnect", "󱚼  Отключиться"));
                    disc_btn.add_css_class("drawer-chip");
                    disc_btn.set_hexpand(true);
                    let s_disc = sender.clone();
                    let net_disc = net.clone();
                    let b_disc = disc_btn.clone();
                    disc_btn.connect_clicked(move |_| {
                        b_disc.set_sensitive(false);
                        b_disc.set_label(tr("󰑐  Disconnecting...", "󰑐  Отключение..."));
                        s_disc.input(NetworkMsg::SelectAp(net_disc.clone()));
                    });
                    drawer.append(&disc_btn);
                } else {
                    let conn_btn = Button::with_label(tr("󰄬  Connect", "󰄬  Подключить"));
                    conn_btn.add_css_class("drawer-chip");
                    conn_btn.set_hexpand(true);
                    let s_c = sender.clone();
                    let net_c = net.clone();
                    let b_conn = conn_btn.clone();
                    conn_btn.connect_clicked(move |_| {
                        b_conn.set_sensitive(false);
                        b_conn.set_label(tr("󰑐 Connecting...", "󰑐 Подключение..."));
                        s_c.input(NetworkMsg::SelectAp(net_c.clone()));
                    });
                    drawer.append(&conn_btn);

                    let info_label = format!("󰋼 {} • {}%", tr("Open", "Открытая"), net.signal);
                    let info_chip = Label::new(Some(&info_label));
                    info_chip.add_css_class("drawer-chip");
                    info_chip.add_css_class("info");
                    info_chip.set_hexpand(true);
                    drawer.append(&info_chip);
                }

                card.append(&drawer);

                if let Some(auth) = auth_state
                    && auth.ssid == net.ssid
                    && let Some(ref err) = auth.error_msg
                {
                    let err_lbl = gtk4::Label::new(Some(err));
                    err_lbl.add_css_class("auth-error");
                    err_lbl.set_wrap(true);
                    card.append(&err_lbl);
                }
            } else {
                // In-Place Authentication Drawer (WPA2/WPA3-Personal or 802.1X Enterprise)
                let auth_drawer = build_auth_form(net, auth_state, sender);
                card.append(&auth_drawer);
            }
        }

        list_box.append(&card);
    }

    expanded_card
}
