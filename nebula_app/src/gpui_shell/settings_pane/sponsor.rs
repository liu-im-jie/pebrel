//! 设置首页「项目与支持」下的赞助商页：品牌贴图、名称与注册链接。

use std::sync::{Arc, OnceLock};

use super::*;

/// 注册链接。`source` / `campaign` / `promo` 三个参数是赞助商给本项目的
/// 专属归因，README 与应用内必须用同一条，不能各写各的。
pub(super) const SPONSOR_URL: &str =
    "https://fluxionai.space/register?source=github&campaign=pebrel&promo=pebrel";
const SPONSOR_NAME: &str = "Fluxion AI";
const LOGO_PNG: &[u8] = include_bytes!("../../../../extra/logo/sponsor_fluxionai.png");
/// 贴图自带白底圆角，深浅主题都读得清品牌字。
const LOGO_WIDTH: f32 = 188.0;

/// 贴图只解码一次：RGBA8 → BGRA（gpui 帧通道序），与壁纸 / 文档图同款。
fn sponsor_logo() -> Option<Arc<RenderImage>> {
    static LOGO: OnceLock<Option<Arc<RenderImage>>> = OnceLock::new();
    LOGO.get_or_init(|| {
        let mut rgba = image::load_from_memory(LOGO_PNG).ok()?.into_rgba8();
        for pixel in rgba.chunks_exact_mut(4) {
            pixel.swap(0, 2);
        }
        Some(Arc::new(RenderImage::new([image::Frame::new(rgba)])))
    })
    .clone()
}

impl SettingsPane {
    pub(super) fn open_sponsor_page(&mut self, cx: &mut Context<Self>) {
        self.about_sponsor_open = true;
        cx.notify();
    }

    pub(super) fn close_sponsor_page(&mut self, cx: &mut Context<Self>) {
        self.about_sponsor_open = false;
        cx.notify();
    }

    pub(super) fn section_sponsor(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let language = crate::gpui_shell::config::ui_language(cx);
        let theme = cx.theme();
        let muted = theme.muted_foreground;
        let ink = theme.foreground;
        let link = theme.primary;
        let hover = theme.list_hover;
        let base_px = self.font_size_px(cx);

        let back = h_flex()
            .id("sponsor-back")
            .w_auto()
            .h(px(32.0))
            .px_2()
            .gap_2()
            .items_center()
            .rounded_md()
            .cursor_pointer()
            .text_color(muted)
            .hover(move |row| row.bg(hover).text_color(ink))
            .on_click(cx.listener(|this, _, _, cx| this.close_sponsor_page(cx)))
            .child(Icon::new(IconName::ArrowLeft).small())
            .child(language.pick("返回", "Back"));

        let logo =
            div().w(px(LOGO_WIDTH)).flex_shrink_0().when_some(sponsor_logo(), |slot, image| {
                slot.child(gpui::StyledImage::object_fit(
                    img(image).w(px(LOGO_WIDTH)).h(px(LOGO_WIDTH * 276.0 / 376.0)),
                    gpui::ObjectFit::Contain,
                ))
            });

        let register = h_flex()
            .id("sponsor-register")
            .w_auto()
            .mt(px(4.0))
            .gap_1()
            .items_center()
            .cursor_pointer()
            .text_color(link)
            .hover(|row| row.underline())
            .on_click(|_, _, cx| cx.open_url(SPONSOR_URL))
            .child(language.text(crate::i18n::Message::CommonOpen))
            .child(Icon::new(IconName::ExternalLink).xsmall());

        let copy = v_flex()
            .flex_1()
            .min_w_0()
            .gap(px(10.0))
            .child(
                div()
                    .text_size(px(base_px * 1.3))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(ink)
                    .child(SPONSOR_NAME),
            )
            .child(register);

        v_flex()
            .w_full()
            .gap(px(20.0))
            .child(back)
            .child(
                div()
                    .h(px(30.0))
                    .text_size(px(base_px * 0.85))
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_color(muted)
                    .child(language.pick("赞助商", "Sponsors")),
            )
            .child(h_flex().w_full().items_start().gap(px(28.0)).child(logo).child(copy))
    }
}
