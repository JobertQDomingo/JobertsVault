# 0008. A fixed 1440 × 900 window that can't be resized

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-26 |
| **Related** | PRD §8 (Window), DESIGN_SYSTEM §5, UI_FLOWS §1, ARCHITECTURE §11 |

## Context

The owner wants a window that can't be resized. They use two monitors: a 29" ultrawide at 2560 × 1080 and a 24" at 1920 × 1080, both at 100% display scaling. With the Windows taskbar, each has about **1032 px** of usable height.

## Decision

- **Size:** the window's content area is a fixed **1440 × 900**.
- **Settings:** `resizable: false` and `maximizable: false` (minimize and close only), and `center: true`.
- **Fallback:** at startup Rust checks the monitor's usable area. If 1440 × 900 plus the title bar doesn't fit, for example after display scaling is raised above 100%, the window opens at **1280 × 800** instead.

**Why this size:**
- The 900 px height plus a ~32 px title bar leaves about 100 px spare on both monitors.
- Its 16:10 shape gives lists more rows than 16:9.
- 1440 px is wide enough for the 228 px sidebar plus a 1176 px content column.

## Alternatives considered

| Option | Why not |
|---|---|
| **Resizable with a minimum size and responsive layouts** | The owner asked for a fixed size. It would also need extra layouts and testing at many widths. |
| **Remember the size and position (`tauri-plugin-window-state`)** | Not needed once the size is fixed, and it avoids an extra plugin. |
| **1440 × 810 (16:9)** | Shows fewer list rows, with no benefit on these monitors. |
| **1600 × 900 or larger** | Fills almost the whole 1920 px monitor, leaving no room to see anything beside it. |

## Consequences

- **Good:** every screen is designed and tested at exactly one size, with no responsive breakpoints for the window, and mockups match the real app.
- **Behavior:** the sidebar and top bar stay in place. Content below the top bar scrolls inside the window.
- **Cost:** the window can't be maximized to use the ultrawide's extra width.
- **Revisit if:** the monitors change, or Windows display scaling is raised above 100%. The fallback size covers the scaling case, but a new decision record should pick a better size then.
