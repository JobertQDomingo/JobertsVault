# UI Flows: Jobert's Vault

| | |
|---|---|
| **Status** | Approved v1.3 |
| **Style** | Glassmorphism 2.0 ([DESIGN_SYSTEM.md](DESIGN_SYSTEM.md)) |
| **Last updated** | 2026-11-23 |
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
│ │ ¤ Debt-free (P4) │                                                                 │
│ │ ◷ Forecast  (P4) │                                                                 │
│ │ ⬡ Level     (P5) │                                                                 │
│ │ ──────────────── │                                                                 │
│ │ ⚙ Settings       │                                                                 │
│ ╰──────────────────╯                                                                 │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

- **Material:** every rounded box is a frosted glass panel (Level 1) floating **12 px** apart over the aurora. Dialogs, menus, the date picker and toasts are raised glass (Level 2). Text never sits straight on the aurora. See DESIGN_SYSTEM §2.
- **Sidebar:** a glass panel with the app mark and name at the top, the screens in the middle and Settings at the bottom. The current screen gets a soft indigo tint. Items from later phases are **hidden** until their phase is built. From Phase 5, a **level chip** sits to the right of the app name: the level's number in a small hexagon, inside a ring that fills as the next level's conditions are met (LVL-5, DESIGN_SYSTEM §9.2). Clicking it opens `/level`. While the status loads, the chip isn't shown.
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
| Stat cards | Income, expense and net cashflow for the period. Transfers aren't included. From Phase 2, each card shows its % change from the previous period, compared by the Settings choice (SET-7), and the dates compared with: `▲ 3.1% vs Oct 1–6`. The change is green when the direction is good (DESIGN_SYSTEM §7), or "—" when there's nothing to compare with. The net cashflow card is the only one tinted with the accent. | DSH-2, DSH-3 |
| Income vs expense (Phase 2) | A small version of the Reports chart (§8.1) for the period, under the stat cards. Its bars come with the rest of the dashboard from `get_dashboard`. | RPT-1 |
| Recent transactions | The newest transactions **regardless of the selected period**: the panel stretches to the right column's height and shows as many whole rows as fit, at least 5 and up to 20. Clicking a row opens it for editing. "View all" goes to `/transactions`. | DSH-8 |
| Accounts | Current balances (as of today), with the credit card summary and the net balance. Clicking an account opens its details. | DSH-4, DSH-5 |

**Added later:**
- **Phase 2:** a "No backup in the last 7 days." banner at the top with **Back up now** (BAK-7, F9)
- **Phase 3:**
  - a "Credit card payment due in 3 days: ₱640.00 owed" banner at the top (CC-7), with a **Pay bill** button that opens F2's transfer. Each active card due within 5 days with something owed gets its own banner, named after the card, soonest first. It says "due today" and "due tomorrow" for 0 and 1 days
  - a **Budgets** panel under Recent transactions, with **Manage** (to `/budgets`) in its header: one row per budget, with its chip and name, its period and a `ProgressBar` with the % used. Each bar is for that budget's own current week, month or year, whatever the period switcher shows, like the account balances (DSH-6). With no budgets, it says "No budgets yet."
  - a **Coming up** panel beside Budgets, with **View all** (to `/recurring`) in its header: every date an active recurring rule falls on from tomorrow to 7 days from today, soonest first (DSH-7). Each row has the category chip (or the transfer chip), the title ("Dining Out · Netflix", or "Bank → E-wallet"), "Fri, Nov 6 · Credit card · auto" (or "· needs confirming" for a Confirm rule; a transfer leaves out the account its title names), and the amount. A rule with several dates in the week has a row for each. Past 8 rows, it ends with "+ N more", linking to `/recurring`. With none, it says "Nothing in the next 7 days." Paused and ended rules have no dates, and one-off future-dated transactions stay under Upcoming on the Transactions screen (TXN-8)
  - the "2 recurring transactions are due." banner (F10)
- **Phase 4:**
  - **Loan banners** (LOAN-4) at the top, before every other banner. There's one per active loan whose next unpaid payment is due within 5 days or is overdue, soonest first (DATA_MODEL §5.11):
    - `warning`: "E-wallet loan: ₱1,000.00 due Fri, Nov 20" ("due today" and "due tomorrow" for 0 and 1 days)
    - `danger`, once it's overdue: "E-wallet loan: ₱1,000.00 overdue since Mon, Nov 16", plus "and 1 more overdue" when the loan has others
    - Each has a **Pay** button that opens Mark paid for that payment (F13). When its account is short, a second line says "E-wallet has ₱200.00. Move ₱800.00 there first."
  - a **Safe to spend** card at the top of the right column, above Accounts (SAFE-6). It's for today, like the balances, whatever the period switcher shows:
    - "Safe to spend until Sun, Nov 29" ("until today" when the window ends today), then the amount in `display` size
    - "₱560.00 a day · 15 days left"
    - "₱1,500.00 held back for Nov 30 – Dec 4", or, when even that isn't enough, ₱0.00 and "Short by ₱1,240.00 on Fri, Dec 4" in `--danger`, with an icon
    - from 3 days before a payday: "Payday Mon, Nov 30. Paid early? **Record now**" (PAY-4, F14)
    - **View forecast** in its header, to `/forecast`
    - With no payday rule: "Mark your pay as a payday to see what's safe to spend." with **Open Recurring**
  - **"Loans left: ₱14,000.00 · debt-free Aug 10, 2027"** under the net balance in Accounts, linking to `/debt-free`. When everything left is overdue, it says "Loans left: ₱1,000.00 · all overdue" in `--danger` instead of a date already passed, like the Debt-free screen (§8.5). It's hidden when nothing is left (LOAN-11).
- **Phase 5:**
  - the **focus rows** at the top of the Safe to spend card, above its title and divided from it by `--hairline`, shown only while a focus is active (CHK-8). There's one flat row per active focus, the month's first: "This week's focus: Keep Dining Out under ₱1,500.00 · ₱640.00 so far" with a `ProgressBar` under it, amber from 80% and red past the target (its state, DATA_MODEL §5.22). A focus that isn't a cap shows its own progress: "Stay under your ₱4,000.00 mid level · ₱2,310.00 this cycle", "Pay E-wallet loan by Fri, Nov 20 · not paid yet" (or "· paid Nov 18"), "Log on at least 5 days this week · 3 days so far". Clicking them opens `/level`. They're for today, like the balances, whatever the period switcher shows. They were planned as a strip, a glass panel of their own between the banners and the stat cards, but the dashboard is already past the blur budget, so they're flat in the card instead (§11 item 88, DESIGN_SYSTEM §11).
  - the dashboard is never shown while a check-in is due: the app opens on `/check-in` instead (§8.7).
  - **the banners share one panel:** every banner above sits in a single glass panel at the top, as flat tinted rows in the same order (loans, the backup warning, cards, then recurring dates due). The panel shows only while a banner does (item 100).

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
- **Future-dated** transactions (Phase 3, TXN-8) appear above "Today" under an **Upcoming** header, in muted text, grouped by day with no day net. While the period hasn't ended, Upcoming holds every later row that matches the other filters, even past the period's end, so a row dated next month can still be opened. The summary line leaves them out (DATA_MODEL §5.2).
- **Payslip rows** (Phase 4) are listed like any other, the gross as income and each deduction as an expense. Clicking one opens its payslip (F14), not the transaction form. A row that **pays a loan** opens the transaction form as usual, and its Delete confirm says "Delete this transaction? Its loan payment becomes unpaid again." Unpaid loan payments aren't transactions, so they never appear here.

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
| Date | **today** | ↑/↓ change it by one day. From Phase 3 it may be in the future, and the row waits under Upcoming until its date (TXN-8). |
| Payee | empty | Optional. From Phase 2, typing suggests up to 8 payees used before on this type, most recent first (TXN-6). ↓ / ↑ highlight one, and Enter or a click picks it and fills in its last category, unless that's archived now. Nothing is highlighted until ↓, so Enter still saves the payee as typed. |
| Method, Note | empty | Optional. |

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
| Opening balance | 0 or more. For a credit card the field holds the opening amount owed (DATA_MODEL §5.1): the new account form labels it **"Amount owed now"**, and the edit form labels it **"Opening amount owed"**, with the help text "Owed today: ₱640.00" (the card's `amountOwed`, CC-6) under it, as it was when the form opened. Changing the field moves the card's balance by the same amount. A card in credit owes ₱0.00. |
| *(Credit card only)* Credit limit, Statement closing day, Payment due day | Days are 1–31, with the help text "If a month is shorter, the last day is used." |
| *(Credit card only, Phase 4)* Spending levels: Soft, Mid, Hard | Three amounts on one row, ₱3,000.00, ₱4,000.00 and ₱5,000.00 to start with. Help text: "Warnings for each statement cycle's spending. They never block a purchase." (CC-10) |

### 5.3 Account details (`/accounts/:id`)

**Cash, bank or e-wallet:** the name, the current balance in `display` size, and that account's transactions, including incoming transfers (the same list and period switcher as `/transactions`, already filtered to the account). Edit and archive actions are in a ⋯ menu, which also has **Delete** if the account has never been used.

**Credit card** (CC-6):

```
┌─ 💳 Credit card ──────────────────────────────── [Pay bill] [⋯] ┐
│ ₱640.00 owed                                                    │
│ ████░░░░░░░░░░░░░░░░░░░░  6% of ₱10,000.00 limit                 │
│ Available credit   ₱9,360.00                                    │
│ This cycle         ₱412.30 spent   (Oct 21 – Nov 20)            │
│ (Phase 4) Spending level   Past your ₱3,000.00 soft level       │
│ Statement closes   Nov 20          Payment due   Nov 5          │
├─────────────────────────────────────────────────────────────────┤
│ (this card's transactions)                                      │
└─────────────────────────────────────────────────────────────────┘
```

---

## 6. Settings (`/settings`), Phase 1

| Tab | Contents | PRD |
|---|---|---|
| **General** | Currency symbol, its position (before or after the amount), decimal places (**locked** once any amount is saved, with an explanation), first day of week, month start day (1–28), **compare periods** (same days so far or full previous period, SET-7, Phase 2), show payment method field. **Updates:** the current version, **Check for updates** and a switch, "Install updates automatically when the app starts" | SET-1, SET-2, SET-6, SET-7, UPD-3 |
| **Categories** | Two lists (Income and Expense). Add, rename, change color (12 swatches) and icon (curated list), reorder, archive, unarchive, **merge**, delete if unused | CAT-2, CAT-4 |
| **Payment methods** | Add, rename, reorder, archive, unarchive, delete if unused | TXN-7 |
| **Backup** (Phase 2) | **Back up now** (BAK-1, F9). **Restore from backup…** (BAK-4). **Export to CSV**, with From and To dates, from the start of this month to today by default (BAK-6). The backup folder with **Change…**, and a switch for the daily automatic backup (SET-3). The date of the last backup, and the warning when it's more than 7 days old (BAK-7). | BAK-1 to BAK-4, BAK-6, BAK-7, SET-3 |
| **Appearance** (Phase 5) | **Theme:** System, Light or Dark (`?tab=appearance`). It changes the whole window at once, title bar included. A **Reduce transparency** switch | SET-5, SET-8 |

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

### F5: First launch (Phase 1, ONB-1)

```
Welcome to Jobert's Vault
  Step 1  Currency: [ ₱ ▾ ]  Position: [Before ▾]  Decimals: [2 ▾]
  Step 2  Your accounts: Cash (already added) · [+ Bank] [+ E-wallet] [+ Credit card]
          (name + opening balance; card asks for its limit, statement day, due day)
  Step 3  [Start using the app]
```

- The **Phase 1** version is a plain form, because the currency must be set before the first transaction. It stays a plain form: the polished version with a tour (ONB-2) was dropped on 2026-11-17 (PRD §14).
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
2. **Merge:** pick a category of the same kind → confirm: "Move 23 transactions from 'Snacks' to 'Dining Out' and delete 'Snacks' and any budget for it?" (CAT-4)
3. A category with no transactions can simply be **deleted**. The confirm says "Any budget for it is deleted too." (Phase 3)
4. Archived categories can be restored with **Unarchive**. The last active category of each kind can't be archived or deleted.

### F8: Look at a different period (Phase 1)
- Use the segmented control to switch between Day, Week, Month and Year. ◀ ▶ move one period back or forward, and **Today** jumps back to the current period. Custom opens a date range picker that can't choose future dates, and ◀ ▶ then move by the length of the range. ▶ is disabled once the next period would start after today.
- **Phase 2** adds a "Compare to: Same days so far ▾ / Full previous period" option in Reports' Period comparison section (RPT-4a, SET-7, §8.1).

### F9: Back up and restore (Phase 2)
- **Back up now:** Settings → Backup → **Back up now** → toast "Backup saved as jobertsvault-2026-10-26_0930.db" with an **Open folder** link, which opens the backup folder in Explorer.
- **Restore:** **Restore from backup…** → pick a file in the Windows open dialog → the app checks it first, and a file that isn't a Jobert's Vault backup, or one from a newer version, is refused with a toast saying why → confirm "This replaces all current data with the backup from Oct 20, 2026 9:30 AM. A safety copy of your current data will be saved first." (the time the file was last written) with a destructive **Restore** → every screen's data reloads, the app goes to the dashboard, and a "Backup restored" toast appears.
- **Export to CSV:** Settings → Backup → From and To (this month so far, from `get_period`) → **Export…** → the Windows save dialog, suggesting `jobertsvault-2026-11-01-to-2026-11-07.csv` → toast "Exported 15 transactions to <file name>".
- **Stale-backup warning:** if the last backup (manual or automatic) is more than 7 days old by local date, or there's none, a warning banner appears at the top of Settings → Backup ("Your last backup is more than 7 days old." or "You don't have a backup yet.") and at the top of the dashboard ("No backup in the last 7 days." with **Back up now**) (BAK-7).
- **Backup folder:** **Change…** opens the Windows folder picker. The app refuses a folder it can't write in, and **Use the default** goes back to `<app data>\backups`. New backups go to the folder; earlier ones stay where they are (SET-3).

### F10: Confirm recurring transactions (Phase 3)
1. An info banner on the dashboard says "2 recurring transactions are due." ("1 recurring transaction is due." for one) → **Review**. It shows while any date a Confirm rule queued is waiting (`recurringDue` from `get_dashboard`).
2. **Review** opens the "Recurring transactions due" dialog: one row per date, oldest first, with the category chip, "Category · Payee" (a transfer: "Bank → Card"), "Due Sat, Oct 24 · Bank" and the amount. Each row has **Post**, **Edit & post** and **Skip** (REC-3). One is settled at a time.
   - **Post** posts the rule's template on that date, with a "Posted" toast.
   - **Edit & post** opens the transaction form (§4) over the dialog, titled "Edit & post", filled in from the template with the due date. **Save** posts what's in it and closes it, with a "Posted" toast. It has no Save & add another, and it changes only that date, never the rule.
   - **Skip** settles the date with no transaction, with a "Skipped" toast. It never comes back.
3. The dialog closes by itself once nothing is left, and the banner goes with it. A failure is a toast, and the date stays on the list.
4. **Paydays (Phase 4):** a payday's row has **Record payslip** (F14) and **Skip**, in place of Post and Edit & post. After the due dates, the dialog lists each payday 1–3 days ahead as "Payday Mon, Nov 30 · Bank" with **Record now**, for pay that arrives early (PAY-4). The banner counts only the dates that are due, never the early ones.

### F12: Add a loan (Phase 4, LOAN-1, LOAN-10)
1. Debt-free → **+ Add loan** opens the "Add loan" dialog (640 px, since it holds a list):
   - **Name**, e.g. "E-wallet loan"
   - **Paid from:** an active cash, bank or e-wallet account
   - **Category:** Loan to start with
   - **This is:** [A new loan │ A loan I already have]. A new loan is selected to start with, so a new one always gets the check below.
   - **Amount borrowed:** optional. The help text says "Add it to see what the loan costs in interest."
   - **Date borrowed:** today to start with. It's required for a new loan and optional for one the user already has.
   - **Payments:** one row each, with **Date** and **Amount** and an ✕ to remove it. **+ Add payment**, or Enter in the last amount, adds a row with the previous row's amount, dated one more month after the first payment, with the day clamped to the month's end, so month ends don't drift (Jan 31, Feb 28, Mar 31). That's a form default from `lib/dates.ts` that the user can change; the backend checks every row. On a loan the user already has, a row dated before the day the loan is added (today, while it's being added) shows "Paid before".
   - **Total**, under the rows, as the user types, adds, edits and removes them: "Total ₱24,000.00". When any row counts as paid (a locked "Paid Nov 9" row, or one that shows "Paid before"), it reads "Total ₱24,000.00 · Still to pay ₱16,000.00", since what's still to pay is the figure a lender app shows. An empty amount counts as 0. It comes from the backend (`total_loan_payments`), and it's a check, never a rule: it never blocks Save (§11 item 95).
2. **Save:**
   - **A loan the user already has** is saved at once, with a "Loan added" toast.
   - **A new loan** first shows the "Before you add this loan" confirm (LOAN-10), from `preview_loan`: "Your debt-free date moves from Tue, Aug 10, 2027 to Fri, Oct 15, 2027.", "Safe to spend drops from ₱8,400.00 to ₱7,400.00.", and, when it would make a pay window short, a line such as "Nov 30 – Dec 4 would run short by ₱180.00." **Go back** returns to the form, and **Add this loan anyway** (destructive) saves it and restarts the loan-free streak.
3. **Edit** (the loan's ⋯ → **Edit**, or **Edit** in its payments dialog) is the same form, titled "Edit loan". Paid rows are locked and show "Paid Nov 9". "This is" can't change.

### F13: Pay a loan payment (Phase 4, LOAN-3, LOAN-4)
1. Click **Pay** on the dashboard's loan banner, or open the loan on Debt-free and click **Mark paid** on an unpaid row.
2. The transaction form (§4) opens, titled with the payment it pays, "Mark paid: E-wallet loan, due Fri, Nov 20", or "overdue since Mon, Nov 16" like its banner (plain "Mark paid" while the loans load), with the type fixed to Expense and these fields filled in: the scheduled amount, the loan's account and category, today's date, and the loan's name as the payee. Change the amount for a late fee. **Save** records the expense, with a "Marked paid" toast. The balances go down, and the row says "Paid Nov 20".
3. **To undo it**, open the paid row's ⋯ menu in the payments dialog and choose **Mark unpaid**. It confirms first: "Mark this payment unpaid? The expense recorded for it is deleted." Deleting that expense on the Transactions screen does the same.

### F14: Record a payslip (Phase 4, PAY-2, PAY-4)
1. **On payday:** the Due banner (F10) → **Review** → **Record payslip** on the payday's row. **Paid early:** **Record now** on the Safe to spend card, or on the payday's early row in the Due dialog, from 3 days before.
2. The "Payslip · Salary" dialog opens, filled in from that payday rule's last payslip, else the last Salary payslip, else with no gross and no lines the first time (`get_payslip_defaults`):
   - **Date:** the payday, or today when it's early
   - **Account**
   - **Gross pay**
   - **Deductions:** one row each, with a category (the deduction categories come first, then the other expense categories) and an amount, plus an ✕. **+ Add deduction** adds one.
   - **Note**
   - **Net pay**, shown under the rows. It comes from the backend as the user types (`preview_payslip`), and it's in `--danger` with "Deductions can't add up to the gross pay or more" when it isn't above 0. While Gross pay is empty it shows nothing, and no error.
3. **Save** records the gross as income and each deduction as an expense (DATA_MODEL §4.7), with a "Payslip saved" toast, and settles the payday.
4. **Editing:** clicking any of its rows on Transactions opens it again as "Edit payslip". **Delete** (bottom left) asks "Delete this payslip? Its gross pay and 4 deductions are deleted. Its payday won't be offered again, so to fix a mistake, edit the payslip instead.", since the payday stays settled (DATA_MODEL §4.7).

### F15: Before you borrow (Phase 4, LOAN-9)
1. On Debt-free, the **Before you borrow** panel asks for the **Payment** amount, **How many** payments and the **First payment** date. The payments are monthly, on the first payment's day of the month.
2. **Try it** shows the result from `preview_loan`, and nothing is saved:
   - "Debt-free: Aug 10, 2027 → Oct 15, 2027 (66 days later)"
   - "Safe to spend now: ₱8,400.00 → ₱7,400.00"
   - each pay window it changes, with what's left before and after ("Dec 15–29: ₱18,300.00 → ₱17,300.00"), and any window it would make short in `--danger`

### F16: Spend on a card past a level (Phase 4, CC-10, CC-11)
1. The user saves an expense on a card, dated in its current cycle. The form asks `preview_card_spend` first.
2. **Hard level:** a confirm, "Spend anyway?", says "This takes Credit card to ₱5,120.00 this cycle, past your ₱5,000.00 hard level." ("reached your" when the spending is exactly the level) with **Go back** and **Spend anyway** (destructive). Go back returns to the form.
3. **Soft or mid level, reached by this expense:** it saves, and a warning toast says "Credit card: ₱3,240.00 this cycle, past your ₱3,000.00 soft level." (exactly ₱3,000.00: "reached your ₱3,000.00 soft level")
4. Below the soft level, or at a soft or mid level the card had already reached: it saves as usual. The hard level asks every time, even when the card is already past it.
5. The card's details show the level under "This cycle" (soft in the muted text color, mid in `--warning` and hard in `--danger`, each with the level's name, and an icon from mid up), and so do its row in the Accounts list and the dashboard's card summary.

### F17: Finish a check-in (Phase 5, CHK-1 to CHK-9)
1. **It appears by itself.** On Monday (or the first day of a new month), the app opens on the check-in (§8.7) instead of the dashboard. It also appears when the date changes while the app is open: the next time the window gets focus, or at midnight. There's nothing to click to start it, and no way to close or postpone it.
2. **Go through the steps** with **Next** (Enter) and **Back**. Each step opens with one sentence that sums it up, with the figures below.
3. **Log something on the way:** **+ Add** (Ctrl+N) opens quick-add over the check-in. After Save, the step's figures update. This is how a balance that doesn't match gets fixed in the Balance check step.
4. **Act on what's due:** **Pay bill** in the Card step, and **Pay** and **Record now** in the Loans and bills step, open the same forms as the dashboard's banners (F2, F13, F14), over the check-in (CHK-9).
5. **Pick a focus** in the last step. Nothing is preselected, and **Finish** is disabled until one is chosen.
6. **Finish** saves the check-in and opens the dashboard, with a "Check-in done" toast. If an account was left as **Doesn't match**, it opens that account's details instead (the first such account in the step's order), with the toast "Check-in done. Check this account against your real balance." Finish sets the check-in query to none before it moves, so the guard doesn't send it back. **If the save fails,** the check-in reloads and stays on its last step, and the error toast offers **Open the app anyway**: the only way past a due check-in, for this session only (ADR 0014).
7. **Closing the app before Finish** saves nothing, so the check-in starts again from its first step at the next start.

### F18: A save reaches the focus (Phase 5, CHK-8)
1. A category cap or spending cap is active, and the user saves an expense that counts toward it.
2. **It reaches 80% of the target:** it saves, and a warning toast says "Dining Out focus: ₱1,240.00 of ₱1,500.00."
3. **It goes past the target:** it saves, and the toast says "Dining Out focus: ₱1,720.00, past your ₱1,500.00 target." A spending cap says "Spending focus: …".
4. The toast shows once for each step up (to near, then to over), never on every save, and never for a save that doesn't raise the focus's state. Nothing is asked and nothing is blocked. A card-level focus has no toast of its own: the card's spending levels already warn (F16).

### F11: Update the app (Phase 1)
- **Automatic (the default):** at startup, the app checks for a newer version. "Checking for updates…" appears only if the check takes more than 150 ms. If there's a newer version, "Updating Jobert's Vault to 0.2.0…" appears, and the app closes and reopens on it. There's nothing to click (UPD-1).
- **No internet, or the check fails:** the app starts normally and shows no error (UPD-2).
- **By hand:** Settings → General → **Check for updates** says "You're up to date." or "0.2.0 is out." with an **Update now** button. It restarts the app, so anything unsaved in an open form is lost: the button is in Settings, where no form is open (UPD-3).
- **Automatic updates off:** the app doesn't connect by itself. **Check for updates** still works (UPD-3).
- **If an update is refused or fails:** the message says why and adds "Nothing was changed." (UPD-4).

---

## 8. Phase 2–5 screens (outline)

### 8.1 Reports (`/reports`), Phase 2
- Period switcher plus an **account filter** (RPT-5): All accounts, or one account, with archived ones offered only when already chosen (DESIGN_SYSTEM §9.1). It's kept in the URL (`account=2`), and every section then counts only that account's own rows; a transfer into it counts nowhere (DATA_MODEL §5.2).
- **Compare to** (Same days so far / Full previous period) sits in the Period comparison section's header, the only report it changes. It starts on the Settings choice (SET-7), changes only this view and is kept in the URL (`compare=full`). It's disabled, with a tooltip, and shows Full previous period when `get_comparison` says the period is over, since that's always compared in full (RPT-4a).
- **Section 1: Income vs expense.** A bar chart grouped by day, week or month (RPT-1, DATA_MODEL §5.9), with net cashflow as a line. Income bars rise above the zero line and expense bars hang below it, so they're told apart by direction as well as color. Hovering a bar shows its dates and the three figures. A hidden table gives screen readers the same figures.
- **Section 2: Category breakdown.** A donut chart showing the largest 8 categories plus "Other", with the total in its middle, next to a ranked list showing each category's chip, % and amount (DATA_MODEL §5.10). Hovering a slice shows its name, amount and %. An **Expense / Income** switch in the section's header picks the kind, kept in the URL (`breakdown=income`) so Back returns to it. Clicking a category in the list opens `/transactions` filtered to that category, the same period and the same account (RPT-2, RPT-3, RPT-5). Archived categories with spending in the period still appear.
- **Section 3: Period comparison.** Rows for income, expense, net and each category, with columns for this period, the comparison period, the difference and the % change (RPT-4). The column headers are the dates counted, e.g. "Nov 1–6" and "Oct 1–6" (the year appears when it isn't this year). The % change is `▲ 12.3%` or `▼ 4.0%`, green when the direction is good (DESIGN_SYSTEM §7), or "—" when there's nothing to compare with. Income and expense amounts are plain, and the difference is signed, in the plain text color.

### 8.2 Recurring (`/recurring`), Phase 3
A list of rules, oldest first, in one panel with **+ New rule** in its header. Each row shows:
- the category chip (⇄ for a transfer) and "Category · Payee" ("Bank → Card" for a transfer)
- the schedule in words, from the start date, and the next date: "Monthly on the 25th · next Nov 25", with the year when it isn't this year, or "· ended" once the rule has ended
- the amount, and a badge: **Auto** or **Confirm**, or **Paused** in place of either. A paused rule shows no next date, because resuming moves it (DATA_MODEL §4.1).

Clicking a row edits the rule. Its ⋯ menu has **Edit**, **Pause** or **Resume**, and **Delete**, which asks first: "Delete this rule? The transactions it already created stay." With no rules, an empty state offers **New rule**.

**+ New rule** opens the transaction form (§4), titled "New rule", with a **Schedule** section under it:
- **Repeats:** Daily, Weekly, Monthly or Yearly. Monthly to start with.
- **Every** *N*, with its unit ("1 month", "2 weeks"). 1 to start with.
- **Starts:** today to start with. It may be in the past or the future, and a past start fills in the dates since then.
- **Ends:** Never, On a date, or Number of times. The last two add an "Ends on" date or an "Ends after *N* times" field.
- **Mode:** Auto-post (the default) or Confirm (REC-3).

The rule form has no Save & add another. Saving a rule posts or queues its dates up to today at once (ARCHITECTURE §8).

**Editing** a rule ("Edit rule") keeps its start date, so changing the amount, category, mode or end keeps the rule's place. Changing **Repeats** or **Every** starts it again, so the form offers the rule's next date as the new start, unless another start was already chosen (DATA_MODEL §4.1).

**Paydays (Phase 4, PAY-1):** an Income rule's form has a **This is a payday** switch under Mode. Turning it on sets Mode to Confirm and disables the Mode choice. The amount's label becomes "Expected net pay", with the help text "Used until 3 payslips are recorded. After that, the lowest of the last 3 counts." In the list, a payday rule has a **Payday** badge in place of Confirm. For pay on the 15th and the 30th, the user makes two monthly rules, one starting on each day.

### 8.3 Budgets (`/budgets`), Phase 3
One panel, with **+ New budget** in its header, holding a grid of flat budget cards (three across), all spending first, then in the categories' order. Each card has the category chip (or "All spending"), the period, a `ProgressBar` and "₱320.00 of ₱400.00 · ₱80.00 left". Past its limit, the card says "₱180.00 of ₱150.00 · ₱30.00 over" in `--danger`. The bar turns amber at 80% and red at 100% (BUD-3). The cards are flat tints inside the one panel, so any number of budgets stays within the blur budget (DESIGN_SYSTEM §2.3).

Clicking a card edits it. Its ⋯ menu has **Edit** and **Delete**, which asks first: "Delete this budget? Its transactions stay. This can't be undone." With no budgets, an empty state offers **New budget**.

**+ New budget** opens a small dialog, "New budget" (or "Edit budget"):
- **Category:** All spending (to start with), or an active expense category. An edit keeps its own category even if it's been archived.
- **Period:** Weekly, Monthly or Yearly. Monthly to start with.
- **Amount:** the limit for each period. A note under it says each period starts again from the full amount (BUD-4).

A category, or all spending, has one budget per period. A second one shows the backend's message under the field it names.

### 8.4 Goals (`/goals`), Phase 3
One panel, with **+ New goal** in its header, holding a grid of flat goal cards (three across), in the order they were created. Each card shows the name, the linked account and target date ("Bank · by May 5, 2027"), a `ProgressBar` with the % done, "₱2,400.00 of ₱5,000.00 · ₱2,600.00 left", and "Save ₱433.34/month to reach it on time" (GOL-3), what its own target date needs (DATA_MODEL §5.7). Once the target date is today or has passed, that line says "Target date passed" instead, and a goal with no target date has no line. A goal that's been reached gets an **Achieved** badge in `--success`, its bar turns green, and it asks for no monthly amount, even if the balance drops later (GOL-4). A goal's bar has no amber or red thresholds.

Clicking a card edits it. Its ⋯ menu has **Edit**, **Archive** and **Delete** ("Delete this goal? Its account and transactions stay. This can't be undone."). Any goal can be archived, achieved or not. Archived goals fold under **Show archived (n)** below the grid, as the same cards, whose ⋯ has **Unarchive** and **Delete**. With no active goals, an empty state offers **New goal**: "No goals yet.", or "No active goals." when some are archived.

**+ New goal** opens a small dialog, "New goal" (or "Edit goal"):
- **Name:** 1–50 characters, unique.
- **Target:** the amount to reach.
- **Target date:** required from Phase 4 (optional before). The card says how much to save each month.
  - **Phase 4 (GOL-5, GOL-6):** once the target and account are chosen, a line under the date says "The most you could set aside is about ₱650.00 a month. The earliest realistic date is Mar 5, 2027." (`suggest_goal`, asked as the user types, like the payslip's net). That's the monthly room, the most there is for this goal, not what the chosen date needs: the card shows that figure, which on the day a date is set is never more than the room (§11 item 96). On a new goal the date starts on that suggestion. A missing or sooner date is refused when saving, with the backend's message under the field. With no payday rule, the line says "Mark your pay as a payday first, so the app can suggest a realistic date." with **Open Recurring**, and saving a new goal (or a new date) is refused with the same message. A monthly room of 0 says "You can't set anything aside each month right now, so no date is realistic yet."
- **Account:** an active account that isn't a credit card. An edit keeps its own account even if it's been archived. A note says the account's balance is the goal's progress (GOL-2).

The backend's message shows under the field it names.

**Phase 4 (GOL-7):** when a save moves a goal's date, a toast says "Emergency fund now aims for Jul 5, 2027 (was May 5, 2027), so it stays realistic." (one line per goal moved). A goal that's behind says "Behind: nothing can be set aside right now" in `--warning` on its card in place of the monthly amount, or "Behind: the earliest realistic date is now Jul 5, 2027" when there is room.

### 8.5 Debt-free (`/debt-free`), Phase 4

```
┌─ Debt-free ─────────────────────────────────────────────────────────────────────┐
│ Debt-free on Tue, Aug 10, 2027                                 270 days to go   │
│ ₱14,000.00 left · ₱4,000.00 paid   █████░░░░░░░░░░░░░░░░░ 22% paid               │
│ 52 days since your last new loan · ₱3,000.00 in interest on 2 loans             │
└─────────────────────────────────────────────────────────────────────────────────┘
┌─ Loans ──────────────────────────────── [+ Add loan] ┐ ┌─ Before you borrow ─────┐
│ ┌─ E-wallet loan ────────┐ ┌─ Bank loan ───────────┐ │ │ Payment   [ ₱1,000.00 ] │
│ │ E-wallet · 3 of 8 paid │ │ Bank · 1 of 10 paid   │ │ │ How many  [ 6 ]         │
│ │ ███████░░░░░░ 37%      │ │ █░░░░░░░░░░░░ 10%     │ │ │ First     [ 12/18/2026 ]│
│ │ ₱5,000.00 left         │ │ ₱9,000.00 left        │ │ │           [ Try it ]    │
│ │ Next ₱1,000.00 Nov 20  │ │ Next ₱1,000.00 Dec 10 │ │ │ (the result, F15)       │
│ └────────────────────────┘ └───────────────────────┘ │ └─────────────────────────┘
│ ▸ Show archived (1)                                  │
└──────────────────────────────────────────────────────┘
┌─ Money freed up ────────────────────────────────────────────────────────────────┐
│ November 2026     ₱2,000.00 to loans                                            │
│ April 2027        ₱1,000.00 to loans    ₱1,000.00 less than this month          │
│ September 2027    ₱0.00                 ₱2,000.00 less than this month          │
└─────────────────────────────────────────────────────────────────────────────────┘
```

- **The top panel** (LOAN-5 to LOAN-8) shows the debt-free date and days to go, the total left and paid with a `ProgressBar` of the share paid, the loan-free streak ("52 days since your last new loan", hidden when no loan has a date borrowed) and the total interest (hidden when no loan has an amount borrowed). With everything paid, it says "You're debt-free." in `--success`. When everything left is overdue, it says "₱2,000.00 overdue. Pay it to be debt-free." in `--danger`, with an icon, in place of the countdown (DATA_MODEL §5.11).
- **Loans:** one flat card per active loan, two across, in the order they were added. Each card has the name, "E-wallet · 3 of 8 paid", a `ProgressBar` of % paid, "₱5,000.00 left", the interest when it has an amount borrowed, and the next payment. An overdue next payment says "Overdue since Mon, Nov 16" in `--danger`, with an icon. A paid-off loan gets a **Paid off** badge in `--success`. The cards are flat tints inside the one panel, as on Budgets (DESIGN_SYSTEM §2.3). Archived loans fold under **Show archived (n)**.
- **Clicking a card** opens its payments dialog (640 px), titled with the loan's name, with **Edit** in its header. It has one row per payment, oldest first: the date, the amount, and "Unpaid" with **Mark paid** (F13), "Overdue" in `--danger` with **Mark paid**, "Paid Nov 20" with a ⋯ menu holding **Mark unpaid**, or "Paid before".
- **A card's ⋯ menu** has **Edit**, plus **Archive** once it's paid off, and **Delete** when none of its payments was marked paid: "Delete this loan? Its payments are deleted. This can't be undone." An archived card's ⋯ has **Unarchive**.
- **Before you borrow** (LOAN-9) is F15.
- **Money freed up** (LOAN-7) has one row per month, from this month to the month after the debt-free date: what goes to loans, and how that compares with this month. The list scrolls inside its panel.
- With no loans, the screen is one empty state: "No loans. Keep it that way." with **Add loan**.

### 8.6 Forecast (`/forecast`), Phase 4

```
┌─ Safe to spend ─────────────────────────────────────────────────────────────────┐
│ ₱8,400.00 until Sun, Nov 29 · ₱560.00 a day                                     │
│ ₱1,500.00 held back for Nov 30 – Dec 4                                          │
└─────────────────────────────────────────────────────────────────────────────────┘
╭─ [ Pay windows │ Months ] ─╮
┌─────────────────────────────────────────────────────────────────────────────────┐
│ Window            Start         Pay in        Bills          Left               │
│ Nov 15–29         ₱12,500.00    —             ₱2,600.00      ₱9,900.00          │
│                   ₱10,500.00 received Nov 15                                    │
│ Nov 30 – Dec 4    ₱9,900.00     ₱10,500.00    ₱12,000.00     ₱8,400.00  lowest  │
│ ▾ Dec 5–14        ₱8,400.00     ₱6,000.00     ₱4,000.00      ₱10,400.00         │
│     Sat, Dec 5    Bonus (expected)                   Bank      +₱6,000.00       │
│     Thu, Dec 10   Insurance · yearly                 Bank      −₱4,000.00       │
│ Dec 15–29         ₱10,400.00    ₱10,500.00    ₱2,600.00      ₱18,300.00         │
│ …                                                                               │
└─────────────────────────────────────────────────────────────────────────────────┘
```

Today in this sketch is Sun, Nov 15, a payday whose payslip is already recorded. Spending the ₱9,900.00 left now would leave Nov 30 – Dec 4 ₱1,500.00 short, so ₱1,500.00 is held back and ₱8,400.00 is safe to spend.

- **The top panel** is the dashboard's Safe to spend card at full width (SAFE-5, SAFE-6): the amount, the amount per day, and what's held back and for which window, or "Short by ₱X on *date*" in `--danger`. When a payday is 1–3 days away, it also has "Paid early? **Record now**" (F14).
- **The toolbar's segmented control** switches between **Pay windows** and **Months**, kept in the URL (`?view=months`) (SAFE-7).
- **The table** has one row per pay window, or per month, over the next 12 months. Each row shows its dates (the current one starting today), what it starts with, the pay and other money coming in, the bills, and what's left at its end if nothing else is spent (DATA_MODEL §5.15). A payslip already recorded shows as "₱10,500.00 received Nov 15" in muted text, by its own date, without being added again. The window money is held back for is marked **lowest** (in the Months view, the month holding that window's lowest day). A window that would run short on any day has a triangle icon and its Left in `--danger` with "Short".
- **Clicking a row** opens it in place to list its movements, by date: each loan payment (by loan name), card bill ("Credit card payment"), recurring date ("Utilities · Water bill"), future-dated row, and payday ("Salary (expected)"), with its account and a signed amount. Only one row is open at a time.
- **With no payday rule,** the screen is one empty state: "Mark your pay as a payday to see what's safe to spend." with **Open Recurring**.

### 8.7 Check-in (`/check-in`), Phase 5

A full-window screen with **no sidebar**, shown while a check-in is due (CHK-2). It's a route, like `/welcome`, not a dialog, so quick-add and the pay forms open over it as ordinary dialogs.

```
┌─ window: aurora background ──────────────────────────────────────────────────────────┐
│ ╭──────────────────────────────────────────────────────────────────────────────────╮ │
│ │ ◆ Weekly check-in · Nov 9–15                                    [+ Add  Ctrl+N]  │ │
│ ╰──────────────────────────────────────────────────────────────────────────────────╯ │
│         ╭────────────────────────────────────────────────────────────────╮           │
│         │  ●───●───◉───○───○───○     Step 3 of 6 · Card                  │           │
│         │                                                                │           │
│         │  You're past your mid level this cycle.                        │           │
│         │                                                                │           │
│         │  ┌─ Credit card ────────────────────────────────────────────┐  │           │
│         │  │ ₱4,310.00 this cycle          ▲ Mid level                │  │           │
│         │  │ Soft ₱3,000.00 · Mid ₱4,000.00 · Hard ₱5,000.00          │  │           │
│         │  │ ₱6,120.00 owed · 61% of your limit · due Sat, Dec 5      │  │           │
│         │  │                                             [Pay bill]   │  │           │
│         │  └──────────────────────────────────────────────────────────┘  │           │
│         │                                                                │           │
│         │  [Back]                                              [Next ⏎]  │           │
│         ╰────────────────────────────────────────────────────────────────╯           │
└──────────────────────────────────────────────────────────────────────────────────────┘
```

- **Top bar:** the logo, the title and the period ("Weekly check-in · Nov 9–15", "Monthly check-in · October 2026"), and **+ Add**. There's no close button, and Esc does nothing here. A merged check-in is titled as the monthly one.
- **The panel** is one Level 1 glass panel, 880 px wide and centered, with the `StepIndicator` (DESIGN_SYSTEM §9.2) at the top, the step's sentence in `h1` size, its figures in flat `--wash` cards with 12 px corners inside the panel, and **Back** (hidden on the first step, `secondary`) and **Next** (`primary`, with its `Enter` hint like + Add's) at the bottom. The panel runs from 12 px under the top bar to 12 px above the window's bottom edge, with 24 px padding and 20 px between its parts, so Back and Next stay in the same place on every step, and figures that don't fit scroll inside it. The last step's button is **Finish**. Enter presses Next, and the step is kept in the URL by name (`?step=card`), so Back and a reload stay put, and a step that appears after a save (Goals, when a goal moves) doesn't move the user.
- **The steps**, in order (CHK-3). A weekly check-in has 6 to 8 (with a month start day above 1, the first one finished in January can be weekly and still have Your year), and a monthly one 8 or 9:

| Step | When | What it shows |
|---|---|---|
| **Start** | Always | The win, when there is one: "Dining Out fell by ₱640.00 from the week before." A personal best under it: "Your lowest-spending week of the last 12." Then the last focus's result: "Last week's focus: keep Dining Out under ₱1,500.00. You spent ₱1,720.00, past your ₱1,500.00 target." (or "You kept it: ₱1,180.00."), with a check icon in `--success` or an x icon in `--danger`. With no win and no focus, it says "Here's how last week went." |
| **Money in and out** | Always | The three stat cards (income, expense, net) with their % change from the period before, the 3 largest expense categories with each one's change, and the budgets for that period as "kept" or "₱30.00 over" |
| **Card** | Always | The sentence names the highest level reached this cycle: "You're past your mid level this cycle." ("You've reached your mid level this cycle." at exactly the level), or "You're under your soft level this cycle." With several cards it names the one furthest along: "Credit card is past its mid level this cycle." Then each active card in a flat card: its name with the `credit-card` icon, this cycle's spending in `display` size with its `SpendLevelTag` at the right, the three levels ("Soft ₱3,000.00 · Mid ₱4,000.00 · Hard ₱5,000.00"), and "₱6,120.00 owed · 61% of your limit · due Sat, Dec 5", with **Pay bill** (`secondary`, since Next is the step's main button) when something is owed (CHK-9). With no active card: "No credit card." |
| **Loans and bills** | Always | What was paid: "2 loan payments paid, ₱2,000.00." Each payment the dashboard's loan banner would show (due within 5 days, or overdue in `--danger`), with **Pay**. One paid from here stays in its place as "Bank loan: ₱1,050.00 paid today" (§11 item 91). "Loans left: ₱14,000.00 · debt-free Aug 10, 2027", and "It moved from Jul 10, 2027." when it changed since the last check-in. Then **Coming up**: the next payday, with **Record now** when it's due ("Payday Sun, Nov 15", "Payday today") or 1–3 days away ("Payday Wed, Nov 18. Paid early?"), the Safe to spend line as on the dashboard card, and the next 3 pay windows as `ForecastRow`s, a short one marked. With no payday rule: "Mark your pay as a payday to see what's coming." |
| **Goals** | A monthly check-in with at least one active goal. A weekly one only when a goal moved or is Behind | Each active goal's `ProgressBar`, with "Now aims for Jul 5, 2027 (was May 5, 2027)" on one that moved by itself, and its Behind line (§8.4) |
| **Balance check** | A monthly check-in | "Do these match your real accounts?" One row per account with its balance and a segmented control, **Matches** / **Doesn't match**, with neither chosen to start with. **Next** is disabled until every row has an answer. A row set to Doesn't match shows "Add what's missing with + Add. The balance updates here." (CHK-4) |
| **Your year** | The first check-in finished in January | "2026: you earned ₱310,400.00 and spent ₱288,150.00." with the three figures against the year before, the 3 largest categories, the yearly budgets, the loans paid off, and "You started the year at level 2 and ended it at level 5." (CHK-5) |
| **Stats and level** | Always | The `StatHexagon` with the six scores beside it, each with its change since the last check-in. The `LevelBadge`: "Level 4 · In the black". When it rose, the badge plays the level-up moment and the sentence is "You reached level 4." When it fell: "Level 2 now: a loan payment is overdue." When it's the same: "You're still at level 4.", or "You're at level 4." on the first check-in, which has no earlier one to compare with. Under it, the next best move (as on the Level screen, §8.8) and the two streaks in `small` text: "4 check-ins in a row on time · 2 focuses kept in a row" (LVL-4, LVL-6, LVL-7) |
| **Pick a focus** | Always | "Pick one focus for this week." ("for this month." in a monthly or merged one). Up to 3 focus offer cards, one per offer, each with a sentence and, under it, what it's based on: "Keep Dining Out under ₱1,500.00" / "That's what you spent the week before", "Spend under ₱3,920.00 this week" / "Your Safe to spend, ₱560.00 a day", or, once something is spent that week, "Spend under ₱2,980.00 this week" / "₱1,300.00 spent so far, then your Safe to spend, ₱560.00 a day". The other kinds name the stat they're offered for: "Stay under your ₱5,000.00 hard level this cycle" / "Helps your Card stat", "Pay E-wallet loan by Fri, Nov 20" / "Helps your Bills stat", "Log on at least 5 days this week" / "Helps your Habit stat". Clicking a card selects it, and ↑ ↓ move between them. **Finish** is disabled until one is selected (CHK-7) |

- **A step with nothing to show** still appears, with one line saying so, except Goals, which is left out with no active goal (item 93). Only Goals can come and go, after a save.
- **The forms** opened by + Add, Pay bill, Pay and Record now are the usual Level 2 dialogs over the screen. When one saves, the check-in's figures reload in place and the step stays where it is.
- **Loading** is a skeleton of the panel. **An error** loading the names it shows (categories, accounts, loans) is the inline message with Retry (§9). The guard never traps the user behind a failed load: if `get_checkin` fails, the app opens normally and tries again on the next window focus or at midnight. A reload that fails while a check-in is already on screen keeps it there, as it was. Until its first answer, a guarded route shows nothing, so the dashboard never flashes before the check-in.
- **Finish** is F17. While it saves, a second press does nothing, and Finish keeps the keyboard focus. A failed save is an error toast, bottom centre so it never covers Finish, and the check-in stays open on its last step. It reloads at once, and a picked focus that's no longer offered is cleared, so a Finish refused because the day changed is fixed by picking again. The toast offers **Open the app anyway**, which lets the user past the check-in for this session only: nothing is saved, and it's back at the next start (ADR 0014).

### 8.8 Level (`/level`), Phase 5

```
┌─ Vault level ───────────────────────────────────────────────────────────────────────┐
│  ⬢ 4   Level 4 · In the black                                                       │
│  ●───●───●───◉───○───○───○───○───○───○                                              │
│  1   2   3   4   5   6   7   8   9   10                                             │
│  Next: Level 5 · Steady                                                             │
│   ○ Earn more than you spend 3 months in a row · 1 of 3 so far                      │
│   ✓ No pay window runs short in the next 12 months                                  │
│  Next best move: keep November's spending under your income.                        │
└─────────────────────────────────────────────────────────────────────────────────────┘
┌─ Stats ─────────────────────────────────────┐ ┌─ Focus ─────────────────────────────┐
│        Cashflow                             │ │ This week                           │
│     Habit  ╱▔▔▔╲  Bills    Cashflow 72 ▲ 6  │ │ Keep Dining Out under ₱1,500.00     │
│           ▕ ⬡  ▏           Bills   100 —    │ │ ████████░░░░░░ ₱640.00 so far       │
│   Savings  ╲▁▁▁╱  Card     Card     48 ▼ 10 │ │                                     │
│          Debt              Debt     40 ▲ 3  │ │ Last 8: ✓ ✓ ✕ ✓ ✓ ✓ ✕ ✓             │
│                            Savings  22 ▲ 2  │ │ 2 focuses kept in a row             │
│                            Habit    90 ▲ 5  │ │ 4 check-ins in a row on time        │
│ Weakest: Savings. Your money covers less    │ │ 52 days since your last new loan    │
│ than a month of spending.                   │ │                                     │
└─────────────────────────────────────────────┘ └─────────────────────────────────────┘
```

- **The top bar** says "Vault level", the screen's name (PRD §14). The sidebar item is "Level".
- **The top panel** has the `LevelBadge` and name, then the `LevelPath` (DESIGN_SYSTEM §9.2): ten nodes in a row, the finished ones filled with a check, the current one ringed, the later ones hollow with a lock. Hovering or focusing a node shows that level's name and conditions. Under it, "Next: Level 8 · Debt-free" in `h3`, and the next level's conditions as a checklist, each with `check` in `--success` when it's met or `circle` in `--muted-foreground` when it isn't, and its gap in `--muted-foreground` where there is one: "Pay off every loan · ₱8,000.00 left", "90 days without a new loan · 30 days to go". Then the **next best move** (LVL-2 to LVL-4): "Next best move: pay off your loans, ₱8,000.00 left." At level 10 it says "You're at the top level." and the move names the lowest stat. At level 0 the badge reads "Level 0" with no name.
- **Stats:** the `StatHexagon`, with last check-in's shape as an outline behind it, and the six scores as a list: the name, the score, and `▲ 6`, `▼ 10` or `—` against the last check-in, colored by direction with the arrow, never by color alone. A stat that doesn't apply shows "—" for its score and has no corner. From 3 check-ins on, each row ends with a small trend line of its last 12 snapshots (LVL-8): 64 × 20 px, a 1.5 px `--primary` line with no axes or dots, scaled to its own range but never less than 20 points tall, so a flat line sits in the middle. Under the chart, one sentence gives the lowest stat's reason: "Weakest: Debt. Loan payments are due this month." Before the first check-in there's no outline and no change column.
- **Focus:** each active focus as on the dashboard (§2), flat, with a cap's `ProgressBar` under its sentence. With none: "No focus right now. You'll pick one at your next check-in." Then the **focus record**, "Last 8" and the last 8 graded focuses as 16 px icons 4 px apart, `check` in `--success` and `x` in `--danger`, oldest first, each with a tooltip naming the focus and the result, and the three streaks, one per line: "2 focuses kept in a row", "3 check-ins in a row on time", and the loan-free streak (LOAN-8), "122 days since your last new loan", hidden when it has none.
- **The wording for the other condition, move and reason codes** (DATA_MODEL §5.20, §5.21) follows the same pattern: a condition as the thing to do, its gap after "·"; a move as "Next best move:" and the action; a reason as one plain sentence after "Weakest: <stat>.". All of it is in `src/lib/vault.ts`, one place for both screens. The checklist was written with the Level screen, and the moves, the reasons and why a level fell with the check-in:
  - **Checklist** (each path node's tooltip lists its level's conditions the same way, without gaps): Log on 20 of the last 30 days · 6 days more to log; Mark your pay as a payday; Set a budget; Nothing overdue; No loan payment paid late in the last 30 days; Earn more than you spent last month; Earn more than you spend 3 months in a row · 1 of 3 so far; No pay window runs short in the next 12 months; Every card under its mid level for 3 cycles · Credit card is ₱300.00 over now; Two weeks of spending saved · ₱X to go; Pay off every loan · ₱8,000.00 left; 90 days without a new loan · 30 days to go; A month of spending saved · ₱X to go; Every stat at 80 or more · the lowest is 62; No goal behind; A year without a new loan · 120 days to go. A met condition shows no gap, and neither does one with no figure yet.
  - **Moves:** "pay the overdue E-wallet loan payment, ₱1,000.00", "pay Credit card's overdue bill, ₱840.29", and for each condition: log on 6 days more · mark your pay as a payday · set a budget · pay what's overdue · pay every loan payment on time for 30 days · keep this month's spending under your income · earn more than you spend 3 months in a row, 1 of 3 so far · keep every pay window from running short · keep Credit card under its mid level, ₱300.00 over now · save ₱X more to cover two weeks of spending · pay off your loans, ₱8,000.00 left · go 90 days without a new loan, 30 days to go · save ₱X more to cover a month of spending · get every stat to 80, the lowest is 62 · catch up on the goal that's behind · go a year without a new loan, 120 days to go. A gap that's none leaves its figure out. At level 10 the move is "You're at the top level." and the weakest stat's reason.
  - **Reasons:** no take-home pay in the last 3 months · you spent more than you took home · little of your take-home pay is left after spending · a loan payment is overdue · a card bill is overdue · a loan payment was paid late · a card is past its hard, mid or soft level · a card uses much of its limit · loan payments are due this month · something is left on your loans · a goal is behind · your money covers less than 3 months of spending · you logged on few of the last 30 days · some check-ins weren't finished on time.
  - **Why a level fell** ("Level 2 now: …"): the first unmet condition, as a fact: "you logged on fewer than 20 of the last 30 days", "there's no payday rule", "there's no budget", "a payment is overdue", "a loan payment was paid late in the last 30 days", "last month you spent more than you earned", "you didn't earn more than you spent 3 months in a row", "a pay window runs short", "a card went past its mid level", "your money covers less than two weeks of spending", "a loan isn't paid off", "you took a new loan in the last 90 days", "your money covers less than a month of spending", "a stat is below 80", "a goal is behind", "you took a new loan in the last year".
- Nothing on this screen can be edited. It has no period switcher: everything is for today.
- The screen has 3 glass panels, and everything inside them is flat (DESIGN_SYSTEM §2.3).

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
| **Check-in due** (Phase 5) | The check-in fills the window, with no sidebar, until it's finished (§8.7). Only + Add and the forms it offers open over it. |
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
| `Enter` / `Alt+←` | Next step (Finish on the last) / previous step | Check-in (Phase 5) |
| `↑` / `↓` | Move between the offered focuses | Check-in, Pick a focus |
| `?` | Show these shortcuts | Everywhere, the check-in included |

While a check-in is due, `Ctrl+N` and `?` still work, each opening its dialog over the check-in, and `Ctrl+1` … `Ctrl+4`, `Ctrl+,` and Esc do nothing (CHK-2). Neither one navigates, so neither can skip the check-in, and Esc closes only the dialog (§11 item 98).

`?` opens this table in the **Keyboard shortcuts** dialog, with the two sentences above under it, without their references. The dialog and the key handlers read one list in the code (`src/lib/shortcuts.ts`), so they can't drift apart. While you're typing in a field, `?` types a question mark and opens nothing.

**Skip to content:** the first Tab on any screen of the app shell shows a **Skip to content** link at the top left. Enter on it moves the focus past the sidebar's 13 stops to the screen, so the next Tab is the screen's first control. It's hidden until it has the focus, and the check-in has none, having no sidebar. When a dialog closes, the focus goes back to the button that opened it, if it's still there (a deleted row's Edit isn't) and not behind another open dialog.

---

## 11. Open questions

None at the moment. Answered on 2026-10-26:

1. **First launch:** a plain setup form in Phase 1 (ONB-1), polished in Phase 5 (ONB-2).
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
12. **Compare to:** Settings → General holds the default comparison (SET-7), used by the dashboard's % change and as Reports' starting choice. Reports' **Compare to** changes only that view and is kept in the URL (§6, §8.1).
13. **Reports controls:** the period bar, Compare to and the account filter don't fit in one row at 1440 px, so **Compare to** sits in the Period comparison section's header, the only report it changes, like the breakdown's Expense / Income switch. The account filter, which changes every section, sits beside the period bar (§8.1).

Answered on 2026-11-07:

14. **The Backup tab** lives at `/settings?tab=backup`, like the other Settings tabs, rather than a route of its own (§6, ARCHITECTURE §5.4).
15. **Export to CSV** sits in the Backup tab, with its own From and To dates, rather than on the Transactions screen (§6, BAK-6).

Answered on 2026-11-11:

16. **Review due items** (F10) opens a dialog over the dashboard, rather than a Due section on the Recurring screen, so it works from the banner on its own. Edit & post opens the transaction form over it.

Answered on 2026-11-12:

17. **Card due banner** (CC-7): one banner per card, named after it, so two cards are never confused (CC-8). Archived cards get none, since they take no new transfers and archiving one with a balance already needed a confirmed warning (§2).
18. **Planning screens in the sidebar:** Recurring, Budgets and Goals come after Reports, as §1 shows, with no `Ctrl+` shortcut, since §10 lists none for them. The Phase 5 shortcut review can add some.

Answered on 2026-11-13 (Phase 4):

19. **Debt-free and Forecast** come after Goals in the sidebar (§1), as `/debt-free` and `/forecast`, with no `Ctrl+` shortcut for now.
20. **Loan banners (LOAN-4):** one per loan, for its next unpaid payment, rather than one per payment. That keeps the dashboard to a few banners, inside the blur budget, even when payments pile up, and the banner says how many others are overdue (§2).
21. **Safe to spend on the dashboard** is a card at the top of the right column, above Accounts. Like the balances, it's always for today (§2).
22. **Recording pay early (PAY-4)** works from the Safe to spend card and from the Due dialog. The Due banner counts only the dates that are due, so it doesn't nag for 3 days before every payday (§2, F10).
23. **The payslip's net pay** comes from the backend (`preview_payslip`) as the user types. The frontend never adds up money itself (CLAUDE.md rule 2) (F14).
24. **A new payment row's date** (one more month after the first payment, never after the previous row, so month ends don't drift) is a form default from `lib/dates.ts`, not a figure the app relies on. The backend checks every row (F12).
25. **Adding a new loan** goes through a "Before you add this loan" confirm whose action is **Add this loan anyway**, styled destructive, so taking on debt is never the easy button (F12, LOAN-10).
26. **A loan's payments** open in a dialog from its card, rather than on a page of their own, like the recurring dates due (§8.5).
27. **The Forecast table** opens one row at a time to show its movements, inside the one panel, so nothing new is blurred (§8.6).

Answered on 2026-11-15 (Phase 4, the Debt-free screen):

28. **"Paid Nov 20"** is the date of the expense that paid it, which `list_loans` now returns as each payment's `paidOn`, so it follows an edit to that expense (§8.5, F12, F13).
29. **The Loans panel is full width** until Before you borrow (F15) arrives beside it. Its cards stay two across (§8.5).
30. **"A loan I already have" clears an untouched Date borrowed** (today), and switching back to "A new loan" puts today back. Today's date on a loan the user already has would restart the loan-free streak (F12, DATA_MODEL §5.11).
31. **On the debt-free day itself**, the top panel says "Last payment today" in place of "0 days to go". When everything left is overdue it shows what's overdue instead (§8.5).
32. **Money freed up** shows "This month" on the first row, then "₱X less than this month", "₱X more than this month" or "Same as this month". The mockup's "The last payment" line is left out, since §8.5 doesn't ask for it (§8.5).
33. **A payment row's error** shows under the list, and marks the row it names. The backend numbers rows among those sent, which leaves out the locked paid ones, so the form numbers its rows the same way ("Payment 3 date"), leaves the locked ones unnumbered, and maps the number back to the row on screen (F12).
34. **Editing a transaction that pays a loan** hides the type switch, since it must stay an expense (DATA_MODEL §4.6).
35. **A card's overdue next payment** reads "Next ₱1,000.00 overdue since Mon, Nov 16", in `--danger` with the icon, so it keeps the amount and matches the "Next ₱1,000.00 due Fri, Nov 20" line it replaces, using DESIGN_SYSTEM §8's "overdue since" phrase (§8.5).

Answered on 2026-11-15 (Phase 4, paydays and payslips):

36. **The deduction categories listed first** on a payslip line are the six PAY-3 presets, matched by name: SSS, PhilHealth, Pag-IBIG, Withholding tax, Salary loan and Other deductions. It only orders the list, so a renamed preset is listed with the other expense categories (F14).
37. **A payslip line's error** shows under that line, and an error about the lines as a whole (the net not above 0, more than 30 lines) shows once under Net pay (F14).
38. **Duplicate on a payslip row** copies just that row as a plain transaction, which belongs to no payslip (§3).

Answered on 2026-11-15 (Phase 4, the Safe to spend card):

39. **Recent transactions fills its column.** The Safe to spend card made the right column taller than 5 recent rows, which left a gap. The panel now stretches to the right column's height and shows as many whole rows as fit, from 5 to 10 (`get_dashboard` returns 10). The rows that don't fit are invisible, never cut in half (§2, DSH-8).

Answered on 2026-11-16 (Phase 4, the Forecast screen):

40. **Which rows run short or are lowest** comes from `get_forecast`, as each row's `short` and `lowest`. A row can run short and recover by its end, so the frontend can't tell from its figures without adding up its movements (§8.6, DATA_MODEL §5.15).
41. **The Months view's lowest** is the month holding the lowest day, the same day the window held back for holds. Nothing is marked when nothing is held back (§8.6).
42. **Each movement's label and account** come from `get_forecast` too, rather than from four more lookups on the frontend. A transfer to or from an account that isn't counted, such as a goal's, reads "Transfer to Savings" or "Transfer from Savings", on the counted account (§8.6).
43. **Received pay shows its date** ("₱10,500.00 received Nov 15"), since one row can hold two payslips: one recorded early, or two paydays in the Months view's first month (§8.6, DATA_MODEL §9 item 81).
44. **The current row starts today** in the Pay windows view too, though its window started on the last payday (§8.6).
45. **"Whose bills are more than its pay"** is left out of the held-back line. Money can also be held back for a bill due before the pay in the same window, where the clause isn't true. The Forecast's top panel is the dashboard's card as it is, without its View forecast link (§8.6).

Answered on 2026-11-16 (Phase 4, Before you borrow):

46. **Before you borrow lists every pay window it changes**, in a list that scrolls inside the panel, since what's left carries from window to window: every window from the first payment on changes. A window it makes short reads "Dec 25, 2026 – Jan 24, 2027 would run short by ₱991.23" in `--danger` with an icon, as in the preview. "Before you add this loan" names only the short ones (F12, F15).
47. **Which windows are short, and the days later,** come from `preview_loan` (`shortBy`, `daysLater`), like the Forecast's `short` (item 40). A window that runs short without the loan isn't blamed on it (DATA_MODEL §9 item 82).
48. **When nothing moves**, the panel says "(same day)" and the confirm "Your debt-free date stays Mon, Jun 28, 2027." With nothing owed before, the panel starts from "today" and the confirm says "You'd be debt-free on Fri, Oct 15, 2027." With no payday rule, the Safe to spend lines are left out (F12, F15).
49. **Before you borrow sits beside Loans**, half width each (item 29), and is left out with no loans at all, when the screen is one empty state (§8.5).
50. **The first payment starts a month from today**, a form default the user can change. Blank fields go to the backend as 0 or empty, so its message shows under the field. Changing a field clears the result and its errors, so nothing shown is for other values, and Try it with the same values asks again (F15).
51. **"Before you add this loan" opens over the Add loan form**, so Go back returns to it as it was. The other fields are checked when Add this loan anyway saves, and an error there shows on the form (F12).

Answered on 2026-11-16 (Phase 4, planning smart goals and card spending levels):

52. **The goal dialog asks `suggest_goal` as the user types**, after 250 ms, like the payslip's net (item 23). The frontend never works out the monthly room or the date (§8.4, DATA_MODEL §5.17).
53. **A new goal's date starts on the suggestion** once there is one, and the user may change it but not clear it. An edit keeps its date, and the line shows the suggestion beside it. A goal from before Phase 4 with no date opens with an empty date, which must be filled in to save (§8.4, DATA_MODEL §9 items 90–91).
54. **The card spending confirm** is a ConfirmDialog over the transaction form, as "Before you add this loan" is over the loan form (item 51), so Go back returns to the form as it was (F16).
55. **A goal moved by a save is told with a toast**, not a dialog, since there's nothing to choose: the save went ahead and the date moved (DATA_MODEL §9 item 92). The toast follows any card spending confirm, which comes before the save (F16).
56. **Behind is shown on the goal's card only**, in `--warning`, without a banner, since goals don't hold money back (PRD §3).
57. **The goal-moved toast has no icon and stays 8 s**, one line per goal in a single toast (answered on 2026-11-16, building GOL-7): it's a note rather than a success or an error, and several lines need longer than the 4 s default to read. Toasts now show one above the other rather than piled up, since the form's "Saved" comes after the note and would otherwise cover it (§8.4, DESIGN_SYSTEM §9.1).

Answered on 2026-11-16 (Phase 4, building card spending levels):

58. **The level is its own row under This cycle** on the card's details ("Spending level · Past your ₱3,000.00 soft level"), since the This cycle row already says what's spent. The Accounts list and the dashboard add "· Soft level" to the card's line (§5.3, F16 step 5).
59. **The level warning is its own toast**, in `--warning` with the `triangle-alert` icon, after "Saved", and stays 8 s like the goal-moved note. Edit & post and the Due dialog's Post never ask (DATA_MODEL §9 items 104–105).

Answered on 2026-11-16 (Phase 4, testing on a copy of the owner's data):

60. **Deleting a payslip says its payday won't come back.** The payday stays settled (DATA_MODEL §4.7), so the pay it recorded leaves the forecast for good, which can make a window run short, and Review or Record now never offers it again. The confirm says so and points to editing instead; the rule itself is unchanged (F14 step 4).
61. **Mark paid names the payment** in its title, "Mark paid: E-wallet loan, due Fri, Nov 20" (or "overdue since …"), so paying early or from a banner says which payment it records (F13).
62. **Exactly at a level says "reached"**, not "past": "Reached means equal or more" (DATA_MODEL §5.18), and "past" read wrong at the boundary. Above it, "past" stays (F16, §5.3).
63. **The first payslip starts with no gross**, rather than the rule's amount, which is the expected **net** and looked like a gross with nothing deducted. The net shows nothing until a gross is typed (F14).
64. **"Safe to spend until today"** when the current window ends today, instead of today's own date (§2).

Answered on 2026-11-17 (Phase 5, the check-in and the Vault level):

65. **The check-in is a full-window route, not a dialog** (§8.7), so quick-add and the pay forms can open over it without glass inside glass, and so it can't be dismissed like a dialog ([ADR 0012](decisions/0012-required-check-in.md)).
66. **Every step is its own screen**, with the same steps for a week and a month, so the weekly one stays about a minute long and a step can be acted on before moving on.
67. **The focus has nothing preselected**, so Enter alone can't finish a check-in.
68. **The sidebar item is "Level"**, with the level chip beside the app name on every screen (§1, §8.8).
69. **A failed load never locks the app.** If the check-in can't be loaded, the app opens normally and asks again on the next window focus (§8.7).
70. **The first-launch form stays plain.** ONB-2's step indicator and tour were dropped (the owner, 2026-11-17): the only user is past it. This replaces item 1's "polished in Phase 5" (F5).
71. **The check-in's step is kept in the URL by name**, not number, because the Goals step can appear after a save (§8.7).
72. **A check-in that can't be saved doesn't lock the app** (the owner, 2026-11-19): a failed Finish reloads the check-in, clears a pick that's no longer offered, and offers "Open the app anyway" for that session only. The check-in is also asked for at midnight, not only on window focus (§8.7, F17, ADR 0014).

Answered on 2026-11-19 (Phase 5, drawing the previews):

73. **The level chip shows the number, not "Lv 4"** (the owner): about 40 px is free beside the app name, enough for the ring around the small hexagon but not for text beside it. Screen readers still hear "Level 4, 1 of 2 conditions met for level 5" (§1, DESIGN_SYSTEM §9.2, PRD LVL-5).
74. **The check-in's panel keeps one height** on every step, from the top bar to the window's bottom, so Back and Next never move (§8.7).
75. **Each step's sentence** that §8.7 didn't give is settled where it was needed to draw it: the Card step's under-soft and several-card sentences, "You're still at level 4." for a level that didn't change, and "Helps your Bills stat" (or Card, or Habit) as what a non-cap offer is based on, since `get_checkin` gives a `basis` only for caps (ARCHITECTURE §4.5).
76. **The Level screen's checklist, move and reason wording** follows one pattern (§8.8), with the codes the preview needed written out. The others are written with the screen.

Answered on 2026-11-20 (Phase 5, building the check-in):

77. **The sentences the steps open with** that §8.7 didn't give: a win as "You kept your focus.", "Your Savings stat rose from 73 to 85.", "You spent less than the week before." or "Every loan payment was paid on time."; Money in and out as "You earned ₱X more than you spent." (or "spent ₱X more than you earned", "Nothing came in or went out this week."); Loans and bills as what's overdue first ("A loan payment is overdue."), else what was paid ("2 loan payments paid, ₱2,000.00."), else "No loan payments were paid this week." or "No loans to pay."; Goals as "1 goal is behind.", "Your goals are on track." or "No goals yet."; and a missed cap's result as F18 says it: "You spent ₱1,720.00, past your ₱1,500.00 target." A missed card level says "It reached ₱5,300.00.", a missed logging focus "You logged on 3 days.", and a void one "That loan payment was removed, so it doesn't count." (§8.7).
78. **What a budget went over by and the total paid on loans** come from `get_checkin` (`over`, `paidTotal`), so the frontend never subtracts or adds money (CLAUDE.md rule 2, ARCHITECTURE §4.5).
79. **A card-level offer names its card** only when there's more than one active card: "Stay under Credit card's ₱5,000.00 hard level this cycle" (§8.7).
80. **Enter is Next** unless the keyboard is on a button, link or field of its own, so Enter on a focused Back, segment or form control does what that control does. Alt+→ and Alt+Home do nothing on the check-in, so WebView2 never goes forward or back behind it (§10).
81. **A step that goes away after a save** (Goals, when no goal is Behind or moved any more) leaves the user on the step after it, since the step is kept by name. A Balance check not yet answered can't be passed that way: a step after it shows the Balance check instead, which also covers a weekly check-in that becomes the merged one at midnight (§8.7, CHK-4).
82. **A check-in that comes due while a screen's own dialog is open** (a goal, budget or loan form) takes over all the same, and the dialog's draft is lost. ARCHITECTURE §5.4's redirect is kept as it is: it needs a form left half-filled across midnight or a window switch, and holding the redirect for open dialogs would let one postpone the check-in. Quick-add and the pay forms sit above the guard, so they stay open (§8.7, ADR 0012).

Answered on 2026-11-20 (Phase 5, building the Level screen):

83. **The checklist's wording** is in §8.8. A condition reads as a fact or as the thing to do, whichever reads plainly both met and unmet, and only an unmet one shows its gap (§8.8).
84. **An unchanged score shows `—`**, as the previews draw it, on the Level screen and in the check-in's Stats and level step, which showed `0` before. Screen readers hear "No change" (§8.7, §8.8).
85. **A path node's label says its state in words**, "Level 4 · In the black, your level", "…, reached" or "…, not reached", so the state is never shown by color or icon alone. Its tooltip, which focus and hover both open, is raised glass like a menu (§8.8, DESIGN_SYSTEM §9.2).
86. **A card-level focus on the Level screen** reads "Stay under your ₱4,000.00 mid level · ₱2,310.00 this cycle", as on the dashboard (§2). It names its level when its target is still one of the card's three, and otherwise its amount and card: "Stay under ₱4,000.00 on Credit card". A logging focus keeps the offer's sentence, "Log on at least 5 days this week · 3 days so far", so a focus reads the same in the check-in, on the dashboard and here, and a cap's bar takes its color from its `state`, so it turns red only past the target (§2, §8.8).
87. **The loan-free streak** on the Level screen is `get_vault_status`' `loanFreeDays`, the same §5.11 figure the loans summary gives, so the screen needs only that one command for its figures (§8.8, DATA_MODEL §5.11).
88. **The dashboard's focus rows are flat in the Safe to spend card**, not a strip of their own (2026-11-20). Measured in the running app, with every banner showing once, the dashboard already has 16 blurred panels: the sidebar, the top bar and the period bar; the loan, backup, card and recurring banners; the three stat cards; and the chart, Recent transactions, Safe to spend, Accounts, Budgets and Coming up. With no banner it's 12, the "about 12" of DESIGN_SYSTEM §11, so a strip would take it to 13 with none and 17 with every banner. The rows come from `get_dashboard`'s `focuses`. A save that raises a cap's `state` warns once (F18): after each save the user makes, the frontend compares the cached `get_vault_status` from before it with the one after the ledger refresh, so recurring dates posted by processing never warn, and a card level, a loan payment or logging, which have no `state`, never warn either (§2, F18, ARCHITECTURE §5.2).
89. **The focus record's icons and the trend lines** (2026-11-20, LVL-8). Each icon is a tab stop, like a `LevelPath` node, and its label and tooltip are the focus's sentence and its result after "·": "Keep Dining Out under ₱1,500.00 · Missed". It's the focus's sentence as it was offered, with today's names, so a past focus keeps "this week" and a card level that has changed since gives its amount instead. The label stays "Last 8" with fewer than 8. A trend line breaks where the stat didn't apply at a check-in, so it never suggests a value there, and a stat with no 2 points in a row has an empty slot, so the rows stay aligned. The lines are hidden from screen readers, which get the score and its change (§8.8).
90. **The level-up moment plays once per opening of the check-in** (2026-11-20, LVL-6). It plays when Stats and level is first shown, whether reached with Next or opened straight from the URL, and leaving the step ends it, so Back and Next never replay it. Reloading the window opens the check-in afresh and plays it again, since nothing about it is stored. The glow is a CSS `drop-shadow` filter on the hexagon, never `backdrop-filter` (DESIGN_SYSTEM §10, §11).
91. **Acting from the check-in** (2026-11-20, CHK-9). **Pay bill** shows on each card with something owed, **Pay** on every row of the dashboard's `loansDue`, and **Record now** on the first payday `list_due_occurrences` gives (a due one before an early one), which then takes the place of the "Next payday" line. A payment paid from here is after the shown period, so it isn't in the step's `paid`: the screen remembers which payments Pay was clicked for, and each one `list_loans` then shows as paid after the period's end stays in its place (one dated back into the period is already in `paid`) as "<loan>: <paid amount> paid today" with a check icon, until the check-in closes or the window reloads. A cancelled form changes nothing. When nothing is overdue any more and one of those was overdue, the sentence is "You paid what was overdue.", with the period's paid line under it (the owner, 2026-11-20). If `list_due_occurrences` fails, the step just has no Record now. A row whose account is short has the banner's "E-wallet has ₱150.00. Move ₱850.00 there first." under it, and each button's accessible name says what it pays: "Pay E-wallet loan", "Pay bill for Credit card".
92. **Recent transactions shows up to 20** (2026-11-20, the owner, DSH-8). With the focus rows (CHK-8), several accounts and the loans line, the right column grew taller than 10 rows, which left about two rows of empty panel under the last one (item 39). `get_dashboard` now returns 20, and the panel still shows as many whole rows as fit, at least 5.
93. **Two check-in sentences after testing on real data** (2026-11-20, the owner, T18). The Goals step is left out when there's no active goal, instead of a step that only said "No goals yet.", so a monthly check-in with no goals has 7 steps. The Card step stays its own step rather than being folded into Loans and bills: with one card it's short, and it holds **Pay bill**. The first check-in says "You're at level 0.", not "still at", since there's no earlier level (§8.7).
94. **The card form says what its amount owed field holds** (2026-11-20, the owner, CC-1, CC-8). The field is always the opening amount owed, never today's figure. Typing today's figure into it would add the card's transactions a second time, so the balance would stop matching the history. A new card has no transactions yet, so its form keeps "Amount owed now". The edit form calls it "Opening amount owed" and shows "Owed today: ₱X" under it, so the two figures aren't mistaken for each other (§5.2).
95. **The Add and Edit loan forms total their payment rows** (2026-11-20, LOAN-1). A loan can have dozens of rows, and the total is what the user checks against the lender app before saving. Each row is sent with whether it shows as paid, so the figures always match the labels on screen, and the backend adds them up (CLAUDE.md rule 2), as `preview_payslip` does for net pay. `preview_loan` isn't used: it reads the forecast and refuses a half-typed row. One figure while nothing counts as paid, since then the total is what's still to pay; two once something does (F12).
96. **The goal dialog and the goal card name their monthly figures** (2026-11-20, GOL-3, GOL-6). For one goal and one date they showed two amounts, both "a month": the dialog's "You can set aside about ₱650.00 a month" and the too-soon message's "saving about ₱650.00 a month" are the monthly room, while the card's "Save ₱433.34/month to reach it" is what the chosen date needs. Now the dialog's line says "The most you could set aside is about ₱650.00 a month. The earliest realistic date is …" (two sentences, since with nothing left to save the date is today whatever the room), the too-soon message says "Even setting aside the most you could, about ₱650.00 a month, the earliest you can reach this is …", and the card says "… to reach it on time". The figures are worked out as before (§8.4, DATA_MODEL §5.17, §9 item 148).
97. **HMO is a deduction category** (2026-11-21, PAY-3). This replaces item 36's six presets and its exact-name match. A payslip line lists HMO after Withholding tax, so the seven presets come first: SSS, PhilHealth, Pag-IBIG, Withholding tax, HMO, Salary loan and Other deductions. The match ignores case, as migrations 0003 and 0006 do, so a category the user named "hmo" is listed with them too, while a renamed one is still listed with the other expense categories (F14, item 36, DATA_MODEL §7).
98. **`?` shows the keyboard shortcuts, the check-in included** (2026-11-19, the owner). It works wherever `Ctrl+N` does, so not on the first-launch setup form, which sits outside the app shell and the check-in (F5). It opens the **Keyboard shortcuts** dialog over the check-in, as `Ctrl+N` opens quick-add: it never navigates, so it can't skip the check-in, and Esc closes only the dialog (§10, CHK-2). The dialog and the key handlers read one list (`src/lib/shortcuts.ts`), and a handler can only bind a key on it. While you're typing in a field `?` types a question mark. Every shortcut in §10 was pressed in the real app on its own screen (2026-11-21). One was wrong: Alt+← on a focused card in Pick a focus also moved the pick to the previous offer, and now it's only Back. The screens added since Phase 2 (Recurring, Budgets, Goals, Debt-free, Forecast, Level) still have no `Ctrl+` number, so item 18 stands until the owner asks for some.
99. **The accessibility pass** (2026-11-23, the owner, from T24's findings). The app shell has a Skip to content link (§10), a dialog gives the focus back to what opened it, a toast with an action stays until it's closed, and a failed Finish keeps the focus on Finish with its toast clear of it (§8.7). Sonner's own Alt+T, which jumped to the toasts, is off: it clashed with quick-add's Alt+T and isn't in §10.
100. **The dashboard's banners share one panel** (2026-11-23, the owner, from T25's PERF-01). With every banner showing, five separate banner panels took the dashboard to 14 blurred surfaces, past DESIGN_SYSTEM §11's "about 12". One panel holding them as flat tinted rows keeps it at 10, whatever shows. Each banner keeps its tone, icon, text and button.
