# 0001. Tauri 2 with React, rather than Electron

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-26 |
| **Related** | PRD §1, ARCHITECTURE §1–2, [0003](0003-logic-in-rust.md) |

## Context

Jobert's Vault is a Windows desktop app for one person, used every day and kept for years. It needs a modern UI with charts, forms and glass effects, a local SQLite database, and no network access. The owner asked for a Rust backend.

## Decision

Build it with **Tauri 2**:
- **Frontend:** React 19, TypeScript and Vite, running in Windows' built-in WebView2.
- **Backend:** Rust, which owns all logic and the database.

## Alternatives considered

| Option | Why not |
|---|---|
| **Electron + React** | Every app ships its own copy of Chromium and Node: an installer of about 100 MB or more, and much higher memory use. Its backend would be JavaScript, not Rust. |
| **Tauri + Svelte** | Lighter and pleasant to write, but React has more mature libraries for what this app needs (Recharts, shadcn/ui, react-hook-form, TanStack Query), and AI coding tools handle React best. |
| **Python + Qt (PySide)** | Native-looking, but the modern glass style is much harder to build, and packaging Python for Windows is clumsy. |
| **.NET (WPF / Avalonia)** | A good Windows option, but it has a smaller ecosystem for charts and modern styling, and it isn't the Rust backend the owner asked for. |

## Consequences

- **Good:** an installer of about 10 MB and low memory use. Rust's safety for the code that handles money. The whole web ecosystem for the UI.
- **Cost:** two languages (Rust and TypeScript) and a boundary between them (IPC). [0003](0003-logic-in-rust.md) and ts-rs-generated types keep that boundary safe.
- **Dependency:** WebView2 must be installed. It ships with Windows 10 (version 1803 and later) and Windows 11, and the installer adds it if it's missing.
- **Learning curve:** Rust compile times are slow the first time, a few minutes for the first `tauri dev`.
