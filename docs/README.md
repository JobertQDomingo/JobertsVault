# Jobert's Vault docs

Everything that defines what Jobert's Vault is and how it's built. All the docs listed here are **approved** and are the source of truth: if code and a doc disagree, the doc wins until it's changed on purpose.

## Reading order

| # | Doc | Read it to learn… |
|---|---|---|
| 1 | [PRD.md](PRD.md) | What the app does: goals, numbered requirements (`TXN-1`, `CC-3`, …) and the four phases |
| 2 | [DATA_MODEL.md](DATA_MODEL.md) | The database tables and every calculation rule: balances, periods, comparisons, card cycles |
| 3 | [ARCHITECTURE.md](ARCHITECTURE.md) | How it's built: Tauri, React and Rust layers, commands, startup, testing, security |
| 4 | [DESIGN_SYSTEM.md](DESIGN_SYSTEM.md) | The Glassmorphism 2.0 style: glass layers, colors, type, components, the logo |
| 5 | [UI_FLOWS.md](UI_FLOWS.md) | Every screen and the key flows, such as quick-add and paying the card bill |
| 6 | [decisions/](decisions/README.md) | Why the big choices were made (decision records 0001–0010) |

The progress tracker, [ROADMAP.md](../ROADMAP.md), and the rules for AI coding sessions, [CLAUDE.md](../CLAUDE.md), live at the repository root.

## Visual previews

The design system and UI flows also exist as visual pages, drawn with the real tokens, glass and sample data in ₱:

| Page | Shows |
|---|---|
| [design/design-system.html](design/design-system.html) | Every token, the glass layers, type, icons, the logo and every component, in light and dark mode |
| [design/ui-flows.html](design/ui-flows.html) | Every screen at the real 1440 × 900 window size, the key flows and the Phase 2–4 screens |

- **To view them:** open the `.html` file from your local clone in a browser. GitHub shows `.html` files as code, not as pages.
- **They need internet** to load the Sora font and the icons. Offline, they fall back to the system font and show no icons. (The app itself bundles everything, and only goes online to check for updates.)
- **Keep them in sync:** when a doc changes something visual, update its page too. `vault.css` and `vault.js` are shared by both pages.

## Brand assets

The logo and the app icon source are in [`assets/brand/`](../assets/brand/). See DESIGN_SYSTEM §6.1 for the files and the rules for using them.
