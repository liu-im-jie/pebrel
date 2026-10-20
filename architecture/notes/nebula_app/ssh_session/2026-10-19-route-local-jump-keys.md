# Route-local jump-host authentication

## Status

Implemented; native compilation and runtime validation pending.

## Context

The SSH host editor accepts a jump address, but a manually entered jump host
could only obtain authentication from a separately saved profile or SSH config.
The editor needs password and private-key authentication independent of the target host.

## Evidence

- `ssh_profiles/connection.rs` persists the target's connection options.
- `ssh_session/route.rs` resolves a separate profile for each hop and includes
  each hop's effective authentication in the connection pool identity.
- The connection test and live session both consume this resolved route.

## Decision

Store the optional authentication choice in `connection.jump_auth`, with key
paths in `connection.jump_private_keys`. Explicit choices are inherit, password
and public key. When the choice is absent, a nonempty key list retains the
previous public-key override behavior; otherwise authentication is inherited.
Overrides apply only in explicit `Host` jump mode and only to the immediate hop.
The target, saved jump profile and other hops retain their settings.

Passwords live in the existing system credential store under a route-owner and
jump-address identity, separate from both the target and the saved jump host.
The resolved hop owns this credential identity for loading, prompting, saving and
rejected-password cleanup. Its connection-pool identity includes the credential
scope, never the secret. An unsaved test password belongs only to that hop and is
zeroized when released. An explicitly cleared draft suppresses saved credentials.

Blank editor input preserves the saved password. Renaming a route copies the
secret only when the jump address is unchanged and deletes the old key after
the new write succeeds. Changing the jump or disabling password authentication
cleans up the previous route-specific credential. Host duplication copies it
to an independent identity; committed deletion removes it.

Reuse the existing asynchronous private-key file picker, path de-duplication,
editor-generation check and test-result invalidation. Persist paths, not key
contents. Backups strip these paths, matching target-key backup behavior.

## Rejected alternatives

- Mutating the saved jump profile would change unrelated target routes.
- Requiring a saved jump profile would leave manual jump addresses unsupported.

## Consequences

The JSON change is additive. Explicit inherit ignores retained key paths;
password mode does not try those keys. Profile JSON and backups contain no
password. Changing effective jump authentication changes the route pool identity
without altering the target's credentials. CSV exchange remains metadata-only.

## Validation

Added regression cases for persistence compatibility, route-local ownership,
inactive modes, connection-pool identity, backup exclusion, editor controls,
credential rename/copy/clear/failure and isolated loopback SSH authentication
with both keys and passwords. These cases have not been executed. Native
compilation, UI interaction, system credential-store integration and real SSH
connections remain pending authorization; static checks do not establish them.

## Supersedes

None.

## Revisit when

Jump-host authentication gains additional methods or route identity semantics
change; keep credential ownership separate from target authentication.
