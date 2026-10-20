//! Workspace SSH navigation and terminal launch commands.

use super::*;
use crate::gpui_shell::ssh_hosts::SshHostLists;
use crate::i18n::Message;
use std::collections::{BTreeMap, HashSet};

#[cfg(test)]
mod tests;

struct Host {
    destination: String,
    label: String,
    glyph: char,
}

struct HostGroup {
    name: String,
    hosts: Vec<Host>,
}

fn grouped_hosts(lists: &SshHostLists) -> Vec<HostGroup> {
    let icons = lists.profiles.icons();
    let mut groups = BTreeMap::<String, Vec<Host>>::new();
    for (destination, label) in lists.merged_with_labels() {
        let group = lists.profiles.organization(&destination).group.trim().to_owned();
        let glyph =
            crate::display::ui::os_icons::resolve(icons.get(&destination).map(String::as_str))
                .glyph;
        let label = if label.is_empty() {
            lists.profiles.connection_destination(&destination).to_owned()
        } else {
            label
        };
        groups.entry(group).or_default().push(Host { destination, label, glyph });
    }
    let ungrouped = groups.remove("");
    let mut result: Vec<_> =
        groups.into_iter().map(|(name, hosts)| HostGroup { name, hosts }).collect();
    if let Some(hosts) = ungrouped {
        result.push(HostGroup { name: String::new(), hosts });
    }
    result
}

/// Presentation state only; hosts, grouping and connection identities remain in
/// the existing library. Dropping/replacing `load` discards an obsolete refresh.
#[derive(Default)]
pub(super) struct SidebarState {
    expanded: bool,
    groups: Vec<HostGroup>,
    collapsed_groups: HashSet<String>,
    count: Option<usize>,
    load: Option<gpui::Task<()>>,
    scroll: gpui::ScrollHandle,
}

impl SidebarState {
    pub(super) fn expanded(&self) -> bool {
        self.expanded
    }

    fn replace_hosts(&mut self, groups: Vec<HostGroup>) {
        self.collapsed_groups.retain(|name| groups.iter().any(|group| group.name == *name));
        self.count = Some(groups.iter().map(|group| group.hosts.len()).sum());
        self.groups = groups;
    }

    fn toggle_group(&mut self, name: &str) {
        if !self.collapsed_groups.remove(name) {
            self.collapsed_groups.insert(name.to_owned());
        }
    }

    fn visible_row_count(&self) -> usize {
        self.groups
            .iter()
            .map(|group| {
                1 + if self.collapsed_groups.contains(&group.name) { 0 } else { group.hosts.len() }
            })
            .sum()
    }
}

impl NebulaWorkspace {
    pub(super) fn refresh_ssh_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let worker = cx.background_executor().spawn(async {
            let lists = SshHostLists::load();
            (grouped_hosts(&lists), lists.load_error)
        });
        self.ssh_sidebar.load = Some(cx.spawn_in(window, async move |this, cx| {
            let (groups, error) = worker.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.ssh_sidebar.load = None;
                if let Some(error) = error {
                    if this.ssh_sidebar.expanded {
                        crate::gpui_shell::toast::toast(
                            window,
                            cx,
                            crate::display::ToastKind::Warning,
                            crate::gpui_shell::settings_pane::SshStatus::ProfileLoadFailed(error)
                                .text(workspace_ui_language()),
                        );
                    }
                } else {
                    this.ssh_sidebar.replace_hosts(groups);
                }
                cx.notify();
            });
        }));
    }

    fn toggle_ssh_sidebar(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.ssh_sidebar.expanded = !self.ssh_sidebar.expanded;
        self.tab_menu = None;
        if self.ssh_sidebar.expanded {
            self.refresh_ssh_sidebar(window, cx);
        }
        cx.notify();
    }

    pub(super) fn render_ssh_sidebar_header(&self, cx: &mut Context<Self>) -> impl IntoElement {
        sidebar::section_header(
            "sidebar-ssh-toggle",
            workspace_ui_language().text(Message::SettingsSidebarSsh),
            self.ssh_sidebar.count,
            !self.ssh_sidebar.expanded,
            self.density,
            cx,
        )
        .on_click(cx.listener(|this, _, window, cx| this.toggle_ssh_sidebar(window, cx)))
        .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
            let expanded = this.ssh_sidebar.expanded;
            match event.keystroke.key.as_str() {
                "enter" | "space" => this.toggle_ssh_sidebar(window, cx),
                "right" if !expanded => this.toggle_ssh_sidebar(window, cx),
                "left" if expanded => this.toggle_ssh_sidebar(window, cx),
                "down" if expanded => {
                    this.ssh_sidebar.scroll.scroll_to_item(0);
                    window.focus_next(cx);
                },
                "up" => window.focus_prev(cx),
                "escape" => this.focus_active(window, cx),
                _ => return,
            }
            cx.stop_propagation();
        }))
        .when(self.ssh_sidebar.expanded && self.ssh_sidebar.load.is_some(), |header| {
            header.child(div().flex_1()).child(Spinner::new().xsmall())
        })
    }

    pub(super) fn render_ssh_sidebar_list(
        &self,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        if !self.ssh_sidebar.expanded || self.ssh_sidebar.groups.is_empty() {
            return None;
        }
        let gap = tab_scroll::tab_row_gap(self.density);
        let mut rows = Vec::new();
        for (group_ix, group) in self.ssh_sidebar.groups.iter().enumerate() {
            rows.push(self.render_ssh_sidebar_row(group_ix, None, rows.len(), cx));
            if !self.ssh_sidebar.collapsed_groups.contains(&group.name) {
                for host_ix in 0..group.hosts.len() {
                    rows.push(self.render_ssh_sidebar_row(group_ix, Some(host_ix), rows.len(), cx));
                }
            }
        }
        Some(
            div()
                .id("sidebar-ssh-list-slot")
                .debug_selector(|| "sidebar-ssh-list-slot".to_owned())
                .relative()
                .flex_1()
                .min_h_0()
                .w_full()
                .overflow_hidden()
                .child(
                    v_flex()
                        .id("sidebar-ssh-list")
                        .size_full()
                        .gap(px(gap))
                        .pr(px(tab_scroll::TAB_SCROLL_GUTTER))
                        .overflow_y_scroll()
                        .track_scroll(&self.ssh_sidebar.scroll)
                        .children(rows),
                )
                .child(
                    div().absolute().top_0().right_0().bottom_0().w(px(16.0)).child(
                        gpui_component::scroll::Scrollbar::vertical(&self.ssh_sidebar.scroll),
                    ),
                )
                .into_any_element(),
        )
    }

    fn render_ssh_sidebar_row(
        &self,
        group_ix: usize,
        host_ix: Option<usize>,
        row_ix: usize,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let group = &self.ssh_sidebar.groups[group_ix];
        let host = host_ix.map(|ix| &group.hosts[ix]);
        let collapsed = self.ssh_sidebar.collapsed_groups.contains(&group.name);
        let label = host.map(|host| host.label.clone()).unwrap_or_else(|| {
            if group.name.is_empty() {
                workspace_ui_language().text(Message::HostsUngrouped).to_owned()
            } else {
                group.name.clone()
            }
        });
        let selected = host.is_some_and(|host| {
            !self.settings_open
                && self.tabs.get(self.active).and_then(|tab| tab.focused_view()).is_some_and(
                    |view| {
                        view.read(cx).ssh_destination.as_deref() == Some(host.destination.as_str())
                    },
                )
        });
        let theme = cx.theme();
        let hover = theme.list_hover;
        let active = theme.sidebar_accent;
        let foreground = theme.sidebar_foreground;
        let label_px = cx
            .try_global::<crate::gpui_shell::config::Settings>()
            .map(|settings| settings.ui_font_size_px)
            .unwrap_or(15.0);
        let destination = host.map(|host| host.destination.clone());
        let click_destination = destination.clone();
        let group_name = group.name.clone();
        let click_group = group_name.clone();
        let id: SharedString = match host {
            Some(host) => format!("sidebar-ssh-host:{}", host.destination).into(),
            None => format!("sidebar-ssh-group:{}", group.name).into(),
        };
        h_flex()
            .id(id)
            .debug_selector(move || match host_ix {
                Some(ix) => format!("sidebar-ssh-host-{group_ix}-{ix}"),
                None => format!("sidebar-ssh-group-{group_ix}"),
            })
            .focusable()
            .tab_stop(true)
            .role(gpui::Role::Button)
            .w_full()
            .min_w_0()
            .h(px(tab_scroll::tab_row_height(self.density)))
            .flex_shrink_0()
            .pl(px(if host.is_some() { 40.0 } else { 16.0 }))
            .pr_2()
            .items_center()
            .rounded_md()
            .overflow_hidden()
            .cursor_pointer()
            .font_family(theme.font_family.clone())
            .text_size(px(label_px))
            .text_color(if selected { theme.sidebar_accent_foreground } else { foreground })
            .when(selected, |row| row.bg(active))
            .hover(move |row| row.bg(hover))
            .active(move |row| row.bg(active))
            .focus_visible(move |row| row.bg(hover))
            .on_click(cx.listener(move |this, _, window, cx| {
                cx.stop_propagation();
                if let Some(destination) = &click_destination {
                    this.add_ssh_terminal(destination.clone(), window, cx);
                } else {
                    this.ssh_sidebar.toggle_group(&click_group);
                    cx.notify();
                }
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                match event.keystroke.key.as_str() {
                    "enter" | "space" => {
                        if let Some(destination) = &destination {
                            this.add_ssh_terminal(destination.clone(), window, cx);
                        } else {
                            this.ssh_sidebar.toggle_group(&group_name);
                        }
                    },
                    "left" if destination.is_none() => {
                        this.ssh_sidebar.collapsed_groups.insert(group_name.clone());
                    },
                    "right" if destination.is_none() => {
                        this.ssh_sidebar.collapsed_groups.remove(&group_name);
                    },
                    "up" => {
                        this.ssh_sidebar.scroll.scroll_to_item(row_ix.saturating_sub(1));
                        window.focus_prev(cx);
                    },
                    "down" => {
                        let last = this.ssh_sidebar.visible_row_count().saturating_sub(1);
                        this.ssh_sidebar.scroll.scroll_to_item((row_ix + 1).min(last));
                        window.focus_next(cx);
                    },
                    "escape" => this.focus_active(window, cx),
                    _ => return,
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .child(
                div().w(px(24.0)).flex_shrink_0().child(match host {
                    Some(host) => div()
                        .font_family(crate::font_install::REQUIRED_FONT_FAMILY)
                        .child(host.glyph.to_string())
                        .into_any_element(),
                    None => Icon::new(if collapsed {
                        IconName::ChevronRight
                    } else {
                        IconName::ChevronDown
                    })
                    .xsmall()
                    .text_color(theme.muted_foreground)
                    .into_any_element(),
                }),
            )
            .child(div().flex_1().min_w_0().truncate().child(label))
            .into_any_element()
    }

    /// 打开一个 SSH tab（启动器/设置页共用入口）：russh 直连业务层，连接
    /// 阶段/失败原因由业务层回流到 pane（横幅 + grid）。非 config 源的
    /// 目的地自动进保存列表（旧壳 spawn_tab_ssh 同语义）。
    pub fn add_ssh_terminal(
        &mut self,
        destination: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.add_ssh_terminal_at(destination, None, window, cx);
    }

    pub(super) fn add_ssh_terminal_at(
        &mut self,
        destination: String,
        remote_cwd: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.settings_open {
            self.leave_settings(window, cx);
        }
        {
            let mut lists = crate::gpui_shell::ssh_hosts::SshHostLists::load();
            if !lists.is_from_config(&destination) {
                lists.remember(&destination);
                if let Err(err) = lists.persist() {
                    log::warn!("持久化 SSH 主机列表失败: {err}");
                }
            }
        }
        let grid = self.inherited_grid(cx);
        let launch = crate::gpui_shell::terminal::view::TerminalLaunch::Ssh {
            destination: destination.clone(),
            cwd: remote_cwd,
        };
        let pane = self.new_pane(grid, launch, None, window, cx);
        let focused = pane.id;
        let tab = WorkspaceTab::Terminal {
            tree: SplitTree::leaf(pane.id),
            panes: vec![pane],
            focused,
            zoomed: false,
            broadcast: false,
        };
        let position = nebula_settings::RuntimeSettings::load().new_tab_position;
        let at = new_tab_insert_index(position, self.active, self.tabs.len());
        self.insert_tab_at(
            at,
            tab,
            TabMeta {
                shell_tag: Some("ssh".into()),
                launch: Some(crate::session::LaunchSession::Ssh { host: destination }),
                ..TabMeta::default()
            },
        );
        self.active = at;
        self.reveal_active_tab();
        self.focus_active(window, cx);
        self.sync_side_panel_to_active(true, cx);
        self.refresh_ssh_sidebar(window, cx);
        cx.notify();
    }

    /// 在同一 tab、同一分屏位置替换失败的 SSH pane。先把新实体及订阅完整
    /// 建好，再原子替换树叶和 pane 所有权；旧实体的异步泵只会更新旧 Entity，
    /// 因而无法把迟到的 Failed/Ready 写进新连接。
    pub(super) fn retry_ssh_pane(
        &mut self,
        tab_ix: usize,
        pane_id: u64,
        destination: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(WorkspaceTab::Terminal { panes, .. }) = self.tabs.get(tab_ix) else { return };
        let Some(old) = panes.iter().find(|pane| pane.id == pane_id) else { return };
        let custom_name = old.custom_name.clone();
        let name_history = old.name_history.clone();
        let (grid, remote_cwd) = {
            let view = old.view.read(cx);
            if view.ssh_destination.as_deref() != Some(destination.as_str()) {
                return;
            }
            (
                (view.grid_cols() as u16, view.grid_rows() as u16),
                (!view.cwd.is_empty()).then(|| view.cwd.clone()),
            )
        };

        let launch = crate::gpui_shell::terminal::view::TerminalLaunch::Ssh {
            destination: destination.clone(),
            cwd: remote_cwd,
        };
        let mut replacement = self.new_pane(grid, launch, None, window, cx);
        replacement.custom_name = custom_name;
        replacement.name_history = name_history;
        self.forget_pane_rename(pane_id);
        let replacement_id = replacement.id;
        let old = {
            let Some(WorkspaceTab::Terminal { panes, tree, focused, .. }) =
                self.tabs.get_mut(tab_ix)
            else {
                replacement.view.read(cx).shutdown();
                return;
            };
            let Some(index) = panes.iter().position(|pane| pane.id == pane_id) else {
                replacement.view.read(cx).shutdown();
                return;
            };
            if !tree.replace_leaf(pane_id, replacement_id) {
                replacement.view.read(cx).shutdown();
                return;
            }
            if *focused == pane_id {
                *focused = replacement_id;
            }
            std::mem::replace(&mut panes[index], replacement)
        };

        self.runtime_hub.record_pane_closed(self.runtime_window_id, pane_id);
        self.pane_bounds.borrow_mut().remove(&pane_id);
        old.view.read(cx).shutdown();
        if tab_ix == self.active {
            self.focus_active(window, cx);
            self.sync_side_panel_to_active(true, cx);
        }
        cx.notify();
    }
}
