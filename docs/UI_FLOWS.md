# UI Flows: Jobert's Vault

| | |
|---|---|
| **Status** | Approved v1.1 |
| **Style** | Glassmorphism 2.0 ([DESIGN_SYSTEM.md](DESIGN_SYSTEM.md)) |
| **Last updated** | 2026-11-06 |
| **Related docs** | [PRD.md](PRD.md), [DESIGN_SYSTEM.md](DESIGN_SYSTEM.md), [ARCHITECTURE.md](ARCHITECTURE.md) §5.4 (routes) |

The sketches below are rough layouts, not pixel-perfect designs. Spacing, colors, glass levels and components come from DESIGN_SYSTEM.md. [design/ui-flows.html](design/ui-flows.html) shows every screen drawn in the real style, with consistent ₱ sample data. Open it in a browser.

---

## 1. App shell

```
┌─ window: aurora background (soft colored light) ─────────────────────────────────────┐
│ ╭──────────────────╮  ╭────────────────────────────────────────────────────────────╮ │
│ │ ◆ Jobert's Vault │  │ Dashboard                                  [+ Add  Ctrl+N] │ │
│ │                  │  ╰────────────────────────────────────────────────────────────╯ │
│ │ ▣ Dashboard      │  ╭─────────────────────────────────────────────────────╮        │
│ │ ≡ Transactions   │  │ [Day│Week│Month│Year│Custom]  ◀ October 2026 ▶  Today │        │
│ │ ◫ Accounts       │  ╰─────────────────────────────────────────────────────╯        │
│ │ ◔ Reports   (P2) │  ╭──────────────╮ ╭──────────────╮ ╭──────────────╮             │
│ │ ↻ Recurring (P3) │  │    card      │ │    card      │ │    card      │             │
│ │ ◎ Budgets   (P3) │  ╰──────────────╯ ╰──────────────╯ ╰──────────────╯             │
│ │ ⚑ Goals     (P3) │                     (page content)                              │
│ │ ──────────────── │                                                                 │
│ │ ⚙ Settings       │                                                                 │
│ ╰──────────────────╯                                                                 │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

- **Material:** every rounded box is a frosted glass panel (Level 1) floating **12 px** apart over the aurora. Dialogs, menus, the date picker and toasts are raised glass (Level 2). Text never sits straight on the aurora. See DESIGN_SYSTEM §2.
- **Sidebar:** a glass panel with the app mark and name at the top, the screens in the middle and Settings at the bottom. The current screen gets a soft indigo tint. Items from later phases are **hidden** until their phase is built.
- **Top bar:** a glass panel with the page title on the left. The **"+ Add" button** (Ctrl+N) is always on the right and opens quick-add on every screen.
- **Period bar:** on screens with a period, the period switcher is its own small glass toolbar under the top bar.
- **Window:** a fixed **1440 × 900**. It can't be resized or maximized, so the title bar has minimize and close only. It opens centered on the main monitor. If 1440 × 900 doesn't fit, it opens at 1280 × 800 with the same layout zoomed to 88.9%. The sidebar and top bar stay in place, and the content below the top bar scrolls.
- The window title is "Jobert's Vault", with the app logo.

---

## 2. Dashboard (`/`), Phase 1

```
┌───────────────────────────────────────────────────────────────────────────────┐
│ [Day│Week│Month│Year│Custom]     ◀  October 2026  ▶     [Today]               │
├───────────────────────────────────────────────────────────────────────────────┤
│ ┌─ Income ─────────────┐ ┌─ Expense ────────────┐ ┌─ Net cashflow ───────────┐ │
│ │ +₱4,200.00           │ │ −₱2,950.40           │ │ +₱1,249.60               │ │
│ │ ▲ 3.1% vs Sep 1–26 (P2)│ │ ▼ 8.0% vs Sep 1–26 (P2)│ │ ▲ 44.2% vs Sep 1–26 (P2) │ │
│ └──────────────────────┘ └──────────────────────┘ └──────────────────────────┘ │
│                                                                               │
│ ┌─ Recent transactions ────────────── View all ┐ ┌─ Accounts ─────────────────┐ │
│ │ Today                                        │ │ 👛 Cash          ₱320.00   │ │
│ │ 🛒 Food & Groceries · Market   −₱85.20       │ │ 🏛 Bank        ₱8,410.75   │ │
│ │ ⇄  Bank → Credit card            ₱1,200.00   │ │ 📱 E-wallet      ₱150.00   │ │
│ │ Yesterday                                    │ │ 💳 Credit card             │ │
│ │ 🍴 Dining Out · Corner Cafe    −₱12.50       │ │    ₱640.00 owed            │ │
│ │ 💼 Salary · Employer          +₱4,200.00     │ │    Available ₱9,360.00     │ │
│ │ 🚗 Transportation              −₱20.00       │ │    Due Nov 5               │ │
│ └──────────────────────────────────────────────┘ │ ────────────────────────── │ │
│                                                  │ Net balance    ₱8,240.75   │ │
│                                                  └────────────────────────────┘ │
└───────────────────────────────────────────────────────────────────────────────┘
```

| Area | Behavior | PRD |
|---|---|---|
| Period switcher | Changes every number on the page. The choice is saved in the URL (`?period=month&anchor=2026-10-26`, or `?period=custom&start=…&end=…`). It's a glass toolbar of its own. | DSH-1 |
| Stat cards | Income, expense and net cashflow for the period. Transfers aren't included. The % change line appears in Phase 2. The net cashflow card is the only one tinted with the accent. | DSH-2, DSH-3 |
| Income vs expense (Phase 2) | A small version of the Reports chart (§8.1) for the period, under the stat cards. Its bars come with the rest of the dashboard from `get_dashboard`. | RPT-1 |
| Recent transactions | The 5 newest transactions **regardless of the selected period**. Clicking a row opens it for editing. "View all" goes to `/transactions`. | DSH-8 |
| Accounts | Current balances (as of today), with the credit card summary and the net balance. Clicking an account opens its details. | DSH-4, DSH-5 |

**Added later:**
- **Phase 3:**
  - a "Card payment due in 3 days: ₱640.00 owed" banner at the top (CC-7), with a **Pay bill** button
  - budget progress bars (DSH-6)
  - recurring transactions coming up in the next 7 days (DSH-7), plus a "2 recurring transactions need confirming" banner

---

## 3. Transactions (`/transactions`), Phase 1

```
┌───────────────────────────────────────────────────────────────────────────────┐
│ [Day│Week│Month│Year│Custom]     ◀  October 2026  ▶     [Today]               │
│ 🔍 Search payee or note…   [Type ▾] [Account ▾] [Category ▾] [Method ▾]  Clear │
├───────────────────────────────────────────────────────────────────────────────┤
│ 38 transactions · Income +₱4,200.00 · Expense −₱2,950.40                      │
├───────────────────────────────────────────────────────────────────────────────┤
│ Today, Mon Oct 26                                               −₱85.20       │
│   🛒 Food & Groceries   Market · weekly shop     Cash          −₱85.20  ✎ ⧉   │
│   ⇄  Transfer           October bill    Bank → Credit card    ₱1,200.00        │
│ Yesterday, Sun Oct 25                                         +₱4,167.50      │
│   🍴 Dining Out         Corner Cafe              Credit card   −₱12.50         │
│   💼 Salary             Employer                 Bank       +₱4,200.00         │
│   …                                                                           │
│                              (more load as you scroll)                         │
└───────────────────────────────────────────────────────────────────────────────┘
```

- **Grouped by day**, newest first. Each day header shows that day's net (income − expense; transfers don't count). (TXN-3)
- **Filters** (TXN-4) sit in a glass toolbar under the period bar: period, type, account, category and payment method. The payment method filter is hidden when the field is turned off (SET-6). Search looks in the payee and note. **All filters are saved in the URL.** A summary line shows the number of matches and their totals.
- **Clicking a row** opens the edit dialog. **Hover buttons:** ✎ edit, ⧉ duplicate (TXN-2). **Delete** is inside the edit dialog.
- Rows load in **pages of 100** as you scroll (ARCHITECTURE §12).
- **Future-dated** transactions (Phase 3) appear above "Today" under an **Upcoming** header, in muted text.

---

## 4. Quick-add and edit dialog (Phase 1)

```
┌─ Add transaction ─────────────────────────────── ✕ ┐
│ [ Expense │ Income │ Transfer ]                     │
│                                                     │
│ Amount        [                      250.00 ]       │
│ Category      [ 🍴 Dining Out              ▾ ]       │
│ Account       [ 💳 Credit card              ▾ ]      │
│ Date          [ 10/26/2026                 📅 ]      │
│ Payee         [ Corner Cafe                   ]     │
│ Method        [ Credit Card                ▾ ]  ←hidden if SET-6 is off
│ Note          [                               ]     │
│                                                     │
│          [Cancel]  [Save & add another]  [Save]     │
└─────────────────────────────────────────────────────┘
```

The dialog is raised glass (Level 2). The app behind it dims and blurs slightly.

**Fields and defaults**

| Field | Default | Notes |
|---|---|---|
| Type | **Expense** | Alt+E / Alt+I / Alt+T switch between Expense, Income and Transfer. |
| Amount | empty, **focused when the dialog opens** | Must be > 0. |
| Category | empty | Type to filter. Only categories of the selected type are listed. |
| Account | **the account used most recently** (TXN-1) | Archived accounts are hidden. |
| Date | **today** | ↑/↓ change it by one day. Until Phase 3, a future date shows "Date can't be in the future". |
| Payee, Method, Note | empty | All optional. Payee suggestions start in Phase 2 (TXN-6). |

**For a transfer** the fields become: Amount → **From** account → **To** account → Date → Note. There's no category, payee or method. "To" can't be the same as "From".

**Keyboard (the "under 5 seconds" goal):**
`Ctrl+N` → type `250` → `Tab` → type `din` → `Enter` (picks Dining Out) → `Enter` (saves). That's about 3 seconds.

| Key | Action |
|---|---|
| `Enter` | Save (when no dropdown is open) |
| `Shift+Enter` | **Save & add another**: saves, keeps the type, account and date, and clears the other fields |
| `Esc` | Cancel. If anything has been typed, asks "Discard changes?" first. |

**After saving:** the dialog closes, a "Saved" toast appears, and every balance and total refreshes.

**Edit mode:** the title is "Edit transaction". A **Delete** button (destructive) sits bottom-left and opens a confirm dialog ("Delete this transaction? This can't be undone.").

**Errors** appear under the field they belong to, using the backend's validation messages (ARCHITECTURE §4.4).

---

## 5. Accounts

### 5.1 Account list (`/accounts`), Phase 1

```
┌───────────────────────────────────────────────────────────────────────┐
│ Accounts                                              [+ New account] │
├───────────────────────────────────────────────────────────────────────┤
│ ⋮⋮ 👛 Cash              Cash                             ₱320.00       │
│ ⋮⋮ 🏛 Bank              Bank                           ₱8,410.75       │
│ ⋮⋮ 📱 E-wallet          E-wallet                         ₱150.00       │
│ ⋮⋮ 💳 Credit card       Credit card    ₱640.00 owed · 6% of limit      │
├───────────────────────────────────────────────────────────────────────┤
│ Net balance                                            ₱8,240.75       │
│ ▸ Show archived (1)                                                   │
└───────────────────────────────────────────────────────────────────────┘
```

- Drag the ⋮⋮ handle to reorder accounts (`reorder_accounts`), or focus it and use the arrow keys. Categories and payment methods reorder the same way.
- Archived accounts are collapsed at the bottom. Their balances still count toward the net balance.

### 5.2 New / edit account dialog

| Field | Notes |
|---|---|
| Name | Required, must be unique |
| Type | Cash / Bank / E-wallet / Credit card. **Can't be changed after the account is created**, because that would change how its balance works. |
| Opening balance | 0 or more. For a credit card the label changes to **"Amount owed now"** (DATA_MODEL §5.1). |
| *(Credit card only)* Credit limit, Statement closing day, Payment due day | Days are 1–31, with the help text "If a month is shorter, the last day is used." |

### 5.3 Account details (`/accounts/:id`)

**Cash, bank or e-wallet:** the name, the current balance in `display` size, and that account's transactions, including incoming transfers (the same list and period switcher as `/transactions`, already filtered to the account). Edit and archive actions are in a ⋯ menu, which also has **Delete** if the account has never been used.

**Credit card** (CC-6):

```
┌─ 💳 Credit card ──────────────────────────────── [Pay bill] [⋯] ┐
│ ₱640.00 owed                                                    │
│ ████░░░░░░░░░░░░░░░░░░░░  6% of ₱10,000.00 limit                 │
│ Available credit   ₱9,360.00                                    │
│ This cycle         ₱412.30 spent   (Oct 21 – Nov 20)            │
│ Statement closes   Nov 20          Payment due   Nov 5          │
├─────────────────────────────────────────────────────────────────┤
│ (this card's transactions)                                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. Settings (`/settings`), Phase 1

| Tab | Contents | PRD |
|---|---|---|
| **General** | Currency symbol, its position (before or after the amount), decimal places (**locked** once any amount is saved, with an explanation), first day of week, month start day (1–28), show payment method field. **Updates:** the current version, **Check for updates** and a switch, "Install updates automatically when the app starts" | SET-1, SET-2, SET-6, UPD-3 |
| **Categories** | Two lists (Income and Expense). Add, rename, change color (12 swatches) and icon (curated list), reorder, archive, unarchive, **merge**, delete if unused | CAT-2, CAT-4 |
| **Payment methods** | Add, rename, reorder, archive, unarchive, delete if unused | TXN-7 |
| **Backup** (Phase 2) | See §8.1 | BAK-* |
| **Appearance** (Phase 4) | Theme: system / light / dark, and a **Reduce transparency** switch | SET-5, SET-8 |

Changes to settings save **immediately**, with a "Saved" toast. There's no Save button.

---

## 7. Key flows

### F1: Log an expense (Phase 1)
1. Press **Ctrl+N** (or click **+ Add**) from any screen.
2. Type the amount, then pick a category (type a few letters, Enter).
3. The account and date already default to the last-used account and today. Change them if needed.
4. Press **Enter** to save. A toast appears, and the dashboard and lists update.

### F2: Pay the credit card bill (Phase 1, CC-9)
1. On the card's detail page, click **Pay bill**. (In Phase 3 this button is also on the due-date banner.)
2. The transfer form opens already filled in:
   - **To** = the card
   - **From** = the latest non-card account used, or else the first active non-card account (DATA_MODEL §5.8)
   - **Amount** = the current amount owed (empty if nothing is owed)
   - **Note** = "Credit card payment"
3. Change the amount if you're paying part of the bill, then save.
4. The amount owed goes down. The month's expense total **doesn't change**.

### F3: Record a refund on the card (Phase 1)
1. Quick-add → **Income** → amount → category **Refund** → account = the card → save.
2. The amount owed goes down, and income goes up (DATA_MODEL §5.2).

### F4: Edit, duplicate or delete a transaction (Phase 1)
- **Edit:** click the row → change it → Save.
- **Duplicate:** ⧉ on the row opens the form already filled in with **today's** date → Save.
- **Delete:** click the row → **Delete** → confirm.

### F5: First launch (Phase 1 basic version ONB-1, Phase 4 polished version ONB-2)

```
Welcome to Jobert's Vault
  Step 1  Currency: [ ₱ ▾ ]  Position: [Before ▾]  Decimals: [2 ▾]
  Step 2  Your accounts: Cash (already added) · [+ Bank] [+ E-wallet] [+ Credit card]
          (name + opening balance; card asks for its limit, statement day, due day)
  Step 3  [Start using the app]
```

- The **Phase 1** version is a plain form, because the currency must be set before the first transaction. **Phase 4** adds polish: a step indicator and a short tour.
- It can be skipped ("Set up later"). Every setting can be changed later in Settings.
- It lives at `/welcome`. **Start using the app** or **Set up later** ends it for good (`onboarding_completed`). Closing the app before choosing either one shows it again on the next launch.

### F6: Archive an account (Phase 1)
1. Account ⋯ menu → **Archive**.
2. **If the balance is 0:** a confirm dialog says "Archive Old Wallet? It will be hidden from lists but its history stays."
3. **If the balance isn't 0:** a warning dialog says "Old Wallet still has ₱45.00. Archived accounts still count toward your net balance. Archive anyway?" with the buttons **Cancel** and **Archive anyway** (ACC-3).
4. Archived accounts can be restored with **Unarchive** in the "Show archived" section.
5. **Last active account:** it can't be archived or deleted. The app shows "Keep at least one active account".
6. **Unused account:** the ⋯ menu also offers **Delete**, after a confirm dialog.

### F7: Archive or merge a category (Phase 1)
1. Settings → Categories → ⋯ on the category → **Archive** or **Merge into…**.
2. **Merge:** pick a category of the same kind → confirm: "Move 23 transactions from 'Snacks' to 'Dining Out' and delete 'Snacks'?" (CAT-4)
3. A category with no transactions can simply be **deleted**.
4. Archived categories can be restored with **Unarchive**. The last active category of each kind can't be archived or deleted.

### F8: Look at a different period (Phase 1)
- Use the segmented control to switch between Day, Week, Month and Year. ◀ ▶ move one period back or forward, and **Today** jumps back to the current period. Custom opens a date range picker that can't choose future dates, and ◀ ▶ then move by the length of the range. ▶ is disabled once the next period would start after today.
- **Phase 2** adds a "Compare to: Same days so far ▾ / Full previous period" option on Reports (RPT-4a, SET-7).

### F9: Back up and restore (Phase 2)
- **Back up now:** Settings → Backup → **Back up now** → toast "Backup saved to …" with an **Open folder** link.
- **Restore:** **Restore from backup…** → pick a file → confirm "This replaces all current data with the backup from Oct 20, 2026 09:30. A safety copy of your current data will be saved first." → the app reloads.
- **Stale-backup warning:** if the last backup is more than 7 days old, a warning banner appears in Settings and on the dashboard (BAK-7).

### F10: Confirm recurring transactions (Phase 3)
1. A banner on the dashboard says "2 recurring transactions are due" → **Review**.
2. Each one shows **Post**, **Edit & post** or **Skip** (REC-3).

### F11: Update the app (Phase 1)
- **Automatic (the default):** at startup, the app checks for a newer version. "Checking for updates…" appears only if the check takes more than 150 ms. If there's a newer version, "Updating Jobert's Vault to 0.2.0…" appears, and the app closes and reopens on it. There's nothing to click (UPD-1).
- **No internet, or the check fails:** the app starts normally and shows no error (UPD-2).
- **By hand:** Settings → General → **Check for updates** says "You're up to date." or "0.2.0 is out." with an **Update now** button. It restarts the app, so anything unsaved in an open form is lost: the button is in Settings, where no form is open (UPD-3).
- **Automatic updates off:** the app doesn't connect by itself. **Check for updates** still works (UPD-3).
- **If an update is refused or fails:** the message says why and adds "Nothing was changed." (UPD-4).

---

## 8. Phase 2–4 screens (outline)

### 8.1 Reports (`/reports`), Phase 2
- Period switcher plus a "Compare to" dropdown and an account filter (RPT-5).
- **Section 1: Income vs expense.** A bar chart grouped by day, week or month (RPT-1, DATA_MODEL §5.9), with net cashflow as a line. Income bars rise above the zero line and expense bars hang below it, so they're told apart by direction as well as color. Hovering a bar shows its dates and the three figures. A hidden table gives screen readers the same figures.
- **Section 2: Category breakdown.** A donut chart showing the largest 8 categories plus "Other", next to a ranked list showing each category's amount and %. Switch between Expense and Income. Clicking a category opens the filtered transaction list (RPT-2, RPT-3).
- **Section 3: Comparison table.** Rows for income, expense, net and each category, with columns for this period, the comparison period, the difference and the % change (RPT-4).

### 8.2 Recurring (`/recurring`), Phase 3
A list of rules showing the next date, amount, frequency, mode (Auto or Confirm) and whether it's paused. "+ New rule" opens the transaction form plus a schedule section: frequency, every *N*, start date, and when it ends (never, on a date, or after a number of times).

### 8.3 Budgets (`/budgets`), Phase 3
Budget cards, each with the category chip (or "All spending"), the period, a `ProgressBar`, and "₱320 of ₱400 · ₱80 left".

### 8.4 Goals (`/goals`), Phase 3
Goal cards showing the name, the linked account, a progress bar, "₱2,400 of ₱5,000", the target date and "Save ₱433/month to reach it". A goal that's been reached gets an "Achieved" badge.

---

## 9. Screen states

| State | What the user sees |
|---|---|
| **Loading** | A skeleton, only if loading takes more than 150 ms |
| **No transactions yet** (dashboard) | Empty state: "No transactions yet. Press Ctrl+N to add your first one." with a **[+ Add transaction]** button. The stat cards show `₱0.00`. |
| **No transactions in this period** | "Nothing recorded for October 2026." plus **[Go to today]** |
| **No filter matches** | "No transactions match these filters." plus **[Clear filters]** |
| **Error loading data** | An inline message with a **Retry** button. Details are in the error log (ARCHITECTURE §4.4). |
| **App can't start** | A full-window error screen showing the database path and the error, plus **Open data folder** and **Check for updates** (ARCHITECTURE §6, UPD-6) |
| **Updating** | A centered glass panel with the logo and "Updating Jobert's Vault to 0.2.0…". The rest of the app isn't shown yet. |
| **Transparency off** (Windows setting) | The same screens with solid panels and no blur. The aurora and layout stay (DESIGN_SYSTEM §2.4). |

---

## 10. Keyboard shortcuts

| Shortcut | Action | Where |
|---|---|---|
| `Ctrl+N` | Add transaction (quick-add) | Everywhere |
| `Ctrl+F` | Focus the search box | Transactions |
| `Ctrl+1` … `Ctrl+4` | Dashboard, Transactions, Accounts, Reports | Everywhere |
| `Ctrl+,` | Settings | Everywhere |
| `Alt+←` / `Alt+→` | Previous / next period | Screens with a period switcher |
| `Alt+Home` | Go to today | Screens with a period switcher |
| `Alt+E` / `Alt+I` / `Alt+T` | Switch type to Expense / Income / Transfer | Quick-add |
| `Enter` / `Shift+Enter` | Save / save and add another | Quick-add |
| `↑` / `↓` | Previous / next day | Date input |
| `Esc` | Close a dialog or popover | Everywhere |

The Phase 4 polish adds a `?` shortcut that shows this list in a help dialog.

---

## 11. Open questions

None at the moment. Answered on 2026-10-26:

1. **First launch:** a plain setup form in Phase 1 (ONB-1), polished in Phase 4 (ONB-2).
2. **Duplicate:** the copy uses **today's date** (TXN-2).
3. **Pay bill:** suggests the **full amount owed**, and you can edit it (CC-9).
4. **Window:** a fixed **1440 × 900** that can't be resized or maximized, instead of remembering its size. It fits both of the owner's monitors (1920 × 1080 and 2560 × 1080) at 100% scaling, and falls back to 1280 × 800 if the screen is too small.
5. **Logo:** C · Line Door, shown in the sidebar and the title bar (DESIGN_SYSTEM §6.1).

Answered on 2026-11-01:

6. **Custom period:** ◀ ▶ move by the length of the range (F8).
7. **Fallback window:** the same layout zoomed to 88.9% (§1).
8. **Reordering:** drag or arrow keys, using `@dnd-kit` (§5.1).
9. **Last active records and unused records:** see F6 and F7.
10. **First launch:** closing the form early shows it again (F5).

Answered on 2026-11-06:

11. **Dashboard chart:** the small income-vs-expense chart under the stat cards is built in Phase 2 with RPT-1, and its data comes from `get_dashboard`, so the dashboard stays one call (§2).
