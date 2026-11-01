# Jobert's Vault

A private, offline desktop app for Windows that tracks personal cashflow (income and expenses) by **day, week, month or year**. You enter every transaction yourself, and all data stays in a local SQLite file on your computer. Nothing is sent over the internet.

> **Status:** planning complete, development not started yet. The documents below are approved. Code begins with Phase 0 (project setup).

## Features

| Phase | Features |
|---|---|
| **1: MVP** | Accounts (cash, bank, e-wallet, credit card), income, expense and transfer transactions, editable categories, a dashboard with income, expense and net cashflow for each day, week (starting Monday), month or year, account balances and net balance, credit card amount owed, available credit and due date, quick-add with **Ctrl+N**, light and dark themes that follow Windows |
| **2: Reports & backups** | Income-vs-expense charts, category breakdown, period comparison, payee suggestions, automatic daily backups, restore, CSV export |
| **3: Planning** | Recurring transactions, budgets, savings goals, credit card due-date reminders |
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
| `npm run lint` / `npm run typecheck` | Runs ESLint and the TypeScript checks |
| `cargo test --manifest-path src-tauri/Cargo.toml` | Runs the backend tests and regenerates the TypeScript types in `src/bindings/` |
| `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` | Runs the Rust linter |

## Where your data lives

| What | Location |
|---|---|
| Database | `%APPDATA%\com.jobertsvault.app\jobertsvault.db` |
| Error log | `%APPDATA%\com.jobertsvault.app\jobertsvault.log` (no amounts, payees or notes) |
| Backups (Phase 2) | `%APPDATA%\com.jobertsvault.app\backups\` by default (you can change this in Settings) |

> ⚠️ **Phase 1 has no backup feature.** Until Phase 2, close the app now and then and copy `jobertsvault.db` somewhere safe yourself. A OneDrive or Google Drive folder works well.

`npm run tauri dev` uses `jobertsvault-dev.db` and `jobertsvault-dev.log` in the same folder, so development never touches your real data.

Uninstalling the app **does not** delete your data. To start fresh, delete the folder above.

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
| [docs/decisions/](docs/decisions/README.md) | Decision records 0001–0009: why the big choices were made |
| [docs/design/](docs/design/) | Visual previews of the design system and every screen. Open the `.html` files in a browser. |
| `assets/brand/` | The logo (`logo.svg`), one-color versions and the 1024 px app icon source |

## License

A personal project, not licensed for distribution.
