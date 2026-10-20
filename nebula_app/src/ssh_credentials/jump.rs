//! Route-local jump passwords use the system credential store, never profile JSON.

use std::io;

use crate::ssh_profiles::SshConnectionOptions;
use zeroize::Zeroizing;

pub(crate) fn save(
    owner: &str,
    connection: &SshConnectionOptions,
    previous: Option<(&str, &SshConnectionOptions)>,
    draft: Option<Zeroizing<String>>,
) -> io::Result<()> {
    save_with(
        owner,
        connection,
        previous,
        draft,
        super::load_generic_secret,
        super::store_generic_secret,
        super::delete_generic_secret,
    )
}

fn save_with(
    owner: &str,
    connection: &SshConnectionOptions,
    previous: Option<(&str, &SshConnectionOptions)>,
    draft: Option<Zeroizing<String>>,
    mut load: impl FnMut(&str) -> io::Result<Option<Vec<u8>>>,
    mut store: impl FnMut(&str, &[u8]) -> io::Result<()>,
    mut delete: impl FnMut(&str) -> io::Result<()>,
) -> io::Result<()> {
    let target = connection.jump_credential_target(owner);
    let previous_target =
        previous.and_then(|(owner, options)| options.jump_credential_target(owner));
    if let Some(target) = &target {
        if let Some(password) = draft {
            if password.is_empty() {
                delete(target)?;
            } else {
                store(target, password.as_bytes())?;
            }
        } else if let Some((_, old)) = previous {
            if old.jump_host.trim() == connection.jump_host.trim()
                && previous_target.as_ref() != Some(target)
            {
                if let Some(old_target) = &previous_target {
                    if let Some(secret) = load(old_target)? {
                        store(target, &Zeroizing::new(secret))?;
                    }
                }
            }
        }
    }
    if previous_target != target {
        if let Some(old_target) = previous_target {
            delete(&old_target)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh_profiles::{SshHostJumpMode, SshJumpAuthMode};
    use std::cell::RefCell;
    use std::collections::HashMap;

    fn connection() -> SshConnectionOptions {
        SshConnectionOptions {
            jump_mode: SshHostJumpMode::Host,
            jump_host: "jump@bastion:2222".into(),
            jump_auth: Some(SshJumpAuthMode::Password),
            ..Default::default()
        }
    }

    #[test]
    fn rename_copies_only_the_same_jump_and_deletes_after_store() {
        let options = connection();
        let old = options.jump_credential_target("old").unwrap();
        let new = options.jump_credential_target("new").unwrap();
        let secrets = RefCell::new(HashMap::from([(old.clone(), b"fixture-secret".to_vec())]));
        save_with(
            "new",
            &options,
            Some(("old", &options)),
            None,
            |key| Ok(secrets.borrow().get(key).cloned()),
            |key, value| {
                secrets.borrow_mut().insert(key.to_owned(), value.to_vec());
                Ok(())
            },
            |key| {
                secrets.borrow_mut().remove(key);
                Ok(())
            },
        )
        .unwrap();
        assert!(!secrets.borrow().contains_key(&old));
        assert_eq!(secrets.borrow()[&new], b"fixture-secret");
        save_with(
            "new",
            &options,
            Some(("new", &options)),
            Some(Zeroizing::new(String::new())),
            |_| panic!("clear must not reload"),
            |_, _| panic!("clear must not store"),
            |key| {
                secrets.borrow_mut().remove(key);
                Ok(())
            },
        )
        .unwrap();
        assert!(secrets.borrow().is_empty());
    }

    #[test]
    fn changed_jump_never_gets_the_previous_password() {
        let old = connection();
        let mut changed = old.clone();
        changed.jump_host = "other@host".into();
        let mut deleted = Vec::new();
        save_with(
            "target",
            &changed,
            Some(("target", &old)),
            None,
            |_| panic!("must not load another jump's secret"),
            |_, _| panic!("must not copy another jump's secret"),
            |key| {
                deleted.push(key.to_owned());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(deleted, vec![old.jump_credential_target("target").unwrap()]);
    }

    #[test]
    fn failed_store_preserves_the_old_credential() {
        let options = connection();
        assert!(
            save_with(
                "new",
                &options,
                Some(("old", &options)),
                None,
                |_| Ok(Some(b"fixture-secret".to_vec())),
                |_, _| Err(io::Error::other("store unavailable")),
                |_| panic!("must not delete before store succeeds"),
            )
            .is_err()
        );
    }
}
