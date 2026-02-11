use gpui::*;
use gpui::prelude::FluentBuilder;

use super::text_input::TextInput;
use super::theme::Theme;
use crate::core::forward::ForwardMethod;
use crate::core::natter::{NatterConfig, NatterSession, NatterStatus, LogEntry};
use crate::profile::{Profile, ProfileStore};

/// The main application view
pub struct HolePatchApp {
    // Input fields
    bind_ip_input: Entity<TextInput>,
    bind_port_input: Entity<TextInput>,
    target_ip_input: Entity<TextInput>,
    target_port_input: Entity<TextInput>,
    keepalive_host_input: Entity<TextInput>,
    keepalive_port_input: Entity<TextInput>,
    keepalive_interval_input: Entity<TextInput>,
    profile_name_input: Entity<TextInput>,

    // State
    udp_mode: bool,
    forward_method_index: usize,
    forward_methods: Vec<ForwardMethod>,

    // Session
    session: NatterSession,
    cached_status: NatterStatus,
    cached_logs: Vec<crate::core::natter::LogEntry>,

    // Profiles
    profile_store: ProfileStore,
    selected_profile_index: Option<usize>,
    show_profile_panel: bool,

    // Focus
    focus_handle: FocusHandle,
}

impl HolePatchApp {
    fn new(cx: &mut Context<Self>) -> Self {
        let forward_methods = ForwardMethod::all();
        let profile_store = ProfileStore::load();

        let mut app = HolePatchApp {
            bind_ip_input: cx.new(|cx| {
                TextInput::new("bind-ip", "绑定 IP", "0.0.0.0", cx)
            }),
            bind_port_input: cx.new(|cx| {
                TextInput::new("bind-port", "绑定端口", "0 (自动)", cx)
            }),
            target_ip_input: cx.new(|cx| {
                TextInput::new("target-ip", "目标 IP", "0.0.0.0", cx)
            }),
            target_port_input: cx.new(|cx| {
                TextInput::new("target-port", "目标端口", "0 (跟随外部端口)", cx)
            }),
            keepalive_host_input: cx.new(|cx| {
                TextInput::new("ka-host", "保活服务器", "www.baidu.com", cx)
            }),
            keepalive_port_input: cx.new(|cx| {
                TextInput::new("ka-port", "保活端口", "80", cx)
            }),
            keepalive_interval_input: cx.new(|cx| {
                let mut input = TextInput::new("ka-interval", "保活间隔 (秒)", "15", cx);
                input.set_text("15", cx);
                input
            }),
            profile_name_input: cx.new(|cx| {
                TextInput::new("profile-name", "配置名称", "My Profile", cx)
            }),
            udp_mode: false,
            forward_method_index: 1, // TestServer by default
            forward_methods,
            session: NatterSession::new(),
            cached_status: NatterStatus::Idle,
            cached_logs: vec![],
            profile_store,
            selected_profile_index: None,
            show_profile_panel: false,
            focus_handle: cx.focus_handle(),
        };

        // Load last used profile if available
        if let Some(idx) = app.profile_store.last_used_index {
            if idx < app.profile_store.profiles.len() {
                app.selected_profile_index = Some(idx);
                let profile = app.profile_store.profiles[idx].clone();
                app.apply_profile_values(&profile, cx);
            }
        }

        // Start a timer to poll session status
        cx.spawn(async move |this: WeakEntity<HolePatchApp>, cx: &mut AsyncApp| {
            loop {
                cx.background_executor().timer(std::time::Duration::from_millis(500)).await;
                let should_continue = this.update(cx, |this, cx| {
                    this.poll_session(cx);
                    true
                });
                if should_continue.is_err() {
                    break;
                }
            }
        }).detach();

        app
    }

    fn poll_session(&mut self, cx: &mut Context<Self>) {
        if let Ok(status) = self.session.status.lock() {
            let status_changed = !matches!(
                (&self.cached_status, &*status),
                (NatterStatus::Idle, NatterStatus::Idle)
                | (NatterStatus::Connecting, NatterStatus::Connecting)
            );
            // Always update cached status by cloning
            self.cached_status = status.clone();
            if status_changed {
                cx.notify();
            }
        }
        if let Ok(logs) = self.session.logs.lock() {
            if logs.len() != self.cached_logs.len() {
                self.cached_logs = logs.clone();
                cx.notify();
            }
        }
    }

    fn apply_profile_values(&mut self, profile: &Profile, cx: &mut Context<Self>) {
        self.bind_ip_input.update(cx, |input, cx| {
            input.set_text(&profile.bind_ip, cx);
        });
        self.bind_port_input.update(cx, |input, cx| {
            input.set_text(&profile.bind_port.to_string(), cx);
        });
        self.target_ip_input.update(cx, |input, cx| {
            input.set_text(&profile.target_ip, cx);
        });
        self.target_port_input.update(cx, |input, cx| {
            input.set_text(&profile.target_port.to_string(), cx);
        });
        self.keepalive_host_input.update(cx, |input, cx| {
            input.set_text(&profile.keepalive_host, cx);
        });
        self.keepalive_port_input.update(cx, |input, cx| {
            input.set_text(&profile.keepalive_port.to_string(), cx);
        });
        self.keepalive_interval_input.update(cx, |input, cx| {
            input.set_text(&profile.keepalive_interval.to_string(), cx);
        });
        self.udp_mode = profile.udp_mode;

        // Find forward method index
        let method = ForwardMethod::from_str(&profile.forward_method).unwrap_or(ForwardMethod::TestServer);
        self.forward_method_index = self.forward_methods.iter().position(|m| m == &method).unwrap_or(1);

        cx.notify();
    }

    fn build_config(&self, cx: &App) -> NatterConfig {
        let bind_ip = self.bind_ip_input.read(cx).text();
        let bind_port_str = self.bind_port_input.read(cx).text();
        let target_ip = self.target_ip_input.read(cx).text();
        let target_port_str = self.target_port_input.read(cx).text();
        let ka_host = self.keepalive_host_input.read(cx).text();
        let ka_port_str = self.keepalive_port_input.read(cx).text();
        let interval_str = self.keepalive_interval_input.read(cx).text();

        NatterConfig {
            udp_mode: self.udp_mode,
            bind_ip: if bind_ip.is_empty() { "0.0.0.0".into() } else { bind_ip },
            bind_port: bind_port_str.parse().unwrap_or(0),
            stun_servers: vec![],
            keepalive_host: ka_host,
            keepalive_port: ka_port_str.parse().unwrap_or(0),
            forward_method: self.forward_methods[self.forward_method_index].clone(),
            target_ip: if target_ip.is_empty() { "0.0.0.0".into() } else { target_ip },
            target_port: target_port_str.parse().unwrap_or(0),
            keepalive_interval: interval_str.parse().unwrap_or(15),
        }
    }

    fn build_profile(&self, cx: &App) -> Profile {
        let name = self.profile_name_input.read(cx).text();
        Profile {
            name: if name.is_empty() { "Unnamed".into() } else { name },
            udp_mode: self.udp_mode,
            bind_ip: self.bind_ip_input.read(cx).text(),
            bind_port: self.bind_port_input.read(cx).text().parse().unwrap_or(0),
            stun_servers: vec![],
            keepalive_host: self.keepalive_host_input.read(cx).text(),
            keepalive_port: self.keepalive_port_input.read(cx).text().parse().unwrap_or(0),
            forward_method: self.forward_methods[self.forward_method_index].display_name().into(),
            target_ip: self.target_ip_input.read(cx).text(),
            target_port: self.target_port_input.read(cx).text().parse().unwrap_or(0),
            keepalive_interval: self.keepalive_interval_input.read(cx).text().parse().unwrap_or(15),
        }
    }

    fn on_start_click(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if self.session.is_running() {
            return;
        }
        let config = self.build_config(cx);

        // Disable inputs
        self.set_inputs_enabled(false, cx);

        self.session = NatterSession::new();
        self.session.start(config);
        cx.notify();
    }

    fn on_stop_click(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.session.stop();
        self.set_inputs_enabled(true, cx);
        cx.notify();
    }

    fn set_inputs_enabled(&self, enabled: bool, cx: &mut Context<Self>) {
        self.bind_ip_input.update(cx, |i, cx| i.set_enabled(enabled, cx));
        self.bind_port_input.update(cx, |i, cx| i.set_enabled(enabled, cx));
        self.target_ip_input.update(cx, |i, cx| i.set_enabled(enabled, cx));
        self.target_port_input.update(cx, |i, cx| i.set_enabled(enabled, cx));
        self.keepalive_host_input.update(cx, |i, cx| i.set_enabled(enabled, cx));
        self.keepalive_port_input.update(cx, |i, cx| i.set_enabled(enabled, cx));
        self.keepalive_interval_input.update(cx, |i, cx| i.set_enabled(enabled, cx));
    }

    fn on_toggle_udp(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.session.is_running() {
            self.udp_mode = !self.udp_mode;
            cx.notify();
        }
    }

    fn on_cycle_forward_method(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.session.is_running() {
            self.forward_method_index = (self.forward_method_index + 1) % self.forward_methods.len();
            cx.notify();
        }
    }

    fn on_toggle_profiles(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.show_profile_panel = !self.show_profile_panel;
        cx.notify();
    }

    fn on_save_profile(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let profile = self.build_profile(cx);
        if let Some(idx) = self.selected_profile_index {
            self.profile_store.update_profile(idx, profile);
        } else {
            self.profile_store.add_profile(profile);
            self.selected_profile_index = Some(self.profile_store.profiles.len() - 1);
        }
        if let Some(idx) = self.selected_profile_index {
            self.profile_store.set_last_used(idx);
        }
        cx.notify();
    }

    fn get_link_url(&self) -> Option<String> {
        if let NatterStatus::Connected { outer_addr, .. } = &self.cached_status {
            let scheme = if self.udp_mode { "udp" } else { "http" };
            Some(format!("{}://{}", scheme, outer_addr))
        } else {
            None
        }
    }

    fn add_gui_log(&self, level: &str, message: &str) {
        let entry = LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            level: level.to_string(),
            message: message.to_string(),
        };
        if let Ok(mut logs) = self.session.logs.lock() {
            logs.push(entry);
        }
    }

    fn on_copy_link(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(url) = self.get_link_url() {
            cx.write_to_clipboard(ClipboardItem::new_string(url.clone()));
            self.add_gui_log("INFO", &format!("链接已复制: {}", url));
            cx.notify();
        }
    }

    fn on_open_link(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(url) = self.get_link_url() {
            #[cfg(target_os = "windows")]
            let result = std::process::Command::new("cmd").args(["/c", "start", "", &url]).spawn();
            #[cfg(target_os = "macos")]
            let result = std::process::Command::new("open").arg(&url).spawn();
            #[cfg(target_os = "linux")]
            let result = std::process::Command::new("xdg-open").arg(&url).spawn();

            match result {
                Ok(_) => self.add_gui_log("INFO", &format!("已在浏览器中打开: {}", url)),
                Err(e) => self.add_gui_log("ERROR", &format!("打开浏览器失败: {}", e)),
            }
            cx.notify();
        }
    }

    fn render_header(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_running = self.session.is_running();
        let status_text: SharedString = match &self.cached_status {
            NatterStatus::Idle => "就绪".into(),
            NatterStatus::Connecting => "连接中...".into(),
            NatterStatus::Connected { outer_addr, .. } => {
                format!("已连接: {}", outer_addr).into()
            }
            NatterStatus::Error(e) => format!("错误: {}", e).into(),
        };
        let status_color = match &self.cached_status {
            NatterStatus::Idle => Theme::text_secondary(),
            NatterStatus::Connecting => Theme::warning(),
            NatterStatus::Connected { .. } => Theme::success(),
            NatterStatus::Error(_) => Theme::error(),
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .w_full()
            .px(px(20.0))
            .py(px(12.0))
            .bg(Theme::bg_secondary())
            .border_b_1()
            .border_color(Theme::border())
            .child(
                div()
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(px(12.0))
                    .child(
                        div()
                            .text_size(px(18.0))
                            .text_color(Theme::accent())
                            .child("⬡ HolePatch"),
                    )
                    .child(
                        div()
                            .text_size(px(12.0))
                            .text_color(status_color)
                            .child(status_text),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(
                        div()
                            .id("profiles-btn")
                            .px(px(12.0))
                            .py(px(6.0))
                            .bg(Theme::bg_tertiary())
                            .border_1()
                            .border_color(Theme::border())
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .text_size(px(13.0))
                            .text_color(Theme::text_primary())
                            .child("📋 配置")
                            .hover(|s| s.bg(Theme::accent()).text_color(hsla(0.0, 0.0, 1.0, 1.0)))
                            .on_click(cx.listener(Self::on_toggle_profiles)),
                    )
                    .when(!is_running, |el| {
                        el.child(
                            div()
                                .id("start-btn")
                                .px(px(16.0))
                                .py(px(6.0))
                                .bg(Theme::accent())
                                .rounded(px(6.0))
                                .cursor_pointer()
                                .text_size(px(13.0))
                                .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                                .child("▶ 启动")
                                .hover(|s| s.bg(Theme::accent_hover()))
                                .on_click(cx.listener(Self::on_start_click)),
                        )
                    })
                    .when(is_running, |el| {
                        el.child(
                            div()
                                .id("stop-btn")
                                .px(px(16.0))
                                .py(px(6.0))
                                .bg(Theme::danger())
                                .rounded(px(6.0))
                                .cursor_pointer()
                                .text_size(px(13.0))
                                .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                                .child("⏹ 停止")
                                .hover(|s| s.bg(Theme::danger_hover()))
                                .on_click(cx.listener(Self::on_stop_click)),
                        )
                    })
                    .when(matches!(&self.cached_status, NatterStatus::Connected { .. }), |el| {
                        el.child(
                            div()
                                .id("copy-link-btn")
                                .px(px(12.0))
                                .py(px(6.0))
                                .bg(Theme::bg_tertiary())
                                .border_1()
                                .border_color(Theme::border())
                                .rounded(px(6.0))
                                .cursor_pointer()
                                .text_size(px(13.0))
                                .text_color(Theme::text_primary())
                                .child("复制链接")
                                .hover(|s| s.bg(Theme::accent()).text_color(hsla(0.0, 0.0, 1.0, 1.0)))
                                .on_click(cx.listener(Self::on_copy_link)),
                        )
                        .child(
                            div()
                                .id("open-browser-btn")
                                .px(px(12.0))
                                .py(px(6.0))
                                .bg(Theme::bg_tertiary())
                                .border_1()
                                .border_color(Theme::border())
                                .rounded(px(6.0))
                                .cursor_pointer()
                                .text_size(px(13.0))
                                .text_color(Theme::text_primary())
                                .child("浏览器打开")
                                .hover(|s| s.bg(Theme::accent()).text_color(hsla(0.0, 0.0, 1.0, 1.0)))
                                .on_click(cx.listener(Self::on_open_link)),
                        )
                    }),
            )
    }

    fn render_config_panel(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_running = self.session.is_running();
        let udp_label: SharedString = if self.udp_mode { "UDP ●".into() } else { "TCP ●".into() };
        let udp_color = if self.udp_mode { Theme::warning() } else { Theme::accent() };
        let method_name: SharedString = self.forward_methods[self.forward_method_index]
            .display_name()
            .to_string()
            .into();

        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .p(px(16.0))
            .w(px(320.0))
            .min_w(px(320.0))
            .bg(Theme::bg_secondary())
            .border_r_1()
            .border_color(Theme::border())
            // Protocol & Method selectors
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(Theme::text_secondary())
                                    .child("协议"),
                            )
                            .child(
                                div()
                                    .id("udp-toggle")
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .bg(Theme::bg_input())
                                    .border_1()
                                    .border_color(Theme::border())
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .text_size(px(14.0))
                                    .text_color(udp_color)
                                    .child(udp_label)
                                    .when(!is_running, |el| {
                                        el.hover(|s| s.border_color(Theme::border_focused()))
                                            .on_click(cx.listener(Self::on_toggle_udp))
                                    })
                                    .when(is_running, |el| el.opacity(0.5)),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .flex_1()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .text_color(Theme::text_secondary())
                                    .child("转发方式"),
                            )
                            .child(
                                div()
                                    .id("fwd-method")
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .bg(Theme::bg_input())
                                    .border_1()
                                    .border_color(Theme::border())
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .text_size(px(14.0))
                                    .text_color(Theme::text_primary())
                                    .child(method_name)
                                    .when(!is_running, |el| {
                                        el.hover(|s| s.border_color(Theme::border_focused()))
                                            .on_click(cx.listener(Self::on_cycle_forward_method))
                                    })
                                    .when(is_running, |el| el.opacity(0.5)),
                            ),
                    ),
            )
            // Bind settings
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(Theme::accent())
                    .child("— 绑定设置 —"),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(div().flex_1().child(self.bind_ip_input.clone()))
                    .child(div().w(px(100.0)).child(self.bind_port_input.clone())),
            )
            // Target settings
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(Theme::accent())
                    .child("— 转发目标 —"),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(div().flex_1().child(self.target_ip_input.clone()))
                    .child(div().w(px(100.0)).child(self.target_port_input.clone())),
            )
            // Keep-alive settings
            .child(
                div()
                    .text_size(px(13.0))
                    .text_color(Theme::accent())
                    .child("— 保活设置 —"),
            )
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap(px(8.0))
                    .child(div().flex_1().child(self.keepalive_host_input.clone()))
                    .child(div().w(px(70.0)).child(self.keepalive_port_input.clone()))
                    .child(div().w(px(70.0)).child(self.keepalive_interval_input.clone())),
            )
    }

    fn render_log_panel(&self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let logs = self.cached_logs.clone();

        div()
            .flex()
            .flex_col()
            .flex_1()
            .bg(Theme::bg_primary())
            .child(
                div()
                    .px(px(16.0))
                    .py(px(8.0))
                    .border_b_1()
                    .border_color(Theme::border())
                    .text_size(px(13.0))
                    .text_color(Theme::text_secondary())
                    .child(format!("日志 ({})", logs.len())),
            )
            .child(
                div()
                    .id("log-scroll")
                    .flex_1()
                    .overflow_y_scroll()
                    .px(px(12.0))
                    .py(px(8.0))
                    .children(logs.into_iter().map(|entry| {
                        let color = match entry.level.as_str() {
                            "DEBUG" => Theme::log_debug(),
                            "INFO" => Theme::log_info(),
                            "WARN" => Theme::log_warn(),
                            "ERROR" => Theme::log_error(),
                            _ => Theme::text_primary(),
                        };
                        let level_badge_bg = match entry.level.as_str() {
                            "ERROR" => Theme::error(),
                            "WARN" => Theme::warning(),
                            _ => Theme::bg_tertiary(),
                        };
                        div()
                            .flex()
                            .flex_row()
                            .items_start()
                            .gap(px(8.0))
                            .py(px(2.0))
                            .child(
                                div()
                                    .text_size(px(11.0))
                                    .text_color(Theme::text_secondary())
                                    .min_w(px(60.0))
                                    .child(SharedString::from(entry.timestamp)),
                            )
                            .child(
                                div()
                                    .text_size(px(10.0))
                                    .text_color(color)
                                    .bg(level_badge_bg)
                                    .px(px(4.0))
                                    .rounded(px(3.0))
                                    .min_w(px(38.0))
                                    .child(SharedString::from(entry.level)),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .text_size(px(12.0))
                                    .text_color(Theme::text_primary())
                                    .child(SharedString::from(entry.message)),
                            )
                    })),
            )
    }

    fn render_profile_panel(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let profiles = self.profile_store.profiles.clone();
        let selected = self.selected_profile_index;

        div()
            .flex()
            .flex_col()
            .w(px(260.0))
            .bg(Theme::bg_secondary())
            .border_l_1()
            .border_color(Theme::border())
            .child(
                div()
                    .px(px(12.0))
                    .py(px(8.0))
                    .border_b_1()
                    .border_color(Theme::border())
                    .text_size(px(13.0))
                    .text_color(Theme::text_secondary())
                    .child("保存的配置"),
            )
            // Profile list
            .child(
                div()
                    .id("profile-list")
                    .flex_1()
                    .overflow_y_scroll()
                    .px(px(8.0))
                    .py(px(4.0))
                    .children(profiles.iter().enumerate().map(|(idx, profile)| {
                        let is_selected = selected == Some(idx);
                        let bg = if is_selected {
                            Theme::accent()
                        } else {
                            Theme::bg_tertiary()
                        };
                        let text_color = if is_selected {
                            hsla(0.0, 0.0, 1.0, 1.0)
                        } else {
                            Theme::text_primary()
                        };
                        let name: SharedString = profile.name.clone().into();
                        let mode: SharedString = if profile.udp_mode { "UDP".into() } else { "TCP".into() };
                        let method: SharedString = profile.forward_method.clone().into();

                        div()
                            .id(ElementId::Name(format!("profile-{}", idx).into()))
                            .flex()
                            .flex_col()
                            .gap_1()
                            .px(px(10.0))
                            .py(px(6.0))
                            .my(px(2.0))
                            .bg(bg)
                            .rounded(px(6.0))
                            .cursor_pointer()
                            .child(
                                div()
                                    .text_size(px(13.0))
                                    .text_color(text_color)
                                    .child(name),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_row()
                                    .gap(px(6.0))
                                    .text_size(px(11.0))
                                    .text_color(if is_selected {
                                        hsla(0.0, 0.0, 0.9, 1.0)
                                    } else {
                                        Theme::text_secondary()
                                    })
                                    .child(mode)
                                    .child("·")
                                    .child(method),
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.selected_profile_index = Some(idx);
                                let profile = this.profile_store.profiles[idx].clone();
                                this.apply_profile_values(&profile, cx);
                                this.profile_store.set_last_used(idx);
                                cx.notify();
                            }))
                    })),
            )
            // Profile actions
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .p(px(8.0))
                    .border_t_1()
                    .border_color(Theme::border())
                    .child(self.profile_name_input.clone())
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .gap(px(6.0))
                            .child(
                                div()
                                    .id("save-profile-btn")
                                    .flex_1()
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .bg(Theme::accent())
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .text_size(px(12.0))
                                    .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                                    .child("💾 保存当前配置")
                                    .hover(|s| s.bg(Theme::accent_hover()))
                                    .on_click(cx.listener(Self::on_save_profile)),
                            )
                            .child(
                                div()
                                    .id("delete-profile-btn")
                                    .px(px(10.0))
                                    .py(px(6.0))
                                    .bg(Theme::danger())
                                    .rounded(px(6.0))
                                    .cursor_pointer()
                                    .text_size(px(12.0))
                                    .text_color(hsla(0.0, 0.0, 1.0, 1.0))
                                    .child("🗑")
                                    .hover(|s| s.bg(Theme::danger_hover()))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(idx) = this.selected_profile_index {
                                            this.profile_store.remove_profile(idx);
                                            this.selected_profile_index = None;
                                            cx.notify();
                                        }
                                    })),
                            ),
                    ),
            )
    }

    fn render_status_bar(&self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let status_text: SharedString = match &self.cached_status {
            NatterStatus::Idle => "空闲".into(),
            NatterStatus::Connecting => "正在建立 NAT 映射...".into(),
            NatterStatus::Connected { route_info, .. } => route_info.clone().into(),
            NatterStatus::Error(e) => e.clone().into(),
        };

        div()
            .flex()
            .flex_row()
            .items_center()
            .px(px(16.0))
            .py(px(4.0))
            .bg(Theme::bg_secondary())
            .border_t_1()
            .border_color(Theme::border())
            .text_size(px(11.0))
            .text_color(Theme::text_secondary())
            .child(status_text)
    }
}

impl Focusable for HolePatchApp {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for HolePatchApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let show_profiles = self.show_profile_panel;

        div()
            .track_focus(&self.focus_handle)
            .flex()
            .flex_col()
            .size_full()
            .bg(Theme::bg_primary())
            .text_color(Theme::text_primary())
            // Header
            .child(self.render_header(window, cx))
            // Main content
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_1()
                    .overflow_hidden()
                    // Config panel
                    .child(self.render_config_panel(window, cx))
                    // Log panel
                    .child(self.render_log_panel(window, cx))
                    // Profile panel (conditional)
                    .when(show_profiles, |el| {
                        el.child(self.render_profile_panel(window, cx))
                    }),
            )
            // Status bar
            .child(self.render_status_bar(window, cx))
    }
}

pub fn run() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1000.0), px(650.0)), cx);
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    titlebar: Some(TitlebarOptions {
                        title: Some("HolePatch - NAT Hole Punching GUI".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |_window, cx| cx.new(|cx| HolePatchApp::new(cx)),
            )
            .unwrap();

        window
            .update(cx, |_view, _window, cx| {
                cx.activate(true);
            })
            .unwrap();
    });
}
