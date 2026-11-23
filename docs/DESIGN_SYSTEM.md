# Design System: Jobert's Vault

| | |
|---|---|
| **Status** | Approved v1.2 |
| **Style** | **Glassmorphism 2.0**: frosted glass panels floating over a soft colored light (the "aurora") |
| **Last updated** | 2026-11-23 |
| **Related docs** | [PRD.md](PRD.md), [UI_FLOWS.md](UI_FLOWS.md), [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Built with** | Tailwind CSS 4 + shadcn/ui (Radix), lucide-react icons, Sora font (bundled) |

> **Visual version:** [design/design-system.html](design/design-system.html) shows every token and component below, drawn with the real values in light and dark mode. Open it in a browser.

## 1. Principles

1. **Numbers come first.** Amounts are the largest, clearest thing on screen. The glass frames them and never competes with them.
2. **Glass is the material, color is the light.** Panels stay neutral and frosted. Color lives in the aurora behind them and in the meaning colors (income, expense, warnings), never on whole panels.
3. **Readability beats effect.** Text always sits on a frosted panel, a field or a frosted pill, never straight on the aurora. Every text color is checked at the **worst spot**: a panel sitting over the strongest aurora color.
4. **One layer of blur.** A panel is blurred once. Rows, chips, fields and toolbars inside it are flat tints, never blurred again.
5. **Never rely on color alone.** Income and expense always also have a sign (+ / −) or an icon. This is a PRD accessibility rule.
6. **Built for the keyboard.** Every action works without a mouse, with a visible focus ring.

---

## 2. Glass and layers

### 2.1 The three levels

| Level | What | Material |
|---|---|---|
| **0 · Canvas** | The window background | The aurora: four soft color lights over a plain canvas, plus film grain (§2.2) |
| **1 · Panel** | Sidebar, top bar, toolbars (such as the period bar), cards, banners | `--glass` (62%), blur **22 px** + saturate 170%, 1 px `--edge`, `--shadow-panel`, radius 16–20 px |
| **2 · Raised** | Dialogs, menus, popovers, the date picker, toasts | `--glass-strong` (86%), blur **32 px** + saturate 180%, 1 px `--edge`, `--shadow-raised`, radius 16–24 px |

Inside a panel, things are **flat**:

| Element | Uses |
|---|---|
| Rows, hovers, tracks, totals | `--wash`. A track (a progress bar's, a step or level not reached yet, the line between them) also has a 1 px inset `--hairline` edge, or it vanishes on glass |
| Inputs, secondary buttons | `--field` with a `--field-edge` border |
| Dividers | `--hairline` |

### 2.2 The aurora

| | Light | Dark |
|---|---|---|
| Canvas | `#EEF1FA` | `#070815` |
| Top-left light (10% 15%) | pink `#FF8BD0` | violet `#7C3AED` |
| Top-right light (90% 10%) | blue `#7FA8FF` | blue `#1D4ED8` |
| Bottom-right light (78% 96%) | mint `#5EEAD4` | teal `#0E7490` |
| Bottom-left light (18% 92%) | peach `#FDBA74` | magenta `#BE185D` |
| Film grain | 9%, overlay blend | 7%, overlay blend |

- Each light is a radial gradient that fades out by 70% of its radius.
- **The app paints the aurora itself** in CSS. The Tauri window stays opaque and doesn't show the Windows wallpaper through it, so contrast never depends on your desktop. Windows' Mica and Acrylic effects are not used.
- **The aurora is fixed to the window.** It doesn't scroll or animate.

### 2.3 Rules

| Do | Don't |
|---|---|
| Put all text on a panel, a field or a frosted pill | Put text straight on the aurora |
| Blur a panel once; everything inside it is flat | Put blurred panels inside blurred panels |
| Use `--glass-strong` for dialogs, menus and toasts | Give a dialog the same transparency as the cards behind it |
| Keep color in the aurora and in the meaning colors | Tint whole panels with category colors |
| Keep about **12 or fewer** blurred elements on screen | Blur every list row |
| Tint only the net cashflow card with the accent (12%) | Tint every stat card |

### 2.4 When transparency is off

When Windows' **Transparency effects** setting is off, the app detects it through the CSS `prefers-reduced-transparency: reduce` media query:
- Every panel and raised element becomes solid `--glass-solid`.
- Nothing is blurred, the dialog backdrop included.
- The aurora, shapes, colors and layout stay the same.

Settings → Appearance also has a **Reduce transparency** switch (SET-8, Phase 5), stored as `reduce_transparency`, that does the same for this app alone. `FollowReduceTransparency`, in `App` above the router, keeps it as the `.reduce-transparency` class on `<html>`, set before the screens' first paint, so screens outside the shell (`/welcome`, `/check-in`) follow it too. Both come through one Tailwind variant, `transparency-off`, so the switch and the Windows setting can never look different. The "Checking for updates…" notice at startup comes before the settings load, so it follows Windows only.

---

## 3. Color tokens

All colors are **CSS variables** in `src/styles/globals.css`. Components use the token names, never raw values. The names follow shadcn/ui's conventions, plus the glass and app-specific tokens below.

**Contrast is measured at the worst spot:** text on a 62% panel over the strongest aurora color, the lowest ratio over every 4 × 4-pixel block of the window. WCAG AA requires 4.5:1 for text. **Every text color passes on every surface it's used on** (§3.6), and the lowest is dark-mode `--muted-foreground` on a `--wash` row inside a `--wash` card at **4.71:1**. Category colors are used as icons, which need 3:1, and their worst case is 3.7:1 (light) and 6.4:1 (dark). The token figures were re-measured on 2026-11-23 (the T24 accessibility pass), when light `--income`, `--success`, `--expense` and `--danger` were made one step darker so they pass on a `--wash` card and the net cashflow card too. Re-check any token you change.

### 3.1 Glass surfaces

| Token | Purpose | Light | Dark |
|---|---|---|---|
| `--canvas` | Behind the aurora | `#EEF1FA` | `#070815` |
| `--glass` | Level 1 panels | `rgb(255 255 255 / .62)` | `rgb(20 22 38 / .62)` |
| `--glass-strong` | Level 2 raised elements | `rgb(255 255 255 / .86)` | `rgb(20 22 38 / .86)` |
| `--glass-solid` | Panels when transparency is off | `#F7F8FC` | `#151729` |
| `--glass-hover` | Row hover inside a panel | `rgb(255 255 255 / .5)` | `rgb(255 255 255 / .06)` |
| `--field` | Inputs, secondary buttons | `rgb(255 255 255 / .72)` | `rgb(255 255 255 / .06)` |
| `--field-edge` | Input borders | `rgb(11 16 32 / .16)` | `rgb(255 255 255 / .16)` |
| `--edge` | Bright 1 px edge on every glass surface | `rgb(255 255 255 / .8)` | `rgb(255 255 255 / .14)` |
| `--hairline` | Dividers inside panels | `rgb(11 16 32 / .08)` | `rgb(255 255 255 / .08)` |
| `--wash` | Flat tints: tracks, totals, icon squares | `rgb(11 16 32 / .05)` | `rgb(255 255 255 / .07)` |
| `--overlay` | Dims the app behind a dialog | `rgb(11 16 32 / .28)` | `rgb(0 0 0 / .5)` |
| `--shadow-panel` | Level 1 shadow | inner 1 px white highlight + `0 10px 30px rgb(31 38 100 / .12)` | inner highlight 10% + `0 10px 30px rgb(0 0 0 / .35)` |
| `--shadow-raised` | Level 2 shadow | inner highlight + `0 24px 60px rgb(31 38 100 / .24)` | inner highlight + `0 24px 60px rgb(0 0 0 / .55)` |

### 3.2 Text and accent

| Token | Purpose | Light | Dark | Worst-case contrast (L / D) |
|---|---|---|---|---|
| `--foreground` | Main text | `#0B1020` | `#F5F7FF` | 14.0 / 10.7 |
| `--muted-foreground` | Secondary text, labels | `#3D4560` | `#C3C9DE` | 7.0 / 7.0 |
| `--primary` | Accent: main buttons, active nav, links | `#3730A3` (deep indigo) | `#A5B4FC` (periwinkle) | 7.35 / 5.8 |
| `--primary-hover` | Primary hover | `#312E81` | `#C7D2FE` | — |
| `--primary-foreground` | Text on primary | `#FFFFFF` | `#0B1020` | 9.9 / 9.5 |
| `--ring` | Focus ring | `#3730A3` | `#A5B4FC` | — |

### 3.3 Meaning (money and status)

| Token | Purpose | Light | Dark | Worst-case contrast (L / D) |
|---|---|---|---|---|
| `--income` | Income, increases that are good | `#035E43` | `#6EE7B7` | 5.8 / 7.5 |
| `--expense` | Expenses, increases that are bad | `#A21634` | `#FDA4AF` | 5.75 / 6.1 |
| `--transfer` | Transfers (neutral) | `#3D4560` | `#C3C9DE` | 7.0 / 7.0 |
| `--warning` | Budget ≥ 80%, due-date banners (card and loan), stale-backup notice | `#8A4500` | `#FCD34D` | 5.3 / 8.0 |
| `--danger` | Budget ≥ 100%, an overdue loan payment, a pay window that runs short, delete, errors (same red as `--expense`) | `#A21634` | `#FDA4AF` | 5.75 / 6.1 |
| `--danger-foreground` | Text on a danger button | `#FFFFFF` | `#0B1020` | 7.8 / 10.0 |
| `--danger-banner` | The red mixed into a `danger` banner (§9.1). Dark mode uses a deeper red than `--danger`, because the light pink at 14% read as plain glass over the aurora | `#A21634` (`--danger`) | `#DC2626` | 11.2 / 8.4 (text), 16.4 / 7.55 (Pay button), 4.6 / 4.8 (icon) |
| `--danger-banner-mix` | How much of `--danger-banner` a `danger` banner mixes in | `14%` | `30%` | — |
| `--success` | Goal achieved, loan paid off, "You're debt-free", "Saved" toasts | `#035E43` | `#6EE7B7` | 5.8 / 7.5 |

- **Tints:** each meaning color has a tint for chips and banners. It's the color mixed into transparent at **16%** in light mode and **20%** in dark (`--tint`).
- **Banners** are panel glass with the meaning color mixed in at 14%, and a border with it mixed in at 35%. A `danger` banner mixes `--danger-banner` into its glass at `--danger-banner-mix` instead, and into its border at 35%, so in dark mode it reads as red and as more urgent than the amber `warning` banner beside it. Its contrast figures are `--foreground` text, the `secondary` Pay button's text and the `--danger` icon on the banner, at the worst spot and with transparency off.

### 3.4 Category palette (`cat-1` … `cat-12`)

Categories store a palette key, not a hex code (DATA_MODEL §3.2), so they switch automatically between light and dark mode.

| Key | Hue | Light | Dark |
|---|---|---|---|
| `cat-1` | Blue | `#1D4ED8` | `#93C5FD` |
| `cat-2` | Orange | `#C2410C` | `#FDBA74` |
| `cat-3` | Teal | `#0F766E` | `#5EEAD4` |
| `cat-4` | Pink | `#BE185D` | `#F9A8D4` |
| `cat-5` | Violet | `#6D28D9` | `#C4B5FD` |
| `cat-6` | Amber | `#B45309` | `#FCD34D` |
| `cat-7` | Cyan | `#0E7490` | `#67E8F9` |
| `cat-8` | Rose | `#BE123C` | `#FDA4AF` |
| `cat-9` | Lime | `#4D7C0F` | `#BEF264` |
| `cat-10` | Fuchsia | `#A21CAF` | `#F0ABFC` |
| `cat-11` | Stone | `#57534E` | `#D6D3D1` |
| `cat-12` | Emerald | `#047857` | `#6EE7B7` |

- The **color picker** in category settings offers only these 12 swatches.
- The breakdown chart (Phase 2) shows the 8 largest categories plus one **Other** slice in `--muted-foreground`.
- Income-vs-expense charts always use `--income` and `--expense`, never category colors. Green and red are hard to tell apart with red-green color blindness, so **income bars rise above the zero line and expense bars hang below it**: direction carries the meaning as well as color. Net cashflow is a 2 px `--primary` line. Bars are at most 24 px wide with 4 px rounded ends, gridlines are `--hairline`, and the tooltip is flat `--glass-solid` (it sits inside a blurred panel, §2.3).
- **Income and expense colors are numbered separately** (DATA_MODEL §7), because the breakdown chart never shows both together. The 12 most common expense categories each have their own color. Only 5 rarely used ones share (Education, Gifts & Donations, Travel, Fees & Charges, Other expense).
- A category the user adds defaults to the least-used color of its kind.

### 3.5 Implementation (Tailwind 4)

```css
/* src/styles/globals.css */
@import "tailwindcss";
@import "@fontsource/sora/400.css";
@import "@fontsource/sora/600.css";
@import "@fontsource/sora/700.css";

:root { --glass: rgb(255 255 255 / .62); --edge: rgb(255 255 255 / .8); --income: #035E43; /* … every token, light values */ }
.dark { --glass: rgb(20 22 38 / .62);    --edge: rgb(255 255 255 / .14); --income: #6EE7B7; /* … dark values */ }

@utility glass        { background: var(--glass);        border: 1px solid var(--edge); backdrop-filter: blur(22px) saturate(170%); box-shadow: var(--shadow-panel); }
@utility glass-raised { background: var(--glass-strong); border: 1px solid var(--edge); backdrop-filter: blur(32px) saturate(180%); box-shadow: var(--shadow-raised); }

/* §2.4: Windows' setting or the Reduce transparency switch (SET-8), through one variant */
@custom-variant transparency-off {
  @media (prefers-reduced-transparency: reduce) { @slot; }
  &:where(.reduce-transparency, .reduce-transparency *) { @slot; }
}
.glass, .glass-raised { @variant transparency-off { background: var(--glass-solid); } }
* { @variant transparency-off { backdrop-filter: none; } }

@theme inline {
  --color-income: var(--income);
  --color-expense: var(--expense);
  --font-sans: "Sora", "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
  /* … one line per token, so Tailwind classes like text-income work */
}
```

- The aurora is one component, `<AuroraBackground>`, rendered once behind the app shell.
- **Dark mode** is the `.dark` class on `<html>`. `lib/theme.ts` sets it from the `prefers-color-scheme` media query, and changes it while the app is open. **From Phase 1**, that follows the Windows setting.
- **The theme switch** (SET-5, Phase 5) is in Settings → Appearance: System, Light or Dark, stored as `theme`. Rust gives the window that theme (`set_theme`), and WebView2 reports it as `prefers-color-scheme`, so the CSS and `lib/theme.ts` are unchanged and the title bar matches. System hands the window back to Windows, so it follows a change made while the app is open. Rust applies it at startup before the page loads, after every save and after a restore, so the first paint already has the right theme (measured 2026-11-17: `main.tsx` ran before the first paint in every launch). Nothing in the frontend stores or works out the theme, and there's no `next-themes`.

### 3.6 Contrast on the other surfaces

The tables above are text on a panel. Text also sits on these surfaces, measured the same way (the worst spot, light / dark). With transparency off every surface is solid, and every figure is higher. Measured on 2026-11-23.

| Surface | Where | `--foreground` | `--muted-foreground` | `--primary` | `--income` | `--expense` | `--warning` |
|---|---|---|---|---|---|---|---|
| Raised (86%) | Dialogs, menus, toasts | 16.9 / 14.6 | 8.5 / 9.45 | 8.9 / 7.8 | 7.0 / 10.2 | 7.0 / 8.2 | 6.4 / 10.8 |
| A `--wash` card in a panel | The check-in's figures and offer cards, flat `StatCard`s, the Forecast's movements | 12.6 / 8.8 | 6.3 / 5.7 | 6.6 / 4.75 | 5.2 / 6.2 | 5.2 / 5.0 | 4.8 / 6.6 |
| A `--wash` row in a `--wash` card | Rows inside those cards | 11.5 / 7.3 | 5.8 / 4.71 | — | — | — | — |
| A field (`--field`) in a panel / a dialog | Inputs, `secondary` buttons | 17.4 / 9.0, 18.4 / 12.3 | 8.7 / 5.85, 9.2 / 7.95 | — | — | — | — |
| The net cashflow card (12% `--primary`) | The dashboard | 11.6 / 8.6 | 5.8 / 5.6 | — | 4.8 / 6.0 | 4.75 / 4.9 | — |
| A `warning` banner | Due-date and stale-backup banners | 11.5 / 8.0 (button text 16.6 / 6.8) | 5.8 / 5.2 | — | — | — | 4.4 / 5.9 (icon, 3:1) |
| An `info` banner | Recurring dates due | 11.2 / 8.2 | 5.6 / 5.3 | — | — | — | — |

- `--primary` as text on a banner would be 4.3 in dark mode. It isn't used: banner buttons are `secondary`.
- Field edges (`--field-edge`) are 1.4:1 against their surface, under WCAG's 3:1 for a control's edge. Fields are named by their labels, so they stay as they are (the owner, 2026-11-23). The offer card's empty circle, the one place the edge is the only cue, uses `--muted-foreground` (§9.2).
- Disabled controls are exempt from these figures (WCAG 1.4.3).

---

## 4. Typography

**Font: Sora**, a clean geometric sans-serif.
- **Bundled with the app** through the `@fontsource/sora` npm package, in weights 400, 600 and 700. No font is ever downloaded at runtime.
- **Checked on 2026-11-01:** Sora has **tabular (fixed-width) digits**, through `tabular-nums` (its default digits are proportional), and the true minus sign (U+2212). **Sora has no ₱ sign** (U+20B1), and neither do `▲` and `▼`.
- **Fallback:** `"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif`. **The ₱, ▲ and ▼ signs come from Segoe UI**, which ships with Windows 10 and 11, so they work offline. This is what the visual previews show. Don't add another font just for these signs.

**Numbers:** every amount uses `font-variant-numeric: tabular-nums`, so digits line up in columns.

| Style | Size / line height | Weight | Letter spacing | Used for |
|---|---|---|---|---|
| `display` | 30 / 36 px | 700 | −2% | Large amounts on stat cards and card details |
| `h1` | 24 / 32 px | 700 | −2% | Page titles |
| `h2` | 18 / 28 px | 600 | 0 | Section and dialog titles |
| `h3` | 16 / 24 px | 600 | 0 | Card titles |
| `body` | 14 / 20 px | 400 | 0 | Default text, list rows, inputs |
| `body-strong` | 14 / 20 px | 600 | 0 | Amounts in lists, emphasized labels |
| `small` | 12 / 16 px | 400 | 0 | Helper text, captions, chart axes |

- **Sentence case** everywhere ("Add transaction", not "Add Transaction"). No ALL-CAPS text.

---

## 5. Spacing, shape and depth

| Token | Value | Use |
|---|---|---|
| Spacing scale | 4 px base: 4, 8, 12, 16, 24, 32, 48 | Tailwind's default scale |
| Radius `chip` | 10 px | Category chips, icon squares, small buttons |
| Radius `control` | 12 px | Buttons, inputs, segmented controls, menu items |
| Radius `toolbar` | 16 px | Top bar, period bar, filter bar, menus, toasts, banners |
| Radius `panel` | 20 px | Cards, sidebar |
| Radius `dialog` | 24 px | Dialogs |
| Radius `pill` | 999 px | Progress bars, badges |
| Depth | `--shadow-panel`, `--shadow-raised` | §3.1 |
| Control height | 36 px (md), 32 px (sm) | Buttons, inputs, selects |
| Minimum click target | 32 × 32 px | Icon buttons |

### Layout: floating panels

| Element | Size |
|---|---|
| Window | **Fixed 1440 × 900**: not resizable or maximizable (minimize and close only), centered on the main monitor. If the monitor's usable area is too small (for example, display scaling above 100%), it opens at 1280 × 800 instead, showing the same layout zoomed to 88.9%. Every size below is for 1440 × 900. |
| Gaps | Every panel floats with a **12 px gap**, including from the window edge |
| Sidebar | A glass panel, 228 px wide, full height |
| Top bar | A glass panel holding the page title and the **+ Add** button |
| Period bar | Its own small glass toolbar under the top bar |
| Content column | 1176 px wide (1440 − 228 − 3 × 12 px gaps). The top bar stays in place, and the content below it scrolls inside the window. |
| Card padding | 18 px (lists) or 20 px (stat cards) |

---

## 6. Icons

- **lucide-react**, stroke width **1.75**.
- Sizes: **16 px** inline and in buttons, **20 px** in the sidebar, **24 px** in empty states.
- **Category chip:** a 32 px square with 10 px corners. It has the category color's tint as background, a 1 px inner `--edge`, and the icon in full category color. The small chip is 24 px with 8 px corners.
- **Category icon picker:** a curated list of about 40 icons in `src/lib/categoryIcons.ts`, which must include every seed icon from DATA_MODEL §7. Importing only these keeps the rest of Lucide out of the bundle.
- **Logo:** see §6.1.
- Fixed icons:

| Meaning | Icon |
|---|---|
| Income | `arrow-down-left` |
| Expense | `arrow-up-right` |
| Transfer | `arrow-left-right` |
| Cash account | `wallet` |
| Bank account | `landmark` |
| E-wallet | `smartphone` |
| Credit card | `credit-card` |
| Debt-free (sidebar, Phase 4) | `hand-coins` |
| Forecast (sidebar, Phase 4) | `calendar-range` |
| A loan payment due (banner) | `calendar-clock`, the same as a card payment due |
| A loan payment overdue, a pay window that runs short | `octagon-alert` (banner) and `triangle-alert` (in a row) |
| Payday | `banknote` |
| Level (sidebar, Phase 5) | `hexagon` |
| A focus (the dashboard, the Level screen) | `target` |
| A level reached, a focus kept, a condition met | `check` |
| A focus missed | `x` |
| A condition not met yet (the Level screen's checklist) | `circle` |
| A level not reached yet | `lock` |
| A stat or level up / down since the last check-in | `▲` / `▼`, as text, like the percent change (§7) |

The sidebar and banner icons are imported where they're used, like the ones above, and aren't part of the category icon list.

### 6.1 Logo: "Line Door"

A bank-vault door drawn in bold white lines on a rounded indigo tile, with the aurora's light glowing inside the tile. The door has a ring, two hinges on the left, a six-spoke handle wheel with round knobs, and a solid hub.

| File | Use |
|---|---|
| `assets/brand/logo.svg` | Master app icon: the tile plus the door. Used in the sidebar (30 px), the title bar (16 px), the taskbar and the installer. |
| `assets/brand/app-icon-1024.png` | 1024 × 1024 PNG with transparent corners. Run `npm run tauri icon assets/brand/app-icon-1024.png` to generate every Windows icon size. |
| `assets/brand/logo-mark-white.svg` | The door alone in white, for dark or colored backgrounds |
| `assets/brand/logo-mark-indigo.svg` | The door alone in indigo `#3730A3`, for light backgrounds |
| `assets/brand/concepts/` | The three original concepts, kept for reference |

- **Tile colors:** a diagonal gradient `#4338CA` → `#312E81` → `#1E1B4B`, with pink, blue, mint and peach light inside, a soft top sheen and a 35% white edge. The radius is about 23% of the tile's width.
- **Smallest size:** 16 px. The ring and wheel stay readable at that size, which was checked on dark and light taskbars.
- **Clear space:** keep at least one eighth of the icon's width empty around it.
- **Don't:** recolor the tile, rotate the door, add a ₱ or text inside it, or put the full-color icon on a busy photo.
- **Wordmark:** the mark plus "Jobert's Vault" set in Sora 700 (live text, not an image).

---

## 7. Money and number formatting

The currency is the **Philippine peso**, shown with the official sign **₱** (U+20B1), **before** the amount, with **2** decimal places. All formatting goes through `src/lib/money.ts` (ARCHITECTURE §5.3).

| Case | Format | Color |
|---|---|---|
| Income in lists | `+₱1,234.50` | `--income` |
| Expense in lists | `−₱1,234.50` (true minus sign, U+2212) | `--expense` |
| Transfer in lists | `₱1,234.50` with the ⇄ icon | `--transfer` |
| Account balance, positive | `₱1,234.50` | `--foreground` |
| Account balance, negative | `−₱1,234.50` | `--expense` |
| Credit card | `₱1,234.50 owed` | `--foreground` |
| Net cashflow | `+₱500.00` or `−₱500.00` | `--income` / `--expense` |
| Zero | `₱0.00` | `--muted-foreground` |
| Chart axis (compact) | `₱12.5k`, `₱1.2M` | `--muted-foreground` |

- Thousands separator `,` and decimal point `.`.
- **The amount input** accepts digits, one `.`, and pasted text containing commas. It shows thousands separators after you leave the field, and right-aligns the text. The stored value is in centavos (`1,234.50` → `123450`).

### Percent change (Phase 2)

Displayed as `▲ 12.3%` or `▼ 4.0%`, or `—` when there's nothing to compare against. No change is `0.0%` in `--muted-foreground`, with no arrow. Screen readers hear "Up" or "Down" instead of the arrow. A difference between two amounts is signed (`+₱126.00`, `−₱256.60`) in `--foreground`, since the % change already says whether it's good. In the comparison table, income and expense amounts are plain (`₱2,950.40`), because the row's group says which, so the difference adds up as shown. Only net cashflow keeps its sign.

| Measure | ▲ (up) | ▼ (down) |
|---|---|---|
| Income, net cashflow | `--income` (good) | `--expense` (bad) |
| Expense | `--expense` (bad) | `--income` (good) |

## 8. Date formatting

| Use | Format | Example |
|---|---|---|
| Day group headers | "Today", "Yesterday", or the weekday and date | `Mon, Oct 26` (`Mon, Oct 26, 2025` when it's not this year) |
| Day period label | weekday, date, year | `Mon, Oct 26, 2026` |
| Week period label | range | `Oct 26 – Nov 1, 2026` |
| Month period label | month and year, **or** a range when `month_start_day` > 1 | `October 2026` / `Sep 25 – Oct 24, 2026` |
| Year period label | year | `2026` |
| Date input | `MM/DD/YYYY` shown, `YYYY-MM-DD` stored | `10/26/2026` |
| Date and time (backups) | date, year, 12-hour time | `Oct 20, 2026 9:30 AM` |
| Pay window (Phase 4) | a range, with the year when it isn't this year | `Dec 5–14`, `Nov 30 – Dec 4`, `Dec 30, 2026 – Jan 14, 2027` |
| A payment due (Phase 4) | "due today", "due tomorrow", or the weekday and date | `due Fri, Nov 20` |
| An overdue payment (Phase 4) | "overdue since" and the weekday and date | `overdue since Mon, Nov 16` |

Dates always use **US order** (`10/26/2026`, `Oct 26`), whatever the Windows region setting.

---

## 9. Components

Built on shadcn/ui and restyled with the glass tokens. App-specific components live in `src/components/`.

### 9.1 Basics

| Component | Glass level | Notes |
|---|---|---|
| **Button** | Flat (never glass) | Four variants:<br>• `primary`: solid `--primary`, 1 px inner top highlight, soft accent glow `0 6px 18px` at 32%<br>• `secondary`: `--field` with `--field-edge`<br>• `ghost`: transparent, `--wash` on hover<br>• `destructive`: `--danger` with `--danger-foreground`<br>Sizes `md` (36 px, 12 px radius) and `sm` (32 px, 10 px radius). The button shrinks to 97% while pressed. The tooltip shows the shortcut. |
| **Input / Textarea** | Flat | `--field`, `--field-edge` border, 12 px radius. Focus: `--ring` border plus a 3 px ring at 25%. Error: `--danger` border plus an 18% ring, and the message below in `small` text. |
| **Select / Combobox** | Its menu is Level 2 | You can type to filter. Category options show their chip. Archived options are hidden. |
| **Autocomplete** | Its list is Level 2 | A text field that suggests what was entered before (the payee, TXN-6), in the select's list. Nothing is highlighted until ↓ or ↑, so Enter keeps the text as typed. |
| **Date picker** | Its popover is Level 2 | Weeks start on the `week_start` setting (Monday by default). In the input, ↑ and ↓ move one day. From Phase 3 future days can be picked too (TXN-8). Only the period switcher's Custom range picker disables days after today. The selected day is solid `--primary`. |
| **Segmented control** | Flat | A `--wash` track with 12 px radius. The selected segment is solid `--primary` with `--primary-foreground` text. The check-in's Balance check starts with neither segment selected (CHK-4). |
| **Toolbar** | Level 1 | A small glass bar (16 px radius, 6 px padding) for controls that sit on the canvas: the period bar, the filter bar, report controls. Inside a card it becomes a flat `--wash`. |
| **Dialog** | Level 2 | 24 px radius, max width 480 px (forms), 400 px (confirmations) or 640 px (a list whose rows have actions, such as the recurring dates due, which scrolls inside it past 60% of the window's height, and the Keyboard shortcuts table that `?` opens, UI_FLOWS §10). The backdrop is `--overlay` plus a 6 px blur of the app behind. Esc closes it, and focus stays inside. When it closes, focus goes back to what opened it, or to the ⋯ button for a dialog opened from a menu item, while that's still there and not behind another open dialog. |
| **Confirm dialog** | Level 2 | A title, one sentence, `secondary` Cancel, and a `primary` or `destructive` button that names the action. |
| **Toast** | Level 2 | Bottom-right, 16 px radius, several shown one above the other. Closes after 4 s. One with an action, such as **Open folder**, stays until it's closed with its ✕, so it never closes while the keyboard is on its action. The action is a ghost `sm` button in `--primary` text, under the sentence, so the sentence keeps the toast's width. Errors stay until closed, and screen readers read them out at once (`role="alert"`). A failed check-in Finish has the action **Open the app anyway** on its error toast (ADR 0014), and that toast sits bottom centre, between Back and Finish, so it never covers Finish. There's no Alt+T to jump to the toasts: they're reached by Tab. From Phase 4, a warning (a card spending level reached, CC-11) has the `triangle-alert` icon in `--warning` and stays 8 s. From Phase 5, a focus reaching 80% of its target or going past it (CHK-8) is the same warning toast. |
| **Banner** | Level 1, tinted | Panel glass mixed with the meaning color at 14%, and a border with it at 35% (§3.3). Has an icon, the message and an optional action. Variants: `warning`, `danger`, `info`. Inside another panel, the tint is flat (§2.3). The dashboard's banners all sit in one Level 1 panel, 6 px in and 6 px apart, as flat tinted rows with no border, so however many show they blur once (§11); the panel is gone when none shows. `components/Banner.tsx` has `warning` (`.glass-warning`, the backup warning and the card due date) and `info` (`.glass-info`, tinted with `--primary` as in the mockups, the recurring dates due). From Phase 4 it also has `danger` (`.glass-danger`, an overdue loan payment, tinted with `--danger-banner` at `--danger-banner-mix`, §3.3), and uses `warning` for a loan payment due. A banner can have a second line in `small` text (`detail`), such as "Move ₱800.00 there first." |
| **Tabs** | Flat | Inside Settings. A 2 px `--primary` underline marks the active tab. |
| **Skeleton** | Flat | `--wash` blocks with no shimmer, shown only if loading takes more than **150 ms**. |
| **Empty state** | Inside a panel | A 24 px icon, one line of text and one main action. |

### 9.2 App-specific

| Component | Description |
|---|---|
| **`AuroraBackground`** | Paints the canvas, the four lights and the grain once, behind the app shell (§2.2). |
| **`GlassPanel`** | The Level 1 wrapper that cards, the sidebar and toolbars are built on. Its `raised` prop switches it to Level 2. |
| **`MoneyText`** | Displays an amount using the §7 rules. Props: `amount`, `kind` (`income` / `expense` / `transfer` / `balance` / `owed` / `net`), `size`. |
| **`AmountInput`** | Converts what you type into centavos through `money.ts`. Right-aligned, tabular digits, never negative. |
| **`PeriodSwitcher`** | A glass toolbar: `[Day│Week│Month│Year│Custom]  ◀  October 2026  ▶  [Today]`. "Today" is disabled when the current period is already selected. ▶ is disabled when the next period would start after today. |
| **`StatCard`** | A label, a `display`-sized amount, and (from Phase 2) the % change (§7) with the dates compared with in `--muted-foreground`, such as "▲ 3.1% vs Sep 1–26", in `small` text. The **net cashflow** card is tinted with 12% of `--primary`. Clickable cards lift 2 px on hover. Inside a panel (the check-in), it's a flat `--wash` card with 12 px corners and no tint. |
| **`CategoryChip`** | The icon chip from §6, with the name next to it if needed. |
| **`AccountRow`** | Account icon in a `--wash` square, the name, and the balance on the right. A credit card also shows "Available ₱X · Due Nov 5". |
| **`TransactionRow`** | Category chip │ category, plus the payee and note in muted text │ account (or "Bank → Cash") │ amount, right-aligned. Hover adds `--glass-hover` and the edit and duplicate buttons. 52 px tall. |
| **`ProgressBar`** | 8 px pill on a `--wash` track. The fill is `--primary` below 80%, `--warning` from 80% and `--danger` from 100% (BUD-3). A goal's bar has no thresholds: `--primary`, and `--success` once the goal is achieved (GOL-4). A loan's bar is the same, filling as it's paid: `--primary`, and `--success` once it's paid off (LOAN-5). The % is always written next to it. |
| **`QuickAddDialog`** | The transaction form inside a Level 2 dialog (UI_FLOWS §4). |
| **`SafeToSpendCard`** (Phase 4) | A Level 1 card: "Safe to spend until Sun, Nov 29" ("until today" when the window ends today) in `--muted-foreground`, the amount in `display` size and `--foreground` (never green: it's what's left, not income), then "₱560.00 a day · 15 days left" and "₱1,500.00 held back for Nov 30 – Dec 4" in `small` text. When it's short, the amount is ₱0.00, and "Short by ₱1,240.00 on Fri, Dec 4" is in `--danger` with the `triangle-alert` icon, so it never relies on color alone (SAFE-6). |
| **`LoanCard`** (Phase 4) | A flat card inside the Loans panel, like a budget card: the name, "E-wallet · 3 of 8 paid", a `ProgressBar` of % paid, "₱5,000.00 left" and the next payment. An overdue next payment is in `--danger` with the `triangle-alert` icon. A paid-off loan has a **Paid off** badge in `--success` (UI_FLOWS §8.5). |
| **`SpendLevelTag`** (Phase 4) | The level a card's spending has reached this cycle (CC-10): "Soft level" in `--muted-foreground`, "Mid level" in `--warning` and "Hard level" in `--danger`, the last two with the `triangle-alert` icon, so it never relies on color alone. Nothing below soft. On the card's details it reads "Past your ₱3,000.00 soft level", or "Reached your …" when the spending is exactly the level (UI_FLOWS F16). |
| **`ForecastRow`** (Phase 4) | A row of the Forecast table, 52 px tall: the window's dates, then Start, Pay in, Bills and Left, right-aligned with tabular digits. The lowest window gets a **lowest** badge in `--muted-foreground`. A window that runs short has the `triangle-alert` icon, and its Left in `--danger` with the word "Short". Opening it shows its movements as flat rows in `--wash` (UI_FLOWS §8.6). |
| **`StepIndicator`** (Phase 5) | A row of 10 px dots joined by 24 px of 2 px `--wash` line: steps done are solid `--primary`, the current one is 14 px, a 2 px `--primary` ring around a 6 px solid center, and the ones ahead are `--wash`. The ahead dots and the line have the track's `--hairline` edge (§2.1). "Step 3 of 6 · Card" is written beside it in `small` text, 12 px after the dots, so it never relies on the dots alone. It's used by the check-in (UI_FLOWS §8.7). Not clickable. |
| **`StatHexagon`** (Phase 5) | The six stats as a radar chart (Recharts), 280 px, flat inside its panel: one corner per stat, evenly spread in the order Cashflow, Bills, Card, Debt, Savings, Habit, clockwise from the top. The grid is four rings (25, 50, 75 and 100) and a spoke to each corner, 1 px `--wash`. Today's shape is `--primary` at 24% with a 2 px `--primary` edge. Last check-in's shape is a 1.5 px dashed (4 3) outline in `--muted-foreground`, with no fill. A stat that doesn't apply has no corner, so the shape has fewer sides: with no card it's a pentagon. With fewer than 3 that apply, there's no chart, only the list. The corner labels are `small` text in `--muted-foreground`, just outside the 100 ring. The scores are always listed as numbers next to it, and a hidden table gives screen readers the same figures, as on the Reports charts. It doesn't animate. |
| **`LevelBadge`** (Phase 5) | A 40 px pointy-top hexagon with 3 px rounded corners, solid `--primary` with the level number in `--primary-foreground`, `h2` size and weight. Level 0 is a `--wash` hexagon with the number in `--muted-foreground`. Beside it, 12 px away, "Level 4 · In the black" in `h2` ("Level 0" alone at level 0). The **level chip** in the sidebar, at the right of the app name, is its 26 px version, with the number in `small` text, weight 600, inside a 34 px ring 2 px wide: the ring's track is `--hairline`, and its `--primary` arc, from the top and clockwise, is the share of the next level's conditions that are met. It shows the number alone, with no "Lv" (the owner, 2026-11-19: "Lv 4" doesn't fit beside the app name). Its label for screen readers says both: "Level 4, 1 of 2 conditions met for level 5". |
| **`LevelPath`** (Phase 5) | Ten 20 px nodes in a row, 64 px apart from center to center, joined by a 2 px line, numbered below in `small` text. Reached: solid `--primary` with a 12 px `check` in `--primary-foreground`. Current: the same, with a 3 px ring at 25%, like a focused field, and its number in `--foreground`, weight 600. Not reached: `--wash` with a 12 px `lock` in `--muted-foreground`, and the track's `--hairline` edge. The line is `--primary` up to the current node and `--wash` after it. Each node's tooltip has the level's name and conditions, and the nodes can be reached with the keyboard. |
| **`FocusRows`** (Phase 5) | One row per active focus, the month's first, flat inside its panel and divided by `--hairline`: a 16 px `target` icon in `--primary`, the sentence, the progress so far in `--muted-foreground`, and for a cap a `ProgressBar` with the budget thresholds under its sentence, across the row, with its % after it. A card level, a loan payment or logging has no bar. On the dashboard the rows sit at the top of the Safe to spend card, which links them to `/level`; the Level screen and the check-in show the same rows. It was planned as `FocusStrip`, a slim Level 1 panel of its own, but the dashboard is already past the blur budget (§11, UI_FLOWS §11 item 88). |
| **Focus offer card** (Phase 5) | A flat `--wash` card in the check-in's last step, 12 px radius, the cards one under the other 12 px apart: the sentence in `body` weight 600 and what it's based on in `small` text. Each has a 20 px circle in its top-right corner, 12 px in, empty with a `--muted-foreground` border (7:1, where `--field-edge` would be 1.4:1: the circle is what says a card isn't picked). Selected: a 2 px `--primary` border (2 px transparent otherwise, so nothing moves), and the circle filled `--primary` with a 12 px `check` in `--primary-foreground`, like a selected radio, never by tint alone. The cards form one radio group, and the focused card has the usual focus ring. |

---

## 10. Motion

- **Timing:** 150–200 ms, ease-out.
- **Hover and press:** clickable cards lift 2 px on hover. Buttons shrink to 97% while pressed.
- **Dialogs** fade in while growing from 98% to full size in 180 ms, and the backdrop fades in with them.
- **The level-up moment (Phase 5, LVL-6):** the one place a little more is allowed. When a check-in finds a higher level, the `LevelBadge`'s hexagon grows from 80% to 110% and settles at full size over 600 ms, ease-out, while a soft `--primary` glow behind it fades out, and its title beside it stays still. It plays once, when the step is first shown (UI_FLOWS §11 item 90). A lower level, the stat bars, the `StatHexagon` and the level path never animate.
- **What never moves:** the aurora, and numbers (they never count up).
- `prefers-reduced-motion` turns all of this off.

## 11. Performance

Glass costs graphics power, so these rules keep the app smooth:
- **Blur only Level 1 (22 px) and Level 2 (32 px)** surfaces. Never blur rows, chips, fields or buttons.
- Keep **about 12 or fewer** blurred elements on screen at once. Measured on 2026-11-23: the dashboard with every banner and two focus rows has 10 (12 with quick-add open, +1 per toast), Reports 7, and every other screen 5 or fewer. Before the banners shared a panel, the dashboard reached 14 (16 with quick-add) and still scrolled at a 6.2 ms p95 frame time on the owner's PC.
- Long lists scroll **inside** one blurred panel, so moving rows aren't re-blurred.
- Don't animate `backdrop-filter`, and don't put `will-change` on glass panels.
- With transparency off (§2.4), nothing is blurred.
- Check the dashboard and the transactions list scroll smoothly on your PC in every release (part of the PRD §10 checklist).

## 12. Writing style

- **Plain, short and friendly:** "Add transaction", "Delete this transaction?", "Saved".
- **Error messages say what to do:** "Amount must be more than 0", not "Invalid input".
- **Use the same words everywhere:** income, expense, transfer, account, category, owed, available, net cashflow, net balance.

## 13. Open questions

| # | Question | Proposed default |
|---|---|---|
| — | None at the moment | — |

## 14. Decisions

| Date | Decision |
|---|---|
| 2026-10-26 | Style: **Glassmorphism 2.0** |
| 2026-10-26 | Accent: deep indigo `#3730A3` in light mode, periwinkle `#A5B4FC` in dark |
| 2026-10-26 | Font: Sora, bundled with the app |
| 2026-10-26 | The aurora is painted by the app. The window is opaque, so the desktop wallpaper never shows through. |
| 2026-10-26 | Currency: ₱, before the amount, with 2 decimal places |
| 2026-10-26 | Added `--danger-foreground`, because white on the dark-mode red failed contrast |
| 2026-10-26 | Dates always use US order (`MM/DD/YYYY`) |
| 2026-10-26 | Income and expense category colors are numbered separately |
| 2026-10-26 | Logo: **C · Line Door**, a white line vault door on an indigo aurora tile (§6.1) |
| 2026-10-26 | A Reduce transparency switch in Settings → Appearance (SET-8, Phase 4) |
| 2026-10-26 | Fixed 1440 × 900 window, not resizable, with no responsive window layouts |
| 2026-11-01 | The app follows the Windows light or dark setting from Phase 1 |
| 2026-11-01 | The 1280 × 800 fallback zooms the same layout to 88.9% |
| 2026-11-01 | The date picker follows `week_start`, and category icons come from a curated list |
| 2026-11-01 | Sora has no ₱ sign, so ₱ comes from the Segoe UI fallback |
| 2026-11-06 | Income-vs-expense bars go up for income and down for expense, so the chart never relies on green versus red alone |
| 2026-11-13 | Phase 4: the `danger` banner for an overdue loan payment, the Debt-free and Forecast sidebar icons, and the `SafeToSpendCard`, `LoanCard` and `ForecastRow` components. Safe to spend is shown in `--foreground`, never green. |
| 2026-11-16 | Phase 4: the `SpendLevelTag` states and the warning toast (CC-10, CC-11) |
| 2026-11-17 | The theme switch (SET-5) sets the window's theme from Rust rather than the class from a frontend copy of the setting, so the first paint and the title bar are right too (§3.5) |
| 2026-11-17 | Phase 5: the check-in is a full-window screen with one glass panel, never a dialog, so the forms it opens aren't glass inside glass (§2.3, ADR 0012) |
| 2026-11-17 | Phase 5: the `StepIndicator`, `StatHexagon`, `LevelBadge` (and the sidebar's level chip), `LevelPath`, `FocusStrip` and focus offer card, the Level sidebar icon, and the level-up moment, the only animation longer than 200 ms. They use existing tokens only: the level is always `--primary`, never gold or a color of its own, and no stat has its own color. |
| 2026-11-18 | Windows' transparency setting and the Reduce transparency switch share one `transparency-off` variant, and it unblurs the dialog backdrop too (§2.4) |
| 2026-11-18 | The `danger` banner gets its own tint, `--danger-banner` at `--danger-banner-mix` (`#DC2626` at 30% in dark mode, unchanged in light), so an overdue loan payment reads as red beside a warning in dark mode (§3.3, LOAN-4) |
| 2026-11-18 | The banners' 35% tinted border (§3.3) was in this doc and the preview but never built in the app. The owner chose to build it, for every tone |
| 2026-11-19 | The sidebar's level chip shows the level number in its hexagon, not "Lv 4": there's about 40 px beside the app name, which holds the ring but not the text beside it (the owner, §9.2, PRD LVL-5) |
| 2026-11-19 | Drawing the Phase 5 previews settled the sizes left open in §9.2 (the current step's dot, the line lengths, the path's spacing, the chip's ring, the strip's bar, the offer card's circle). A `--wash` track has the 1 px `--hairline` edge the ProgressBar's track already had (§2.1), and an unmet condition is a `circle` (§6) |
| 2026-11-20 | Building the check-in: a `StatCard` inside a panel is flat and untinted, a `Segmented` can start with nothing selected (the Balance check), and an error toast can carry an action (Open the app anyway, ADR 0014) (§9.1, §9.2) |
| 2026-11-20 | Building the Level screen: a `LevelPath` node's tooltip is raised glass (Level 2) like a menu, and opens on focus as well as hover; each node's label names its state, so it never rests on color or the check and lock icons alone. An unchanged stat score shows `—` (§9.2, UI_FLOWS §11 items 84, 85) |
| 2026-11-20 | The dashboard's focus rows are flat at the top of the Safe to spend card, not a `FocusStrip` panel: with every banner showing once the dashboard has 16 blurred panels (12 with none), already past §11's "about 12" (a strip would make it 17), so the fallback UI_FLOWS §2 named was built. The component is `FocusRows` (§9.2, UI_FLOWS §11 item 88) |
| 2026-11-21 | The Keyboard shortcuts dialog (`?`, UI_FLOWS §10) is the 640 px dialog: a three-column table (Shortcut, Action, Where) with each key in a `--wash` `<kbd>` chip, `←` for ArrowLeft and `Esc` for Escape, and the check-in sentence under it in `small` muted text (§9.1) |
| 2026-11-23 | The accessibility pass (T24, decided with the owner): a dialog gives focus back to what opened it; a toast with an action stays until closed, with the action under the sentence; errors are alerts; Sonner's Alt+T is off; a failed Finish's toast sits bottom centre; the offer card's empty circle has a `--muted-foreground` border; light `--income`/`--success` and `--expense`/`--danger` are one step darker, so every text color passes at the worst spot on every surface (§3, §9.1, §9.2) |
| 2026-11-23 | The dashboard's banners share one Level 1 panel as flat tinted rows, so every banner showing at once blurs once: the dashboard's most is 10 blurred surfaces instead of 14 (the performance pass, the owner chose this over recording 14 as an exception) (§9.1, §11) |
