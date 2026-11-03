# 0009. Offline only: no network access at all

| | |
|---|---|
| **Status** | Superseded by [0010](0010-automatic-updates.md) |
| **Date** | 2026-10-26 |
| **Related** | PRD §3 and §8 (Privacy), ARCHITECTURE §10, CLAUDE.md rules 3 and 4 |

## Context

The app holds the owner's complete financial history. They want it private and on their own PC: no bank sync, no cloud sync, no accounts.

## Decision

The app makes **no network connections**:
- **No sync:** no HTTP client, no updater, no analytics, no crash reporting.
- **No remote resources:** no fonts or scripts from the internet. Sora and every library are bundled.
- **Locked down:** the content security policy blocks outgoing connections. The Tauri `http`, `shell` and `fs` plugins are not installed, so the UI can't reach the network or the file system.
- **Local data:** everything lives in `%APPDATA%\com.jobertsvault.app\`.
- **Backups (Phase 2):** written to a folder the owner chooses. A OneDrive or Google Drive folder gives an off-PC copy without the app itself going online.

## Alternatives considered

| Option | Why not |
|---|---|
| **Bank sync (e.g. through an aggregator)** | Needs credentials, a network, a third party and often a fee. Manual entry was chosen in the PRD. |
| **Built-in cloud sync** | A server, accounts and encryption for one user on one PC is a lot of risk for little benefit. |
| **Auto-updater** | Needs network access and code signing. Installing a new version by hand is fine for one user. |

## Consequences

- **Good:** nothing can leak over the network, and the app works the same with or without internet.
- **Cost:** updates are manual (download and run the new installer). Off-PC safety depends on the owner choosing a synced backup folder.
- **Known limitation:** the database file isn't encrypted. It's protected by the Windows account and disk encryption (e.g. BitLocker). Encrypting it (e.g. SQLCipher) would need a new decision record.
- **Rule:** any feature that seems to need the network must first change the PRD and supersede this record.
