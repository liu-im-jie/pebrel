use super::*;

#[gpui::test]
fn settings_rows_keep_controls_without_explanatory_text(cx: &mut gpui::TestAppContext) {
    cx.update(|cx| {
        gpui_component::init(cx);
        cx.set_reduce_motion(true);
        cx.set_global(crate::gpui_shell::config::Settings::load(ThemeName::Nord));
    });
    let mut pane = None;
    let (_, cx) = cx.add_window_view(|window, cx| {
        let view = cx.new(|cx| SettingsPane::new(window, cx));
        pane = Some(view.clone());
        gpui_component::Root::new(view, window, cx)
    });
    let pane = pane.unwrap();
    for language in [crate::display::UiLanguage::ZhCn, crate::display::UiLanguage::EnUs] {
        cx.update(|_, cx| {
            cx.global_mut::<crate::gpui_shell::config::Settings>().ui_language = language;
            pane.update(cx, |pane, cx| {
                pane.active_section = 1;
                cx.notify();
            });
        });
        cx.simulate_resize(gpui::size(px(1000.0), px(1000.0)));
        cx.run_until_parked();
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
        });
        assert!(cx.debug_bounds("open-theme-picker").is_some());
        assert!(cx.debug_bounds("open-icon-picker").is_some());
        let rows = include_str!("design.rs");
        assert!(!rows.contains("settings-help-"));
        assert!(!rows.contains("desc_text("));
    }
}
