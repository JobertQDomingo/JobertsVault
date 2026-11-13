# UI Flows: Jobert's Vault

| | |
|---|---|
| **Status** | Approved v1.2 |
| **Style** | Glassmorphism 2.0 ([DESIGN_SYSTEM.md](DESIGN_SYSTEM.md)) |
| **Last updated** | 2026-11-13 |
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
| Stat cards | Income, expense and net cashflow for the period. Transfers aren't included. From Phase 2, each card shows its % change from the previous period, compared by the Settings choice (SET-7), and the dates compared with: `▲ 3.1% vs Oct 1–6`. The change is green when the direction is good (DESIGN_SYSTEM §7), or "—" when there's nothing to compare with. The net cashflow card is the only one tinted with the accent. | DSH-2, DSH-3 |
| Income vs expense (Phase 2) | A small version of the Reports chart (§8.1) for the period, under the stat cards. Its bars come with the rest of the dashboard from `get_dashboard`. | RPT-1 |
| Recent transactions | The 5 newest transactions **regardless of the selected period**. Clicking a row opens it for editing. "View all" goes to `/transactions`. | DSH-8 |
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
    - "Safe to spend until Sun, Nov 29", then the amount in `display` size
    - "₱560.00 a day · 15 days left"
    - "₱1,500.00 held back for Nov 30 – Dec 4", or, when even that isn't enough, ₱0.00 and "Short by ₱1,240.00 on Fri, Dec 4" in `--danger`, with an icon
    - from 3 days before a payday: "Payday Mon, Nov 30. Paid early? **Record now**" (PAY-4, F14)
    - **View forecast** in its header, to `/forecast`
    - With no payday rule: "Mark your pay as a payday to see what's safe to spend." with **Open Recurring**
  - **"Loans left: ₱24,600.00 · debt-free Jun 28, 2027"** under the net balance in Accounts, linking to `/debt-free`. It's hidden when nothing is left (LOAN-11).

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
| **General** | Currency symbol, its position (before or after the amount), decimal places (**locked** once any amount is saved, with an explanation), first day of week, month start day (1–28), **compare periods** (same days so far or full previous period, SET-7, Phase 2), show payment method field. **Updates:** the current version, **Check for updates** and a switch, "Install updates automatically when the app starts" | SET-1, SET-2, SET-6, SET-7, UPD-3 |
| **Categories** | Two lists (Income and Expense). Add, rename, change color (12 swatches) and icon (curated list), reorder, archive, unarchive, **merge**, delete if unused | CAT-2, CAT-4 |
| **Payment methods** | Add, rename, reorder, archive, unarchive, delete if unused | TXN-7 |
| **Backup** (Phase 2) | **Back up now** (BAK-1, F9). **Restore from backup…** (BAK-4). **Export to CSV**, with From and To dates, from the start of this month to today by default (BAK-6). The backup folder with **Change…**, and a switch for the daily automatic backup (SET-3). The date of the last backup, and the warning when it's more than 7 days old (BAK-7). | BAK-1 to BAK-4, BAK-6, BAK-7, SET-3 |
| **Appearance** (Phase 5) | Theme: system / light / dark, and a **Reduce transparency** switch | SET-5, SET-8 |

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

### F5: First launch (Phase 1 basic version ONB-1, Phase 5 polished version ONB-2)

```
Welcome to Jobert's Vault
  Step 1  Currency: [ ₱ ▾ ]  Position: [Before ▾]  Decimals: [2 ▾]
  Step 2  Your accounts: Cash (already added) · [+ Bank] [+ E-wallet] [+ Credit card]
          (name + opening balance; card asks for its limit, statement day, due day)
  Step 3  [Start using the app]
```

- The **Phase 1** version is a plain form, because the currency must be set before the first transaction. **Phase 5** adds polish: a step indicator and a short tour.
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
   - **Payments:** one row each, with **Date** and **Amount** and an ✕ to remove it. **+ Add payment**, or Enter in the last amount, adds a row with the previous row's amount, dated one more month after the first payment, with the day clamped to the month's end, so month ends don't drift (Jan 31, Feb 28, Mar 31). That's a form default from `lib/dates.ts` that the user can change; the backend checks every row. On a loan the user already has, a row dated before today shows "Paid before".
2. **Save:**
   - **A loan the user already has** is saved at once, with a "Loan added" toast.
   - **A new loan** first shows the "Before you add this loan" confirm (LOAN-10), from `preview_loan`: "Your debt-free date moves from Mon, Jun 28, 2027 to Fri, Oct 15, 2027.", "Safe to spend drops from ₱8,400.00 to ₱7,400.00.", and, when it would make a pay window short, a line such as "Nov 30 – Dec 4 would run short by ₱180.00." **Go back** returns to the form, and **Add this loan anyway** (destructive) saves it and restarts the loan-free streak.
3. **Edit** (the loan's ⋯ → **Edit**, or **Edit** in its payments dialog) is the same form, titled "Edit loan". Paid rows are locked and show "Paid Nov 9". "This is" can't change.

### F13: Pay a loan payment (Phase 4, LOAN-3, LOAN-4)
1. Click **Pay** on the dashboard's loan banner, or open the loan on Debt-free and click **Mark paid** on an unpaid row.
2. The transaction form (§4) opens, titled "Mark paid", with the type fixed to Expense and these fields filled in: the scheduled amount, the loan's account and category, today's date, and the loan's name as the payee. Change the amount for a late fee. **Save** records the expense, with a "Marked paid" toast. The balances go down, and the row says "Paid Nov 20".
3. **To undo it**, open the paid row's ⋯ menu in the payments dialog and choose **Mark unpaid**. It confirms first: "Mark this payment unpaid? The expense recorded for it is deleted." Deleting that expense on the Transactions screen does the same.

### F14: Record a payslip (Phase 4, PAY-2, PAY-4)
1. **On payday:** the Due banner (F10) → **Review** → **Record payslip** on the payday's row. **Paid early:** **Record now** on the Safe to spend card, or on the payday's early row in the Due dialog, from 3 days before.
2. The "Payslip · Salary" dialog opens, filled in from that payday rule's last payslip, else the last Salary payslip, else the rule's amount the first time (`get_payslip_defaults`):
   - **Date:** the payday, or today when it's early
   - **Account**
   - **Gross pay**
   - **Deductions:** one row each, with a category (the deduction categories come first, then the other expense categories) and an amount, plus an ✕. **+ Add deduction** adds one.
   - **Note**
   - **Net pay**, shown under the rows. It comes from the backend as the user types (`preview_payslip`), and it's in `--danger` with "Deductions can't add up to the gross pay or more" when it isn't above 0.
3. **Save** records the gross as income and each deduction as an expense (DATA_MODEL §4.7), with a "Payslip saved" toast, and settles the payday.
4. **Editing:** clicking any of its rows on Transactions opens it again as "Edit payslip". **Delete** (bottom left) asks "Delete this payslip? Its gross pay and 4 deductions are deleted." The payday stays settled.

### F15: Before you borrow (Phase 4, LOAN-9)
1. On Debt-free, the **Before you borrow** panel asks for the **Payment** amount, **How many** payments and the **First payment** date. The payments are monthly, on the first payment's day of the month.
2. **Try it** shows the result from `preview_loan`, and nothing is saved:
   - "Debt-free: Jun 28, 2027 → Oct 15, 2027 (109 days later)"
   - "Safe to spend now: ₱8,400.00 → ₱7,400.00"
   - each pay window it changes, with what's left before and after ("Dec 15–29: ₱18,300.00 → ₱17,300.00"), and any window it would make short in `--danger`

### F11: Update the app (Phase 1)
- **Automatic (the default):** at startup, the app checks for a newer version. "Checking for updates…" appears only if the check takes more than 150 ms. If there's a newer version, "Updating Jobert's Vault to 0.2.0…" appears, and the app closes and reopens on it. There's nothing to click (UPD-1).
- **No internet, or the check fails:** the app starts normally and shows no error (UPD-2).
- **By hand:** Settings → General → **Check for updates** says "You're up to date." or "0.2.0 is out." with an **Update now** button. It restarts the app, so anything unsaved in an open form is lost: the button is in Settings, where no form is open (UPD-3).
- **Automatic updates off:** the app doesn't connect by itself. **Check for updates** still works (UPD-3).
- **If an update is refused or fails:** the message says why and adds "Nothing was changed." (UPD-4).

---

## 8. Phase 2–4 screens (outline)

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
One panel, with **+ New goal** in its header, holding a grid of flat goal cards (three across), in the order they were created. Each card shows the name, the linked account and target date ("Bank · by May 5, 2027"), a `ProgressBar` with the % done, "₱2,400.00 of ₱5,000.00 · ₱2,600.00 left", and "Save ₱433.34/month to reach it" (GOL-3). Once the target date is today or has passed, that line says "Target date passed" instead, and a goal with no target date has no line. A goal that's been reached gets an **Achieved** badge in `--success`, its bar turns green, and it asks for no monthly amount, even if the balance drops later (GOL-4). A goal's bar has no amber or red thresholds.

Clicking a card edits it. Its ⋯ menu has **Edit**, **Archive** and **Delete** ("Delete this goal? Its account and transactions stay. This can't be undone."). Any goal can be archived, achieved or not. Archived goals fold under **Show archived (n)** below the grid, as the same cards, whose ⋯ has **Unarchive** and **Delete**. With no active goals, an empty state offers **New goal**: "No goals yet.", or "No active goals." when some are archived.

**+ New goal** opens a small dialog, "New goal" (or "Edit goal"):
- **Name:** 1–50 characters, unique.
- **Target:** the amount to reach.
- **Target date:** optional, any date. With one, the card says how much to save each month.
- **Account:** an active account that isn't a credit card. An edit keeps its own account even if it's been archived. A note says the account's balance is the goal's progress (GOL-2).

The backend's message shows under the field it names.

### 8.5 Debt-free (`/debt-free`), Phase 4

```
┌─ Debt-free ─────────────────────────────────────────────────────────────────────┐
│ Debt-free on Mon, Jun 28, 2027                                 227 days to go   │
│ ₱24,600.00 left · ₱6,150.00 paid   ██████░░░░░░░░░░░░░░░░ 20% paid               │
│ 52 days since your last new loan · ₱3,050.00 in interest on 2 loans             │
└─────────────────────────────────────────────────────────────────────────────────┘
┌─ Loans ──────────────────────────────── [+ Add loan] ┐ ┌─ Before you borrow ─────┐
│ ┌─ E-wallet loan ────────┐ ┌─ Bank loan ───────────┐ │ │ Payment   [ ₱1,000.00 ] │
│ │ E-wallet · 3 of 8 paid │ │ Bank · 1 of 12 paid   │ │ │ How many  [ 6 ]         │
│ │ ███████░░░░░░ 37%      │ │ █░░░░░░░░░░░░ 8%      │ │ │ First     [ 12/18/2026 ]│
│ │ ₱5,000.00 left         │ │ ₱11,000.00 left       │ │ │           [ Try it ]    │
│ │ Next ₱1,000.00 Nov 20  │ │ Next ₱1,000.00 Dec 14 │ │ │ (the result, F15)       │
│ └────────────────────────┘ └───────────────────────┘ │ └─────────────────────────┘
│ ▸ Show archived (1)                                  │
└──────────────────────────────────────────────────────┘
┌─ Money freed up ────────────────────────────────────────────────────────────────┐
│ November 2026     ₱2,500.00 to loans                                            │
│ December 2026     ₱7,800.00 to loans    ₱5,300.00 more than this month          │
│ April 2027        ₱1,400.00 to loans    ₱1,100.00 less than this month          │
│ November 2027     ₱0.00                 ₱2,500.00 less than this month          │
└─────────────────────────────────────────────────────────────────────────────────┘
```

- **The top panel** (LOAN-5 to LOAN-8) shows the debt-free date and days to go, the total left and paid with a `ProgressBar` of the share paid, the loan-free streak ("52 days since your last new loan", hidden when no loan has a date borrowed) and the total interest (hidden when no loan has an amount borrowed). With everything paid, it says "You're debt-free." in `--success`.
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
│ ₱1,500.00 held back for Nov 30 – Dec 4, whose bills are more than its pay       │
└─────────────────────────────────────────────────────────────────────────────────┘
╭─ [ Pay windows │ Months ] ─╮
┌─────────────────────────────────────────────────────────────────────────────────┐
│ Window            Start         Pay in        Bills          Left               │
│ Nov 15–29         ₱12,500.00    —             ₱2,600.00      ₱9,900.00          │
│                   ₱10,500.00 received                                           │
│ Nov 30 – Dec 4    ₱9,900.00     ₱10,500.00    ₱12,000.00     ₱8,400.00  lowest  │
│ ▾ Dec 5–14        ₱8,400.00     ₱6,000.00     ₱4,000.00      ₱10,400.00         │
│     Sat, Dec 5    Bonus (expected)                   Bank      +₱6,000.00       │
│     Thu, Dec 10   Bank loan                          Bank      −₱4,000.00       │
│ Dec 15–29         ₱10,400.00    ₱10,500.00    ₱2,600.00      ₱18,300.00         │
│ …                                                                               │
└─────────────────────────────────────────────────────────────────────────────────┘
```

Today in this sketch is Sun, Nov 15, a payday whose payslip is already recorded. Spending the ₱9,900.00 left now would leave Nov 30 – Dec 4 ₱1,500.00 short, so ₱1,500.00 is held back and ₱8,400.00 is safe to spend.

- **The top panel** is the dashboard's Safe to spend card at full width (SAFE-5, SAFE-6): the amount, the amount per day, and what's held back and for which window, or "Short by ₱X on *date*" in `--danger`. When a payday is 1–3 days away, it also has "Paid early? **Record now**" (F14).
- **The toolbar's segmented control** switches between **Pay windows** and **Months**, kept in the URL (`?view=months`) (SAFE-7).
- **The table** has one row per pay window, or per month, over the next 12 months. Each row shows its dates (the current one starting today), what it starts with, the pay and other money coming in, the bills, and what's left at its end if nothing else is spent (DATA_MODEL §5.15). A payslip already recorded shows as "₱10,250.00 received" in muted text, without being added again. The window money is held back for is marked **lowest**. A window that would run short has a triangle icon and its Left in `--danger` with "Short".
- **Clicking a row** opens it in place to list its movements, by date: each loan payment (by loan name), card bill ("Credit card payment"), recurring date ("Utilities · Water bill"), future-dated row, and payday ("Salary (expected)"), with its account and a signed amount. Only one row is open at a time.
- **With no payday rule,** the screen is one empty state: "Mark your pay as a payday to see what's safe to spend." with **Open Recurring**.

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

The Phase 5 polish adds a `?` shortcut that shows this list in a help dialog.

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
