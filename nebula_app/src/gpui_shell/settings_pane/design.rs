//! 设置页的设计 token 与行原语。
//!
//! 分组标题使用更大、加粗的主文本；设置项名称留在左列，控件在右列
//! 垂直居中。标准密度沿用原紧凑间距，紧凑档只再微收；字号、命中区域和窗口
//! 标题栏不随内容密度缩小。分组通过留白表达，不添加标题下横线。

use std::time::Duration;

use gpui::{Animation, AnimationExt as _, ElementId, FontWeight, ease_out_quint, relative};

use super::*;

/// 次要信息相对正文的字号比。
pub(super) const DESC_SCALE: f32 = 0.82;
/// Parent headings stay above item labels in the visual hierarchy.
const GROUP_TITLE_SCALE: f32 = 16.0 / 14.0;
/// 标准密度沿用原紧凑档的行留白；间距只由行内 padding 提供。
const ROW_PAD_Y: f32 = 12.0;
/// 紧凑档只再收紧少量留白，字号与控件命中区域保持不变。
const ROW_PAD_Y_COMPACT: f32 = 10.0;
/// 组与组。
pub(super) const GROUP_GAP: f32 = 48.0;
/// 轨道宽度。
const RAIL_W: f32 = 2.0;
/// 内容相对轨道的缩进。标题左对齐轨道本身、行内容缩进这么多——标题是命名者
/// 而不是组员，这个位置差比任何字重都更能表达层级。
pub(super) const RAIL_INDENT: f32 = 13.0;
/// 控件列宽。够放下最宽的下拉（220）加一点余量；开关这类窄控件在列内右对齐，
/// 右缘与下拉保持一致。文字列允许收缩换行，避免窄窗口被旧的 320px 下限撑宽。
const CTRL_COL_W: f32 = 232.0;
/// 脏值段升起的时长。
const MARK_RISE: Duration = Duration::from_millis(260);

#[derive(Clone, Copy)]
pub(super) enum RowLayout {
    Standard,
    IntrinsicControl,
}

/// One heading role for settings groups, including specialized feature pages.
pub(super) fn group_heading(
    title: impl Into<SharedString>,
    item_font_size: f32,
    cx: &App,
) -> gpui::Div {
    div()
        .text_size(px(item_font_size * GROUP_TITLE_SCALE))
        .font_weight(FontWeight::MEDIUM)
        .text_color(cx.theme().foreground)
        .child(title.into())
}

impl SettingsPane {
    pub(super) fn row_padding_y(&self) -> f32 {
        if self.runtime.density == nebula_settings::DensityName::Compact {
            ROW_PAD_Y_COMPACT
        } else {
            ROW_PAD_Y
        }
    }

    /// 一组设置的开头：标题出线。轨道不在这里画——它由组内每一行自己接续，
    /// 这样才能做到"同一条线，某几段是亮的"。
    pub(crate) fn group(&self, title: &'static str, cx: &Context<Self>) -> gpui::Div {
        let base_px = self.font_size_px(cx);
        // 组间距归 section 容器的 `gap`（HTML 原型里就是 `.a-main` 自己
        // `gap:24`），组不自带 `pt`：自带的话，首个元素不是分组的页会拿不到
        // 那段留白而直接贴住页头线，而首组又会拿到"正文上留白 + 组上留白"的
        // 双份。间距是**容器**的事，不是组的事。
        // 这里**不能**写 `w_full()`。`width:100%` 依赖父的已解析宽度，链条上
        // 任何一层是 auto / max-content，100% 就解析成内容宽——于是每个分组、
        // 每一行各按自己的文字长度取宽，控件右缘参差不齐（宽窗口下差到 280px，
        // 窄窗口反而齐，因为那时被可用宽度压住了）。
        //
        // 不设宽度则走 flex 交叉轴 stretch：布局算法直接拉伸，不依赖父宽解析。
        v_flex().w_full().child(group_heading(title, base_px, cx).pb(px(20.0)))
    }

    /// 组与组之间的间隔。
    ///
    /// 原来这里是 32px 容器夹一条 1px 细线。线现在多余了：分组的范围由左侧
    /// 轨道表达，横线再圈一道等于同一件事说两遍，而且横线会把"一组"读成
    /// "一段到此为止"——前者是归属，后者是切断。
    /// 保留给"两块内容之间需要一口气"的非分组场景（关于页把外链沉到下面就
    /// 用它）。分组之间**不要**用它——组自己带上留白，再插一段就是两倍。
    pub(crate) fn group_divider(_cx: &Context<Self>) -> gpui::Div {
        div().w_full().h(px(GROUP_GAP - 8.0)).flex_shrink_0()
    }

    /// 设置行显示名称和控件。
    pub(crate) fn row(
        &self,
        label: &'static str,
        control: impl IntoElement,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        self.row_shell(label, None, false, RowLayout::Standard, control, cx)
    }

    /// 带撤销的设置行：该项被覆盖过时，左侧轨道这一段亮起来，行内出现 ↶。
    ///
    /// 两个信号的时态不同，所以显示时机也不同：
    /// - 轨道亮色是**状态**（"这台机器上我动过它"），必须常显才能一眼扫完；
    /// - ↶ 是**动作**，只有真想撤销时才有用，常显就是噪音——所以 hover 才现。
    pub(crate) fn row_with_reset(
        &self,
        label: &'static str,
        dirty: bool,
        on_reset: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        control: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        self.row_with_reset_layout(label, dirty, on_reset, RowLayout::Standard, control, cx)
    }

    pub(super) fn row_with_reset_layout(
        &self,
        label: &'static str,
        dirty: bool,
        on_reset: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
        layout: RowLayout,
        control: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let hover_group = Self::row_hover_group(label);
        let reset = dirty.then(|| {
            div()
                .id(SharedString::from(format!("setting-reset-{label}")))
                .size(px(32.0))
                .rounded_md()
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .invisible()
                .group_hover(hover_group, |el| el.visible())
                .hover(|el| el.bg(cx.theme().list_hover))
                .tooltip(|window, cx| {
                    gpui_component::tooltip::Tooltip::new(
                        crate::gpui_shell::config::ui_language(cx)
                            .pick("还原为默认值", "Restore default"),
                    )
                    .build(window, cx)
                })
                .on_click(cx.listener(move |this, _, window, cx| on_reset(this, window, cx)))
                .child(Icon::new(IconName::Undo2).size(px(16.0)))
                .into_any_element()
        });
        self.row_shell(label, reset, dirty, layout, control, cx)
    }

    /// 行 hover 组名。↶ 要跟着**整行**的 hover 显形，而不是自己被指到才现
    /// ——后者等于让用户先找到一个看不见的东西。
    fn row_hover_group(label: &'static str) -> SharedString {
        SharedString::from(format!("settings-row-{label}"))
    }

    pub(super) fn row_shell(
        &self,
        label: &'static str,
        reset: Option<gpui::AnyElement>,
        dirty: bool,
        layout: RowLayout,
        control: impl IntoElement,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let base_px = self.font_size_px(cx);
        let pad_y = self.row_padding_y();
        let text = v_flex().child(
            h_flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .min_w_0()
                        .text_size(px(base_px))
                        .font_weight(FontWeight::NORMAL)
                        .text_color(theme.foreground)
                        .child(label),
                )
                .children(reset),
        );
        let control = control.into_any_element();
        let columns = match layout {
            RowLayout::IntrinsicControl => {
                // 胶囊依实际文字取宽；空间不足时控件整块换行，不缩字号或截断选项。
                h_flex()
                    .w_full()
                    .items_center()
                    .flex_wrap()
                    .gap_4()
                    .child(text.flex_1().min_w(px(180.0)))
                    .child(h_flex().flex_grow(1.0).justify_end().max_w_full().child(control))
            },
            RowLayout::Standard => {
                h_flex().w_full().items_center().gap_4().child(text.flex_1().min_w_0()).child(
                    h_flex()
                        .w(px(CTRL_COL_W))
                        .flex_shrink_0()
                        .justify_end()
                        .items_center()
                        .child(control),
                )
            },
        };
        div()
            .id(label)
            .group(Self::row_hover_group(label))
            .relative()
            .w_full()
            .flex_shrink_0()
            .pl(px(RAIL_INDENT))
            .pr_4()
            .py(px(pad_y))
            // 竖线整条让给状态，不再画常驻的灰轨道。
            //
            // 灰线原本表达"这几行是一组"，但那件事组标题说了一遍、24px 组间
            // 距又说了一遍，第三遍是冗余；而"这行被改过"没有别的元素在说。
            // 一个视觉通道只能有一个主人，所以给不可替代的那个。
            //
            // 何况暗色下灰线必须淡到几乎看不见才不抢戏，一旦提亮到能看清，就
            // 会和左侧导航的分割线形成两条平行的近距离竖线——页面开始变格子。
            //
            // 组的层级仍然成立：标题的字重与颜色、组间距、以及行内容相对标题
            // 的 13px 缩进。缩进不需要一条线来证明自己存在。
            .when(dirty, |row| {
                row.child(
                    div()
                        .absolute()
                        .left_0()
                        .bottom_0()
                        .w(px(RAIL_W))
                        .bg(crate::gpui_shell::theme::settings_mark(cx))
                        .with_animation(
                            ElementId::Name(format!("settings-mark-{label}").into()),
                            Animation::new(MARK_RISE).with_easing(ease_out_quint()),
                            |mark, t| mark.h(relative(t)),
                        ),
                )
            })
            .child(columns)
    }
}
