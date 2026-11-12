# Jobert's Vault

A private desktop app for Windows that tracks personal cashflow (income and expenses) by **day, week, month or year**. You enter every transaction yourself, and all data stays in a local SQLite file on your computer. Your financial data is never sent over the internet. The only network use is checking GitHub for app updates, which you can turn off, and the app works fully offline.

> **Status:** in development. `v0.3.0` has everything from Phases 1–3 below. Phase 4 (Polish, ending in `v1.0.0`) is next. See [ROADMAP.md](ROADMAP.md) for progress.

## Features

| Phase | Features |
|---|---|
| **1: MVP** | Accounts (cash, bank, e-wallet, credit card), income, expense and transfer transactions, editable categories, a dashboard with income, expense and net cashflow for each day, week (starting Monday), month, year or custom range, account balances and net balance, credit card amount owed, available credit and due date, quick-add with **Ctrl+N**, light and dark themes that follow Windows, automatic updates, a database backup before migrations |
| **2: Reports & backups** | Income-vs-expense charts, category breakdown, period comparison, payee suggestions, automatic daily backups, restore, CSV export |
| **3: Planning** | Recurring transactions, budgets, savings goals, credit card due-date reminders, future-dated transactions |
| **4: Polish** | Theme and Reduce transparency switches, more keyboard shortcuts, polished onboarding on first launch, installer polish |

See [PRD.md](docs/PRD.md) for the full requirements.

## Tech stack

**Tauri 2**, with a **React 19 + TypeScript** frontend (built with Vite, styled with Tailwind CSS and shadcn/ui), a **Rust** backend and a **SQLite** database through rusqlite. More detail in [ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Development setup (Windows 10/11)

### 1. Prerequisites

| Tool | How to install |
|---|---|
| **Microsoft C++ Build Tools** | Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) and tick **"Desktop development with C++"** |
| **WebView2** | Already installed on Windows 10 (version 1803 and later) and Windows 11 |
| **Rust** | Install [rustup](https://rustup.rs) and choose the default **MSVC** toolchain. Check it worked with `rustc --version`. |
| **Node.js** | The current **LTS** version from [nodejs.org](https://nodejs.org). Check it worked with `node --version`. |

The official checklist is at [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/).

### 2. Install and run

Keep the project at a short path with **no apostrophe**, such as `C:\Projects\JobertsVault`. An apostrophe in the path makes the Rust build fail.

```bash
git clone <your-repo-url> jobertsvault
cd jobertsvault
npm install
npm run tauri dev
```

The first `tauri dev` compiles every Rust dependency, which takes several minutes. Later runs take seconds.

### 3. Useful scripts

| Command | What it does |
|---|---|
| `npm run tauri dev` | Runs the app with hot reload |
| `npm run tauri build` | Builds the Windows installer (`src-tauri/target/release/bundle/nsis/*.exe`) |
| `npm test` | Runs the frontend tests (Vitest) |
| `npm run lint` / `npm run typecheck` | Runs ESLint and Prettier, and the TypeScript checks |
| `npm run format` | Fixes formatting with Prettier |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Runs the backend tests and regenerates the TypeScript types in `src/bindings/` |
| `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` | Runs the Rust linter |

## Where your data lives

| What | Location |
|---|---|
| Database | `%APPDATA%\com.jobertsvault.app\jobertsvault.db` |
| Error log | `%APPDATA%\com.jobertsvault.app\jobertsvault.log` (no amounts, payees or notes) |
| Pre-migration backups | `%APPDATA%\com.jobertsvault.app\backups\pre-migration\`, made automatically before an update changes the database |
| Backups | `%APPDATA%\com.jobertsvault.app\backups\` by default, with the daily automatic ones (the newest 30) in `auto\`. You can choose another folder in **Settings → Backup**, where **Back up now**, **Restore from backup…** and the CSV export (**Export…**) also live. |

> Since `v0.2.0`, the app backs itself up the first time it opens each day (unless you turn that off in Settings → Backup), so you no longer need to copy the database by hand. ⚠️ The default backup folder is on the same disk as the database, so a failed disk, or uninstalling with the app data deleted, loses both. Choose a backup folder in OneDrive, Google Drive or on another drive, or copy a backup somewhere else now and then.

`npm run tauri dev` uses `jobertsvault-dev.db`, `jobertsvault-dev.log` and `backups-dev\` (with its own `auto\` and `pre-migration\`) in the same folder, so development never touches your real data.

Updates keep your data, because it isn't in the install folder. Uninstalling keeps it too, unless you tick **Delete the application data** in the uninstaller, which removes the folder above, including the database and any backups in it. To start fresh, delete that folder.

## Project documents

Start with [docs/README.md](docs/README.md) for the reading order.

| Doc | Purpose |
|---|---|
| [PRD.md](docs/PRD.md) | What the app does, with numbered requirements and phases |
| [DATA_MODEL.md](docs/DATA_MODEL.md) | Database schema and calculation rules |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | How the app is built: layers, commands, startup, testing, security |
| [CLAUDE.md](CLAUDE.md) | Working rules for AI coding assistants |
| [DESIGN_SYSTEM.md](docs/DESIGN_SYSTEM.md) | The Glassmorphism 2.0 style: glass layers, colors, fonts, components |
| [UI_FLOWS.md](docs/UI_FLOWS.md) | Screens and user flows |
| [ROADMAP.md](ROADMAP.md) | Phase checklists and progress |
| [RELEASING.md](RELEASING.md) | How a change gets from a branch to every installed copy |
| [docs/decisions/](docs/decisions/README.md) | Decision records 0001–0010: why the big choices were made |
| [docs/design/](docs/design/) | Visual previews of the design system and every screen. Open the `.html` files in a browser. |
| `assets/brand/` | The logo (`logo.svg`), one-color versions and the 1024 px app icon source |

## License

A personal project, not licensed for distribution.
