use super::editor::{editor_field, editor_input};
use super::*;
use crate::ssh_profiles::{SshConnectionOptions, SshHostJumpMode, SshJumpAuthMode};

impl SettingsPane {
    pub(super) fn ssh_jump_authentication(&self, window: &Window, cx: &Context<Self>) -> gpui::Div {
        let language = crate::gpui_shell::config::ui_language(cx);
        let editor = self.ssh_editor.as_ref().expect("open SSH editor");
        let auth = editor.connection.effective_jump_auth();
        let modes = [
            (SshJumpAuthMode::Inherit, Message::SettingsSshJumpAuthInherit),
            (SshJumpAuthMode::Password, Message::SettingsSshJumpAuthPassword),
            (SshJumpAuthMode::PublicKey, Message::SettingsSshJumpAuthKey),
        ];
        let selected = modes.iter().position(|(mode, _)| *mode == auth).unwrap_or(0);
        v_flex()
            .mt_3()
            .gap_2()
            .child(div().text_xs().font_medium().child(language.text(Message::SettingsSshJumpAuth)))
            .child(
                crate::gpui_shell::widgets::segmented::SegmentedRow::new(
                    "ssh-jump-auth",
                    selected,
                    h_flex().w_full().gap_1().children(modes.into_iter().enumerate().map(
                        |(index, (mode, label))| {
                            Button::new(("ssh-jump-auth", index))
                                .debug_selector(move || format!("ssh-jump-auth-{index}"))
                                .label(language.text(label))
                                .ghost()
                                .small()
                                .flex_1()
                                .selected(auth == mode)
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    if let Some(editor) = this.ssh_editor.as_mut() {
                                        editor.connection.jump_auth = Some(mode);
                                    }
                                    this.touch_ssh_editor(cx);
                                }))
                        },
                    )),
                    div().rounded_md().bg(cx.theme().popover),
                )
                .w_full(),
            )
            .when(auth == SshJumpAuthMode::Password, |body| {
                body.child(editor_field(
                    language.text(Message::SettingsSshJumpPassword),
                    div().debug_selector(|| "ssh-jump-password".into()).child(
                        editor_input(
                            &self.ssh_jump_password_input,
                            language.text(Message::SettingsSshJumpPassword),
                            window,
                            cx,
                        )
                        .mask_toggle()
                        .disabled(editor.clear_jump_password),
                    ),
                ))
                .when(editor.original_destination.is_some(), |body| {
                    body.child(
                        Checkbox::new("ssh-jump-clear-password")
                            .label(language.text(Message::SettingsSshJumpClearPassword))
                            .checked(editor.clear_jump_password)
                            .on_click(cx.listener(|this, checked, _, cx| {
                                if let Some(editor) = this.ssh_editor.as_mut() {
                                    editor.clear_jump_password = *checked;
                                }
                                this.touch_ssh_editor(cx);
                            })),
                    )
                })
            })
            .when(auth == SshJumpAuthMode::PublicKey, |body| body.child(self.ssh_jump_keys(cx)))
    }

    pub(super) fn ssh_jump_password_from_draft(
        &self,
        cx: &gpui::App,
    ) -> Option<zeroize::Zeroizing<String>> {
        let editor = self.ssh_editor.as_ref()?;
        if editor.connection.jump_mode != SshHostJumpMode::Host
            || editor.connection.effective_jump_auth() != SshJumpAuthMode::Password
        {
            return None;
        }
        if editor.clear_jump_password {
            return Some(zeroize::Zeroizing::new(String::new()));
        }
        let password = self.ssh_jump_password_input.read(cx).value();
        (!password.is_empty()).then(|| zeroize::Zeroizing::new(password.to_string()))
    }

    pub(super) fn ssh_jump_test_password(
        &self,
        connection: &SshConnectionOptions,
        cx: &gpui::App,
    ) -> std::io::Result<Option<zeroize::Zeroizing<String>>> {
        let draft = self.ssh_jump_password_from_draft(cx);
        if draft.is_some() || connection.jump_credential_target("").is_none() {
            return Ok(draft);
        }
        let Some(editor) = self.ssh_editor.as_ref() else { return Ok(None) };
        let Some(original) = editor.original_destination.as_deref() else { return Ok(None) };
        // Tests resolve the edited endpoint, not a copied profile's identity. An empty
        // override prevents falling back to another profile's route credential.
        if editor.original_connection.jump_host.trim() != connection.jump_host.trim() {
            return Ok(Some(zeroize::Zeroizing::new(String::new())));
        }
        let Some(target) = editor.original_connection.jump_credential_target(original) else {
            return Ok(Some(zeroize::Zeroizing::new(String::new())));
        };
        crate::ssh_credentials::load_generic_secret(&target)?
            .map(|secret| {
                let secret = zeroize::Zeroizing::new(secret);
                std::str::from_utf8(&secret)
                    .map(|value| zeroize::Zeroizing::new(value.to_owned()))
                    .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
            })
            .transpose()
            .map(|secret| Some(secret.unwrap_or_default()))
    }
}
