use super::*;

impl SettingsPane {
    pub(super) fn appearance_advanced_settings(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        let language = crate::gpui_shell::config::ui_language(cx);
        let font_picker = self.font_picker_dropdown(window, cx);
        let cjk_font_picker = self.cjk_font_picker_dropdown(window, cx);
        let opacity: SharedString = format!("{:.0}%", self.runtime.opacity * 100.0).into();
        let wallpaper_opacity: SharedString =
            format!("{:.0}%", self.runtime.background_image_opacity * 100.0).into();
        let custom_background = self
            .group(language.pick("自定义背景", "Custom background"), cx)
            .child(self.background_color_row(cx))
            .child(self.select_row(
                "background_media_kind",
                language.text(crate::i18n::Message::WallpaperKind),
                cx,
            ))
            .child(self.background_image_row(cx))
            .child(self.select_row(
                "background_shader_preset",
                language.text(crate::i18n::Message::WallpaperShader),
                cx,
            ))
            .when(self.shader_custom_open || self.runtime.background_effects.wgsl, |group| {
                group.child(self.shader_source_row(cx))
            })
            .child(self.select_row(
                "background_image_fit",
                language.pick("背景图像拉伸模式", "Background image fit"),
                cx,
            ))
            .child(self.select_row(
                "background_image_alignment",
                language.pick("背景图像对齐", "Background image alignment"),
                cx,
            ))
            .child(self.slider_row(
                language.pick("背景图像不透明度", "Background image opacity"),
                &self.wallpaper_opacity_slider,
                wallpaper_opacity,
                cx,
            ))
            .child(self.switch_row(
                "background_image_cover_chrome",
                language.pick(
                    "将背景图扩展到标题栏和侧边栏",
                    "Extend the background image into the title bar and sidebar",
                ),
                self.runtime.background_image_cover_chrome,
                cx,
            ))
            .child(self.custom_effect_settings(window, cx));
        let cursor = self
            .group(language.pick("光标", "Cursor"), cx)
            .child(self.select_row(
                "cursor_motion",
                language.text(crate::i18n::Message::SettingsCursorMotionTitle),
                cx,
            ))
            .child(self.select_row("cursor_shape", language.pick("光标形状", "Cursor shape"), cx))
            .child(self.switch_row(
                "cursor_blink",
                language.pick("光标闪烁", "Blink cursor"),
                self.runtime.cursor_blink.unwrap_or(DEFAULT_CURSOR_BLINK),
                cx,
            ));
        let interface = self
            .group(language.pick("界面", "Interface"), cx)
            .child(self.font_size_row(true, cx))
            .child(self.switch_row(
                "dim_inactive_panes",
                language.text(crate::i18n::Message::SettingsPanesDimInactive),
                self.runtime.dim_inactive_panes,
                cx,
            ))
            .child(self.switch_row(
                "tab_close_visible",
                language.pick("显示标签关闭按钮", "Show tab close buttons"),
                self.runtime.tab_close_visible,
                cx,
            ))
            .child(self.select_row(
                "language",
                language.text(crate::i18n::Message::CommonLanguage),
                cx,
            ))
            .child(self.select_row("density", language.pick("界面外观", "Interface density"), cx))
            .child(self.slider_row(
                language.pick("终端正文不透明度", "Terminal content opacity"),
                &self.opacity_slider,
                opacity,
                cx,
            ))
            .child(self.select_row("blur", language.pick("背景模糊", "Background blur"), cx));
        let terminal = self
            .group(language.pick("终端外观", "Terminal appearance"), cx)
            .child(self.row(
                language.text(crate::i18n::Message::SettingsFontEnglish),
                font_picker,
                cx,
            ))
            .child(self.row(
                language.text(crate::i18n::Message::SettingsFontChinese),
                cjk_font_picker,
                cx,
            ))
            .child(self.font_size_row(false, cx))
            .child(self.switch_row(
                "ctrl_wheel_font_zoom",
                language.text(crate::i18n::Message::SettingsFontCtrlWheelZoom),
                self.runtime.ctrl_wheel_font_zoom,
                cx,
            ))
            .child(self.select_row(
                "ligatures",
                language.text(crate::i18n::Message::SettingsFontLigatures),
                cx,
            ))
            .child(self.select_row(
                "cell_width_mode",
                language.pick("字体间距", "Character spacing"),
                cx,
            ))
            .child(self.select_row(
                "scrollback_lines",
                language.text(crate::i18n::Message::SettingsScrollingHistory),
                cx,
            ))
            .child(self.scroll_speed_row(cx))
            .child(self.select_row(
                "scrollbar_visibility",
                language.text(crate::i18n::Message::SettingsScrollingScrollbar),
                cx,
            ))
            .child(self.switch_row(
                "fetch",
                language.pick("启动欢迎信息", "Startup system information"),
                self.runtime.fetch,
                cx,
            ))
            .child(self.switch_row(
                "powerline",
                language.pick("Powerline 提示符", "Powerline prompt"),
                self.runtime.powerline,
                cx,
            ));

        v_flex()
            .w_full()
            .gap(px(GROUP_GAP))
            .child(terminal)
            .child(cursor)
            .child(interface)
            .child(custom_background)
    }

    pub(super) fn custom_effect_settings(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> gpui::Div {
        use crate::i18n::Message;
        let language = crate::gpui_shell::config::ui_language(cx);
        let expanded = self.effect_settings_open;
        // 展开状态只属于设置页；收起高级入口不能停用或改写用户的效果配置。
        v_flex()
            .w_full()
            .child(
                Button::new("custom-effects-disclosure")
                    .debug_selector(|| "custom-effects-disclosure".to_owned())
                    .ghost()
                    .icon(if expanded { IconName::ChevronDown } else { IconName::ChevronRight })
                    .label(language.text(if self.runtime.terminal_effects.enabled {
                        Message::TerminalEffectAdvancedActive
                    } else {
                        Message::TerminalEffectAdvanced
                    }))
                    .toggled(expanded)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.effect_settings_open = !this.effect_settings_open;
                        cx.notify();
                    })),
            )
            .when(expanded, |group| {
                group.child(self.terminal_effect_row(window, cx)).child(self.select_row(
                    "terminal_effect_animation",
                    language.text(Message::TerminalEffectAnimation),
                    cx,
                ))
            })
    }
}
