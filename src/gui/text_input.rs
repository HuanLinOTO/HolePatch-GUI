use gpui::*;
use gpui::prelude::FluentBuilder;

/// A simple text input component that handles keyboard input via on_key_down.
/// This avoids the complexity of EntityInputHandler for a straightforward use case.
#[derive(Clone)]
pub struct TextInput {
    pub id: ElementId,
    pub content: SharedString,
    pub placeholder: SharedString,
    pub focus_handle: FocusHandle,
    pub label: SharedString,
    pub enabled: bool,
}

impl TextInput {
    pub fn new(
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) -> Self {
        TextInput {
            id: id.into(),
            content: "".into(),
            placeholder: placeholder.into(),
            focus_handle: cx.focus_handle(),
            label: label.into(),
            enabled: true,
        }
    }

    pub fn text(&self) -> String {
        self.content.to_string()
    }

    pub fn set_text(&mut self, text: &str, cx: &mut Context<Self>) {
        self.content = text.to_string().into();
        cx.notify();
    }

    pub fn set_enabled(&mut self, enabled: bool, cx: &mut Context<Self>) {
        self.enabled = enabled;
        cx.notify();
    }

    fn handle_key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        if !self.enabled {
            return;
        }

        let key = &event.keystroke.key;
        let modifiers = &event.keystroke.modifiers;

        match key.as_str() {
            "backspace" => {
                let mut text = self.content.to_string();
                if modifiers.control || modifiers.platform {
                    text.clear();
                } else {
                    text.pop();
                }
                self.content = text.into();
                cx.notify();
            }
            _ => {
                // Use key_char if available for proper character input
                if let Some(key_char) = &event.keystroke.key_char {
                    if !modifiers.control && !modifiers.platform {
                        let mut text = self.content.to_string();
                        text.push_str(key_char);
                        self.content = text.into();
                        cx.notify();
                    }
                }
            }
        }
    }
}

impl Focusable for TextInput {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TextInput {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let is_focused = self.focus_handle.is_focused(window);
        let has_content = !self.content.is_empty();
        let border_color = if is_focused {
            super::theme::Theme::border_focused()
        } else {
            super::theme::Theme::border()
        };

        div()
            .flex()
            .flex_col()
            .gap_1()
            .w_full()
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(super::theme::Theme::text_secondary())
                    .child(self.label.clone()),
            )
            .child(
                div()
                    .id(self.id.clone())
                    .track_focus(&self.focus_handle)
                    .on_key_down(cx.listener(Self::handle_key_down))
                    .px(px(10.0))
                    .py(px(6.0))
                    .w_full()
                    .bg(super::theme::Theme::bg_input())
                    .border_1()
                    .border_color(border_color)
                    .rounded(px(6.0))
                    .cursor(CursorStyle::IBeam)
                    .text_size(px(14.0))
                    .when(has_content, |el| {
                        el.text_color(super::theme::Theme::text_primary())
                            .child(self.content.clone())
                    })
                    .when(!has_content, |el| {
                        el.text_color(super::theme::Theme::text_placeholder())
                            .child(self.placeholder.clone())
                    })
                    .when(!self.enabled, |el| {
                        el.opacity(0.5)
                    }),
            )
    }
}
