use super::*;

fn add_host(lists: &mut SshHostLists, destination: &str, label: &str, group: &str) {
    let mut profile = lists.profiles.for_destination(destination);
    profile.label = Some(label.to_owned());
    lists.profiles.upsert(profile);
    lists
        .profiles
        .set_organization(
            destination,
            crate::ssh_profiles::HostOrganization::from_inputs(group, "", "").unwrap(),
        )
        .unwrap();
}

fn fixture() -> SshHostLists {
    let mut lists = SshHostLists::default();
    add_host(&mut lists, "root@prod", "Production", "Servers");
    add_host(&mut lists, "root@staging", "Staging", "Servers");
    add_host(&mut lists, "root@hidden", "Hidden", "Hidden group");
    lists.saved = vec!["root@prod".into(), "recent-only".into()];
    lists.configured = vec!["root@prod".into(), "config-only".into()];
    lists.pinned = vec!["root@staging".into()];
    lists.hidden = vec!["root@hidden".into()];
    lists
}

#[test]
fn grouping_keeps_source_visibility_labels_and_pinned_order() {
    let groups = grouped_hosts(&fixture());
    assert_eq!(groups.iter().map(|group| group.name.as_str()).collect::<Vec<_>>(), ["Servers", ""]);
    let servers = &groups[0].hosts;
    assert_eq!(servers.len(), 2, "a host from multiple sources appears once");
    assert_eq!((&*servers[0].destination, &*servers[0].label), ("root@staging", "Staging"));
    assert_eq!((&*servers[1].destination, &*servers[1].label), ("root@prod", "Production"));
    assert_eq!(
        groups[1].hosts.iter().map(|host| host.label.as_str()).collect::<Vec<_>>(),
        ["recent-only", "config-only"]
    );
}

#[test]
fn all_managed_hosts_are_listed_beyond_the_recent_host_cap() {
    let mut lists = SshHostLists::default();
    for index in 0..200 {
        add_host(&mut lists, &format!("host-{index}"), "", "Servers");
        lists.remember(&format!("host-{index}"));
    }
    assert_eq!(lists.saved.len(), crate::gpui_shell::ssh_hosts::SAVED_HOSTS_CAP);
    let groups = grouped_hosts(&lists);
    assert_eq!(groups[0].hosts.len(), 200);
    assert!(groups[0].hosts.iter().any(|host| host.destination == "host-0"));
}

#[test]
fn copied_hosts_keep_their_own_launch_identity() {
    let mut lists = SshHostLists::default();
    add_host(&mut lists, "saved-copy", "", "Servers");
    lists.profiles.set_connection_destination("saved-copy", "root@prod").unwrap();
    let groups = grouped_hosts(&lists);
    assert_eq!(groups[0].hosts[0].label, "root@prod");
    assert_eq!(groups[0].hosts[0].destination, "saved-copy");
}

#[test]
fn section_starts_collapsed_and_group_state_survives_snapshot_refresh() {
    let mut state = SidebarState::default();
    assert!(!state.expanded());
    state.replace_hosts(grouped_hosts(&fixture()));
    assert!(!state.expanded());
    assert_eq!(state.count, Some(4));
    state.toggle_group("Servers");
    state.replace_hosts(grouped_hosts(&fixture()));
    assert!(state.collapsed_groups.contains("Servers"));
    state.toggle_group("Servers");
    assert!(!state.collapsed_groups.contains("Servers"));
    state.replace_hosts(Vec::new());
    assert!(state.groups.is_empty());
    assert_eq!(state.count, Some(0));
}

#[cfg(feature = "gpui-test-support")]
mod ui {
    use super::*;
    use gpui::{Modifiers, TestAppContext, VisualTestContext};

    fn open(cx: &mut TestAppContext) -> (Entity<NebulaWorkspace>, VisualTestContext) {
        let hub = crate::runtime_api::RuntimeHub::new();
        cx.update(|cx| {
            gpui_component::init(cx);
            cx.set_reduce_motion(true);
            cx.set_global(crate::gpui_shell::config::Settings::load(
                nebula_settings::ThemeName::Nord,
            ));
            crate::gpui_shell::math_view::register(cx);
            crate::gpui_shell::file_editor::init(cx);
            super::super::super::init(cx);
            windowing::initialize(cx, hub.clone());
        });
        let mut workspace_out = None;
        let (_, visual) = cx.add_window_view(|window, cx| {
            let workspace = cx.new(|cx| {
                NebulaWorkspace::new(
                    window,
                    None,
                    None,
                    1,
                    hub,
                    windowing::WorkspaceStartup::Empty,
                    windowing::WindowRole::Regular,
                    cx,
                )
            });
            workspace.update(cx, |view, cx| {
                view.tabs_position = nebula_settings::TabsPositionName::Sidebar;
                view.sidebar_width = 230.0;
                view.ssh_sidebar.load = None;
                view.ssh_sidebar.replace_hosts(grouped_hosts(&fixture()));
                cx.notify();
            });
            workspace_out = Some(workspace.clone());
            Root::new(workspace, window, cx)
        });
        visual.simulate_resize(size(px(1000.0), px(700.0)));
        (workspace_out.unwrap(), visual.clone())
    }

    fn draw(cx: &mut VisualTestContext) {
        cx.update(|window, cx| {
            window.refresh();
            window.draw(cx).clear(cx);
        });
    }

    fn assert_ssh_layout(cx: &mut VisualTestContext, expanded: bool) {
        let sidebar = cx.debug_bounds("workspace-sidebar").unwrap();
        let section = cx.debug_bounds("sidebar-ssh-section").unwrap();
        let header = cx.debug_bounds("sidebar-ssh-toggle").unwrap();
        let padding = cx.update(|window, _| window.rem_size() * 0.5);
        assert!((f32::from(section.bottom() - (sidebar.bottom() - padding))).abs() < 1.0);
        assert_eq!(header.top(), section.top());
        let height =
            if expanded { (sidebar.size.height - padding) / 3.0 } else { header.size.height };
        assert!((f32::from(section.size.height - height)).abs() < 1.0);
        if expanded {
            let list = cx.debug_bounds("sidebar-ssh-list-slot").unwrap();
            assert!((f32::from(list.bottom() - section.bottom())).abs() < 1.0);
            assert!(list.top() >= header.bottom());
        }
    }

    #[gpui::test]
    fn full_width_header_and_groups_toggle_without_opening_a_connection(cx: &mut TestAppContext) {
        let (workspace, mut cx) = open(cx);
        draw(&mut cx);
        let tabs = cx.debug_bounds("sidebar-tabs-toggle").unwrap();
        let ssh = cx.debug_bounds("sidebar-ssh-toggle").unwrap();
        assert_eq!(tabs.size, ssh.size);
        assert_ssh_layout(&mut cx, false);
        assert!(cx.debug_bounds("sidebar-ssh-group-0").is_none());
        // Exercise the header's empty right side, not just the text or chevron.
        cx.simulate_click(gpui::point(ssh.right() - px(4.0), ssh.center().y), Modifiers::default());
        workspace.update(&mut cx, |view, _| view.ssh_sidebar.load = None);
        draw(&mut cx);
        assert!(workspace.read_with(&cx, |view, _| view.ssh_sidebar.expanded()));
        assert_ssh_layout(&mut cx, true);
        let group = cx.debug_bounds("sidebar-ssh-group-0").unwrap();
        let host = cx.debug_bounds("sidebar-ssh-host-0-0").unwrap();
        assert!(group.size.height >= px(32.0));
        assert!(host.size.height >= px(32.0));
        cx.simulate_click(group.center(), Modifiers::default());
        draw(&mut cx);
        assert!(cx.debug_bounds("sidebar-ssh-host-0-0").is_none());
        assert!(cx.debug_bounds("sidebar-ssh-group-1").is_some());
        cx.simulate_keystrokes("right");
        draw(&mut cx);
        assert!(cx.debug_bounds("sidebar-ssh-host-0-0").is_some());
        let header = cx.debug_bounds("sidebar-ssh-toggle").unwrap();
        cx.simulate_click(header.center(), Modifiers::default());
        draw(&mut cx);
        assert!(cx.debug_bounds("sidebar-ssh-group-0").is_none());
        assert_ssh_layout(&mut cx, false);
        assert!(workspace.read_with(&cx, |view, _| view.tabs.is_empty()));
    }

    #[gpui::test]
    fn bottom_section_keeps_one_third_height_and_scrolls_after_resize(cx: &mut TestAppContext) {
        let (workspace, mut cx) = open(cx);
        let mut lists = SshHostLists::default();
        for index in 0..80 {
            add_host(&mut lists, &format!("host-{index}"), "", "Servers");
        }
        workspace.update(&mut cx, |view, cx| {
            view.ssh_sidebar.expanded = true;
            view.ssh_sidebar.replace_hosts(grouped_hosts(&lists));
            cx.notify();
        });
        for (height, tabs_collapsed) in [(700.0, false), (400.0, true), (900.0, false)] {
            workspace.update(&mut cx, |view, cx| {
                view.tabs_section_collapsed = tabs_collapsed;
                view.ssh_sidebar.scroll.set_offset(gpui::point(px(0.0), px(0.0)));
                cx.notify();
            });
            cx.simulate_resize(size(px(1000.0), px(height)));
            draw(&mut cx);
            assert_ssh_layout(&mut cx, true);
            let list = cx.debug_bounds("sidebar-ssh-list-slot").unwrap();
            cx.simulate_event(gpui::ScrollWheelEvent {
                position: list.center(),
                delta: gpui::ScrollDelta::Pixels(gpui::point(px(0.0), px(-180.0))),
                touch_phase: gpui::TouchPhase::Moved,
                modifiers: Modifiers::default(),
            });
            draw(&mut cx);
            assert!(
                workspace.read_with(&cx, |view, _| view.ssh_sidebar.scroll.offset().y < px(0.0))
            );
            assert_ssh_layout(&mut cx, true);
        }
    }
}
