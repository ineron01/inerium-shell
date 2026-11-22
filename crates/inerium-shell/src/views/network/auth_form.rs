use gtk4::prelude::*;
use gtk4::{Button, CheckButton, Label, Orientation};
use inerium_i18n::tr;
use relm4::ComponentSender;

use super::backend::WifiAp;
use super::{NetworkModel, NetworkMsg};

#[derive(Clone, Debug, Default)]
pub struct AuthFormState {
    pub ssid: String,
    pub identity: String,
    pub password: String,
    pub no_ca_cert: bool,
    pub is_enterprise: bool,
    pub error_msg: Option<String>,
    pub is_connecting: bool,
}

pub fn build_auth_form(
    net: &WifiAp,
    auth_state: Option<&AuthFormState>,
    sender: &ComponentSender<NetworkModel>,
) -> gtk4::Box {
    let drawer = gtk4::Box::new(Orientation::Vertical, 6);
    drawer.add_css_class("card-drawer-inner");
    drawer.set_hexpand(true);

    let is_ent = net.security.contains("802.1X") || net.security.contains("EAP");
    let cur_identity = auth_state.map(|a| a.identity.as_str()).unwrap_or("");
    let cur_pass = auth_state.map(|a| a.password.as_str()).unwrap_or("");
    let cur_no_ca = auth_state.map(|a| a.no_ca_cert).unwrap_or(true);
    let cur_err = auth_state.and_then(|a| a.error_msg.as_deref());
    let is_connecting = auth_state.map(|a| a.is_connecting).unwrap_or(false);

    // Construct Connect Button first so text entry listeners can update its sensitivity
    let is_valid_init = if is_ent {
        !cur_identity.trim().is_empty() && !cur_pass.trim().is_empty()
    } else {
        cur_pass.trim().len() >= 8
    };
    let conn_btn = Button::new();
    conn_btn.add_css_class("drawer-chip");
    conn_btn.add_css_class("on");
    conn_btn.set_hexpand(true);
    if is_connecting {
        conn_btn.set_sensitive(false);
        conn_btn.set_label(tr("󰑐 Connecting...", "󰑐 Подключение..."));
    } else {
        conn_btn.set_sensitive(is_valid_init);
        conn_btn.set_label(tr("󰄬 Connect", "󰄬 Подключить"));
    }
    let s_sub = sender.clone();
    let ssid_sub = net.ssid.clone();
    conn_btn.connect_clicked(move |_| {
        s_sub.input(NetworkMsg::SubmitAuth(ssid_sub.clone()));
    });

    // 1. Identity field (Enterprise only)
    let id_entry_opt = if is_ent {
        let id_entry = gtk4::Entry::new();
        id_entry.add_css_class("drawer-entry");
        id_entry.set_placeholder_text(Some(tr("Username / Student ID", "Логин / ID студента")));
        id_entry.set_text(cur_identity);
        drawer.append(&id_entry);
        Some(id_entry)
    } else {
        None
    };

    // 2. Password field (Personal & Enterprise)
    let pass_entry = gtk4::PasswordEntry::new();
    pass_entry.add_css_class("drawer-entry");
    pass_entry.set_placeholder_text(Some(tr("Password", "Пароль сети")));
    pass_entry.set_show_peek_icon(true);
    pass_entry.set_text(cur_pass);
    drawer.append(&pass_entry);

    // Auto-focus input field and scroll it into view smoothly
    let focus_target: gtk4::Widget = if let Some(ref id) = id_entry_opt {
        id.clone().upcast()
    } else {
        pass_entry.clone().upcast()
    };
    glib::idle_add_local(move || {
        focus_target.grab_focus();
        glib::ControlFlow::Break
    });

    // Real-time sensitivity updater on keystroke
    let b_conn = conn_btn.clone();
    let p_entry_clone = pass_entry.clone();
    let id_entry_clone = id_entry_opt.clone();
    let check_valid = move || {
        let p = p_entry_clone.text().to_string();
        if is_ent {
            let id_ok = id_entry_clone.as_ref().map(|id| !id.text().trim().is_empty()).unwrap_or(false);
            id_ok && !p.trim().is_empty()
        } else {
            p.trim().len() >= 8
        }
    };

    let chk_p = check_valid.clone();
    let b_p = b_conn.clone();
    let s_pass = sender.clone();
    pass_entry.connect_changed(move |e| {
        b_p.set_sensitive(chk_p());
        s_pass.input(NetworkMsg::PasswordInput(e.text().to_string()));
    });

    let s_act_pass = sender.clone();
    let ssid_act_pass = net.ssid.clone();
    pass_entry.connect_activate(move |_| {
        s_act_pass.input(NetworkMsg::SubmitAuth(ssid_act_pass.clone()));
    });

    if let Some(ref id_entry) = id_entry_opt {
        let chk_id = check_valid;
        let b_id = b_conn;
        let s_id = sender.clone();
        id_entry.connect_changed(move |e| {
            b_id.set_sensitive(chk_id());
            s_id.input(NetworkMsg::IdentityInput(e.text().to_string()));
        });
        let s_act_id = sender.clone();
        let ssid_act_id = net.ssid.clone();
        id_entry.connect_activate(move |_| {
            s_act_id.input(NetworkMsg::SubmitAuth(ssid_act_id.clone()));
        });
    }

    // 3. CA bypass toggle (Enterprise only)
    if is_ent {
        let ca_check = CheckButton::with_label(tr(
            "Don't validate CA certificate",
            "Не проверять сертификат CA",
        ));
        ca_check.add_css_class("auth-ca-check");
        ca_check.set_active(cur_no_ca);
        ca_check.set_tooltip_text(Some(tr(
            "Skip CA server certificate validation (recommended for university networks)",
            "Не проверять сертификат CA (рекомендуется для университетских сетей)",
        )));
        let s_ca = sender.clone();
        ca_check.connect_toggled(move |_| {
            s_ca.input(NetworkMsg::ToggleNoCaCert);
        });
        drawer.append(&ca_check);
    }

    // 4. Error label (if any)
    if let Some(err) = cur_err {
        let err_lbl = Label::new(Some(err));
        err_lbl.add_css_class("auth-error");
        err_lbl.set_xalign(0.0);
        err_lbl.set_wrap(true);
        drawer.append(&err_lbl);
    }

    // 5. Append Connect Button
    drawer.append(&conn_btn);

    drawer
}
