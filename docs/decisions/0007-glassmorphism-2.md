# 0007. Visual style: Glassmorphism 2.0, with the aurora painted by the app

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-26 |
| **Related** | DESIGN_SYSTEM.md (all), UI_FLOWS.md, ARCHITECTURE §2, §11 and §12 |

## Context

The owner wanted a modern, distinctive look for an app they open every day. Glass effects cost some graphics power, but the app runs on the owner's own PC, which has strong specs.

## Decision

Use **Glassmorphism 2.0**:
- **Panels:** frosted glass panels at three levels: the canvas, panels and raised elements.
- **Background:** a soft colored "aurora" (pink, blue, mint and peach lights with film grain) behind the panels.
- **Font:** **Sora**, bundled with `@fontsource/sora`.
- **Accent:** deep indigo.
- **Logo:** "Line Door" ([DESIGN_SYSTEM §6.1](../DESIGN_SYSTEM.md)).

**The app paints the aurora itself.** The window is opaque: no Mica, no Acrylic, and no see-through to the desktop.

## Alternatives considered

How the glass is produced:

| Option | Why not chosen |
|---|---|
| **Windows Mica / Acrylic window effects** | They show the desktop wallpaper through the app, so contrast would change with the wallpaper. The painted aurora is identical on every PC. |
| **A transparent Tauri window** | Same problem as Mica, plus more expensive redrawing and odd behavior when the window moves between monitors |

## Consequences

- **Rules to keep:**
  - Text always sits on a panel, never on the bare aurora.
  - Never blur inside a blur.
  - Keep about 12 or fewer blurred elements on screen.
  - Never blur list rows.
- **Contrast:** every text color is measured at the *worst* spot, where a panel sits over the strongest aurora color. All pass WCAG AA. The lowest is 4.95:1. Re-check whenever a token changes.
- **Fallback:** when Windows transparency is off, or the app's own Reduce transparency switch (SET-8) is on, panels become solid and nothing is blurred.
- **Cost:** shadcn/ui components need restyling, and the Sora font adds a small amount to the installer. Phase 1 includes a check that scrolling stays smooth.
