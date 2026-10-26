# Design System: Jobert's Vault

| | |
|---|---|
| **Status** | Approved v1.0 |
| **Style** | **Glassmorphism 2.0**: frosted glass panels floating over a soft colored light (the "aurora") |
| **Last updated** | 2026-10-26 |
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
| Rows, hovers, tracks, totals | `--wash` |
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
- Blur is turned off.
- The aurora, shapes, colors and layout stay the same.

Settings → Appearance also has a **Reduce transparency** switch (SET-8, Phase 4), stored as `reduce_transparency`, that does the same for this app alone.

---

## 3. Color tokens

All colors are **CSS variables** in `src/styles/globals.css`. Components use the token names, never raw values. The names follow shadcn/ui's conventions, plus the glass and app-specific tokens below.

**Contrast is measured at the worst spot:** text on a 62% panel over the strongest aurora color. WCAG AA requires 4.5:1 for text. **Every text color passes**, and the lowest is light-mode `--expense` / `--danger` at **4.95:1**. Category colors are used as icons, which need 3:1, and their worst case is 3.7:1 (light) and 6.4:1 (dark). Values were checked on 2026-10-26. Re-check any token you change.

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
| `--foreground` | Main text | `#0B1020` | `#F5F7FF` | 14.0 / 11.3 |
| `--muted-foreground` | Secondary text, labels | `#3D4560` | `#C3C9DE` | 7.0 / 7.3 |
| `--primary` | Accent: main buttons, active nav, links | `#3730A3` (deep indigo) | `#A5B4FC` (periwinkle) | 7.3 / 6.0 |
| `--primary-hover` | Primary hover | `#312E81` | `#C7D2FE` | — |
| `--primary-foreground` | Text on primary | `#FFFFFF` | `#0B1020` | 9.9 / 9.5 |
| `--ring` | Focus ring | `#3730A3` | `#A5B4FC` | — |

### 3.3 Meaning (money and status)

| Token | Purpose | Light | Dark | Worst-case contrast (L / D) |
|---|---|---|---|---|
| `--income` | Income, increases that are good | `#03684A` | `#6EE7B7` | 5.0 / 7.9 |
| `--expense` | Expenses, increases that are bad | `#B4183A` | `#FDA4AF` | 4.95 / 6.4 |
| `--transfer` | Transfers (neutral) | `#3D4560` | `#C3C9DE` | 7.0 / 7.3 |
| `--warning` | Budget ≥ 80%, due-date banner, stale-backup notice | `#8A4500` | `#FCD34D` | 5.3 / 8.4 |
| `--danger` | Budget ≥ 100%, delete, errors (same red as `--expense`) | `#B4183A` | `#FDA4AF` | 4.95 / 6.4 |
| `--danger-foreground` | Text on a danger button | `#FFFFFF` | `#0B1020` | 6.7 / 10.0 |
| `--success` | Goal achieved, "Saved" toasts | `#03684A` | `#6EE7B7` | 5.0 / 7.9 |

- **Tints:** each meaning color has a tint for chips and banners. It's the color mixed into transparent at **16%** in light mode and **20%** in dark (`--tint`).
- **Banners** are panel glass with the meaning color mixed in at 14%, and a border with it mixed in at 35%.

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
- Income-vs-expense charts always use `--income` and `--expense`, never category colors.
- **Income and expense colors are numbered separately** (DATA_MODEL §7), because the breakdown chart never shows both together. The 12 most common expense categories each have their own color. Only 5 rarely used ones share (Education, Gifts & Donations, Travel, Fees & Charges, Other expense).
- A category the user adds defaults to the least-used color of its kind.

### 3.5 Implementation (Tailwind 4)

```css
/* src/styles/globals.css */
@import "tailwindcss";
@import "@fontsource/sora/400.css";
@import "@fontsource/sora/600.css";
@import "@fontsource/sora/700.css";

:root { --glass: rgb(255 255 255 / .62); --edge: rgb(255 255 255 / .8); --income: #03684A; /* … every token, light values */ }
.dark { --glass: rgb(20 22 38 / .62);    --edge: rgb(255 255 255 / .14); --income: #6EE7B7; /* … dark values */ }

@utility glass        { background: var(--glass);        border: 1px solid var(--edge); backdrop-filter: blur(22px) saturate(170%); box-shadow: var(--shadow-panel); }
@utility glass-raised { background: var(--glass-strong); border: 1px solid var(--edge); backdrop-filter: blur(32px) saturate(180%); box-shadow: var(--shadow-raised); }

@media (prefers-reduced-transparency: reduce) {
  .glass, .glass-raised { background: var(--glass-solid); backdrop-filter: none; }
}

@theme inline {
  --color-income: var(--income);
  --color-expense: var(--expense);
  --font-sans: "Sora", "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
  /* … one line per token, so Tailwind classes like text-income work */
}
```

- The aurora is one component, `<AuroraBackground>`, rendered once behind the app shell.
- **Dark mode** is the `.dark` class on `<html>`. It follows the Windows setting until the theme switch (SET-5) arrives in Phase 4.

---

## 4. Typography

**Font: Sora**, a clean geometric sans-serif.
- **Bundled with the app** through the `@fontsource/sora` npm package, in weights 400, 600 and 700. Nothing is downloaded at runtime, because the app has no network access.
- **Checked on 2026-10-26:** Sora has **tabular (fixed-width) digits** and includes the **₱** sign.
- **Fallback:** `"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif`.

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
| Window | **Fixed 1440 × 900**: not resizable or maximizable (minimize and close only), centered on the main monitor. If the monitor's usable area is too small (for example, display scaling above 100%), it opens at 1280 × 800 instead. |
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

Displayed as `▲ 12.3%` or `▼ 4.0%`, or `—` when there's nothing to compare against.

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
| **Date picker** | Its popover is Level 2 | Weeks start on Monday. In the input, ↑ and ↓ move one day. The selected day is solid `--primary`. |
| **Segmented control** | Flat | A `--wash` track with 12 px radius. The selected segment is solid `--primary` with `--primary-foreground` text. |
| **Toolbar** | Level 1 | A small glass bar (16 px radius, 6 px padding) for controls that sit on the canvas: the period bar, the filter bar, report controls. Inside a card it becomes a flat `--wash`. |
| **Dialog** | Level 2 | 24 px radius, max width 480 px (forms) or 400 px (confirmations). The backdrop is `--overlay` plus a 6 px blur of the app behind. Esc closes it, and focus stays inside. |
| **Confirm dialog** | Level 2 | A title, one sentence, `secondary` Cancel, and a `primary` or `destructive` button that names the action. |
| **Toast** | Level 2 | Bottom-right, 16 px radius. Closes after 4 s. Errors stay until closed. |
| **Banner** | Level 1, tinted | Panel glass mixed with the meaning color at 14%. Has an icon, the message and an optional action. Variants: `warning`, `danger`, `info`. |
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
| **`PeriodSwitcher`** | A glass toolbar: `[Day│Week│Month│Year│Custom]  ◀  October 2026  ▶  [Today]`. "Today" is disabled when the current period is already selected. |
| **`StatCard`** | A label, a `display`-sized amount, and (from Phase 2) the % change with a caption such as "vs Sep 1–26". The **net cashflow** card is tinted with 12% of `--primary`. Clickable cards lift 2 px on hover. |
| **`CategoryChip`** | The icon chip from §6, with the name next to it if needed. |
| **`AccountRow`** | Account icon in a `--wash` square, the name, and the balance on the right. A credit card also shows "Available ₱X · Due Nov 5". |
| **`TransactionRow`** | Category chip │ category, plus the payee and note in muted text │ account (or "Bank → Cash") │ amount, right-aligned. Hover adds `--glass-hover` and the edit and duplicate buttons. 52 px tall. |
| **`ProgressBar`** | 8 px pill on a `--wash` track. The fill is `--primary` below 80%, `--warning` from 80% and `--danger` from 100% (BUD-3). The % is always written next to it. |
| **`QuickAddDialog`** | The transaction form inside a Level 2 dialog (UI_FLOWS §4). |

---

## 10. Motion

- **Timing:** 150–200 ms, ease-out.
- **Hover and press:** clickable cards lift 2 px on hover. Buttons shrink to 97% while pressed.
- **Dialogs** fade in while growing from 98% to full size in 180 ms, and the backdrop fades in with them.
- **What never moves:** the aurora, and numbers (they never count up).
- `prefers-reduced-motion` turns all of this off.

## 11. Performance

Glass costs graphics power, so these rules keep the app smooth:
- **Blur only Level 1 (22 px) and Level 2 (32 px)** surfaces. Never blur rows, chips, fields or buttons.
- Keep **about 12 or fewer** blurred elements on screen at once.
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
