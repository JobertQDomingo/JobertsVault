# 0010. Automatic updates: the one exception to offline

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-11-03 |
| **Supersedes** | [0009](0009-offline-only.md) |
| **Related** | PRD §7.13 and §8 (Privacy), ARCHITECTURE §10 and §11, DATA_MODEL §8, CLAUDE.md rules 3 and 4 |

## Context

The owner wants new versions to install themselves from the MVP onward, using the same release process as their other Tauri app, NBSBuddy. [0009](0009-offline-only.md) ruled this out: no network at all, and updates by running a new installer by hand.

Two things differ from NBSBuddy. This app holds a complete financial history, and an update can carry a database migration. NBSBuddy keeps no local database, so it never had to protect one during an update.

## Decision

**The app makes one kind of network connection: the updater.** Everything else in 0009 still stands.

- **What connects:** only `tauri-plugin-updater`, running in Rust. It fetches `latest.json` and the installer from this repository's GitHub releases, and nothing else.
- **What's sent:** the requests for those files. GitHub sees the computer's IP address and the time, as with any download. **No financial data, and nothing from the database, ever leaves the computer.**
- **The UI stays locked down:** the content security policy doesn't change, and the frontend gets no updater permissions. It calls our own `check_update` and `install_update` commands.
- **Still not allowed:** other HTTP clients, sync, analytics, crash reporting, remote fonts or scripts, and the Tauri `http`, `shell` and `fs` plugins.
- **Automatic by default, with a real off switch:** when automatic updates are on, the app checks at each start and installs a newer version. When they're off, the app never connects by itself. It connects only when the user clicks **Check for updates**.
- **Offline still works:** a failed check never stops or interrupts the app.
- **Signed:** every update is signed with a private key held only by the owner and GitHub Actions. Installed copies refuse an update whose signature doesn't match, or one signed for a different version than announced (`requireSignedVersion`).
- **Data comes first:** the database is backed up before any migration runs (BAK-5, moved from Phase 2 into the MVP). If that backup fails, the migration doesn't run.
- **Forward only:** a bad release is fixed by a newer one. Published releases are never deleted or moved.
- **Development builds never update**, so `npm run tauri dev` can't replace an installed copy.

## Alternatives considered

| Option | Why not |
|---|---|
| **Stay offline, install new versions by hand** ([0009](0009-offline-only.md)) | The owner asked for automatic updates. Manual updates also mean old versions linger, with their bugs. |
| **Notify only, and the user downloads the installer** | Still needs the network, but keeps the manual step that this decision exists to remove. |
| **Check in the background while the app is open** | The app could restart while a transaction is being typed. Updating at startup, or when the user clicks, avoids that. |
| **Keep NBSBuddy's "off" behavior**, where the app still checks and shows an Update button | For a private finance app, "off" should mean no automatic connection at all. |
| **Keep pre-migration backups in Phase 2** | Updates would then reach real data with no safety copy from the first release onward. |

## Consequences

- **Good:** fixes reach the installed app with no manual step, and the release process is the same as NBSBuddy's.
- **Cost:** the app now includes an HTTP client (`reqwest`, through the plugin), and it can't honestly be called fully offline. It's "local data, with updates from GitHub".
- **Cost:** the private signing key must be protected and backed up. If it's lost, no installed copy can accept a new update, so the app must be reinstalled by hand once. If it leaks, together with access to this repository, someone could ship a malicious update to an app holding financial data.
- **Cost:** the repository must stay public, because the updater downloads releases without logging in.
- **Cost:** startup can wait up to 3 seconds on a slow connection before the check gives up.
- **Accepted:** the owner accepted the signing-key and public-repository costs on 2026-11-03, as they did for NBSBuddy.
- **Limit:** a pre-migration backup is restored by hand until Phase 2 adds Restore.
- **Limit:** the installer stays unsigned for Windows. The update signature protects updates, but SmartScreen still warns on a first install.
- **Rule:** any other feature that seems to need the network must first change the PRD and supersede this record.
- **Revisit if:** the repository needs to go private, or the signing key is ever exposed.
