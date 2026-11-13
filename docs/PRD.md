# Product Requirements Document: Jobert's Vault

| | |
|---|---|
| **App name** | Jobert's Vault |
| **Status** | Approved v1.2 |
| **Last updated** | 2026-11-13 |
| **Platform** | Windows desktop app built with Tauri 2: React + TypeScript frontend, Rust backend, SQLite database |

---

## 1. Overview

A private Windows desktop app for tracking personal cashflow (income and expenses) and viewing it by **day, week, month, year or a custom range**. All data is entered by hand and stored locally on the user's computer. Financial data is never sent over the internet. The only network use is checking for and downloading app updates (§7.13).

Besides cashflow, the app tracks balances across several accounts (including one credit card), budgets, recurring transactions and savings goals. Local backups, restore and CSV export came in Phase 2 (`v0.2.0`). Phase 4 (`v0.4.0`) adds loans with payment reminders, payslips with their deductions, and **Safe to spend**: what the user can spend until the next payday after every bill coming in the next 12 months.

## 2. Goals

1. **Fast logging.** Recording a transaction takes under 5 seconds and a few keystrokes.
2. **Clear cashflow at any time scale.** The user sees income, expenses and net cashflow for any day, week, month or year, and how that compares to the previous period.
3. **Know where the money goes.** Spending is broken down by category and shown in charts.
4. **Accurate balances.** Every account balance, including the credit card amount owed, matches reality.
5. **Plan ahead.** Budgets, recurring transactions and savings goals help the user stay on track.
6. **Data is never lost.** Automatic backups, easy restore and export to an open format (CSV).
7. **Never miss a loan payment, and know what's safe to spend.** Every loan payment is reminded until it's paid, borrowing again is made to feel what it costs, and the user always sees how much they can spend until the next payday.

## 3. Non-goals (out of scope for v1)

- Bank or credit card syncing, and importing statements (CSV/OFX)
- Receipt scanning or attaching receipts
- Multiple users, shared households, or login and accounts
- Cloud sync between devices
- Multiple currencies and exchange rates
- Investment, stock or crypto portfolio tracking
- Tax reporting
- macOS, Linux and mobile versions
- Splitting one transaction across several categories
- Recording the time of day on transactions (date only)
- Budget rollover (unused budget never carries over)
- Splitting a loan payment into interest and principal
- Tracking a loan repaid through payroll deductions as a loan (it's only a deduction line, PAY-3)
- Setting money aside in Safe to spend for budgets or savings goals

## 4. Target user

One person (the developer/owner) who:
- has a few accounts: cash, one or more bank accounts, e-wallet(s) and **one credit card**
- wants to log every transaction by hand to stay aware of their spending
- uses Windows 10 or 11
- values privacy and wants their financial data kept on their own computer

## 5. Key concepts and definitions

| Term | Definition |
|---|---|
| **Account** | A place money is held: Cash, Bank, E-wallet or Credit Card. Each has a balance. |
| **Transaction** | A single money movement. Its type is exactly one of: Income, Expense or Transfer. |
| **Income** | Money coming in from outside, such as salary or gifts. It increases an account's balance. |
| **Expense** | Money going out, such as food or rent. It decreases an account's balance, or increases the amount owed on the credit card. |
| **Transfer** | Money moving between the user's own accounts, such as Bank → Cash or Bank → Credit Card (paying the bill). **Transfers are never counted as income or expense.** |
| **Net cashflow** | Income minus expenses for a period. Transfers are not included. |
| **Category** | A label for income or expenses, such as Salary, Food or Rent. Income categories and expense categories are kept in separate lists. |
| **Period** | A time window for reports: Day, Week, Month, Year or Custom. |
| **Opening balance** | An account's balance when it is first added to the app, entered as 0 or more (for a credit card, the amount owed). It is **not** counted as income. |
| **Transaction date** | A calendar date only, with no time of day. Transactions on the same date are listed in the order they were entered. |
| **Loan** | Money the user borrowed and repays on a schedule, kept as one loan per lender app. It's a list of **loan payments**, not an account. |
| **Loan payment** | One scheduled payment of a loan: a date and an amount. It stays **unpaid** until the user marks it paid, which records an expense. An unpaid payment counts toward no balance, total, report or budget. |
| **Payday** | The date of an income recurring rule marked as a payday, such as salary or incentives. |
| **Payslip** | What's recorded on a payday: the gross pay as income, and each deduction as an expense, on the same account and date. Gross minus deductions is the **net pay**. |
| **Pay window** | The days from one payday up to the day before the next one. |
| **Safe to spend** | What the user can spend in the current pay window after setting aside every bill due in the next 12 months (SAFE-5). |

## 6. Period rules

| Period | Definition |
|---|---|
| Day | One calendar date (00:00 to 23:59 local time) |
| Week | **Monday to Sunday** |
| Month | **Calendar month** (the 1st to the last day) by default. A "month start day" setting (1 to 28) allows payday cycles, e.g. the 25th to the 24th. |
| Year | January 1 to December 31 |
| Custom | Any start and end date the user picks (both days included) |

The user can move to the previous or next period with ◀ ▶ buttons and jump back to the current one with a "Today" button. On a Custom period, ◀ ▶ move by the length of the range, so a 10-day range moves 10 days. ▶ stops once the next period would start after today, and the date range picker can't choose future dates.

---

## 7. Functional requirements

Priority: **P1** is in the MVP (Phase 1), **P2** is in Phase 2 (reports), **P3** is in Phase 3 (planning), **P4** is in Phase 4 (loans and forecast), and **P5** is in Phase 5 (polish).

### 7.1 Accounts

| ID | Requirement | Priority |
|---|---|---|
| ACC-1 | The user can create an account with a name, type (Cash, Bank, E-wallet, Credit Card) and an opening balance of 0 or more. | P1 |
| ACC-2 | The user can edit an account's name and opening balance. | P1 |
| ACC-3 | The user can archive and unarchive an account. An archived account is hidden from pickers, but its history still appears in reports. Accounts that have transactions cannot be deleted, only archived; unused accounts can be deleted. The last active account cannot be archived or deleted. Archived accounts still count toward the net balance, so the app warns when an account being archived has a balance other than 0. | P1 |
| ACC-4 | Each account shows its current balance: opening balance + income − expenses ± transfers. | P1 |
| ACC-5 | The dashboard shows a **net balance**: the sum of all asset accounts minus the amount owed on credit cards. | P1 |

### 7.2 Credit card

| ID | Requirement | Priority |
|---|---|---|
| CC-1 | A Credit Card account has three extra fields: **credit limit**, **statement closing day** (1–31) and **payment due day** (1–31). | P1 |
| CC-2 | Its balance is shown as **amount owed**: opening amount owed + expenses − payments − refunds. | P1 |
| CC-3 | A purchase is recorded as an **expense on the card on the day it is made**, so it shows in cashflow straight away. | P1 |
| CC-4 | Paying the bill is recorded as a **transfer** from an asset account to the card. It lowers the amount owed and is not counted as an expense. | P1 |
| CC-5 | A refund or cashback is recorded on the card with a "Refund" income category. It lowers the amount owed and offsets net cashflow. It counts as income, so it doesn't reduce the expense total or the category breakdown. | P1 |
| CC-6 | The card's detail view shows: amount owed, available credit (limit − owed), % of the limit used, spending in the current statement cycle, and the next due date. | P1 |
| CC-7 | When the payment due date is 5 days away or closer, the dashboard shows a reminder banner with the amount owed. | P3 |
| CC-8 | The data model supports any number of cards (v1 is used with one). | P1 |
| CC-9 | A **Pay bill** button on the card's details opens a transfer to the card, pre-filled with the **full amount owed**. The user can edit the amount for a partial payment. If nothing is owed, the amount starts empty. | P1 |

**Due-day rule:** if the statement or due day is greater than the number of days in a month (e.g. 31 in February), the last day of that month is used.

### 7.3 Categories

| ID | Requirement | Priority |
|---|---|---|
| CAT-1 | The app comes with preset categories (see Appendix A). | P1 |
| CAT-2 | The user can add, rename, recolor, change the icon of, archive and unarchive categories. Icons come from a curated list of about 40. The last active category of each kind cannot be archived or deleted. | P1 |
| CAT-3 | Categories are either Income or Expense. A transaction can only use a category of the matching kind. | P1 |
| CAT-4 | Categories that have transactions cannot be deleted, only archived; unused categories can be deleted. The app may offer to move those transactions to another category and then delete the original. | P1 |

### 7.4 Transactions

| ID | Requirement | Priority |
|---|---|---|
| TXN-1 | The user can add a transaction with these fields: **type**, **amount** (must be > 0), **date** (date only, defaults to today, may be in the future from Phase 3: TXN-8), **account** (defaults to the account used most recently), **category** (Income/Expense only), **to-account** (Transfer only), and optionally **payee/source** and **payment method** (Income/Expense only) and **note**. Spending past a card's limit or below 0 in an account is allowed, because the app records what really happened. | P1 |
| TXN-2 | The user can edit, delete (after confirming) and duplicate a transaction. A duplicate opens the form pre-filled with **today's date**. | P1 |
| TXN-3 | The transaction list is grouped by date, newest first, with a total for each day. | P1 |
| TXN-4 | The list can be filtered by period, account, category, type and payment method, and searched by text (payee or note). | P1 |
| TXN-5 | A quick-add form opens with **Ctrl+N** from anywhere in the app. The form can be completed using only the keyboard. | P1 |
| TXN-6 | The payee field suggests previously used payees as the user types. Choosing one fills in the category last used with that payee. | P2 |
| TXN-7 | Payment method is optional. Its values come from an editable list (preset: Cash, Debit Card, Credit Card, Bank Transfer, E-wallet, Other), and the field can be hidden in Settings (SET-6). Methods can be reordered, archived and unarchived, and unused ones can be deleted. | P1 |
| TXN-8 | Future-dated transactions are allowed, appear as "upcoming", and do not count toward the current balance or any total until their date arrives. Before Phase 3, a future date was rejected. | P3 |

### 7.5 Dashboard

| ID | Requirement | Priority |
|---|---|---|
| DSH-1 | A period switcher (Day / Week / Month / Year / Custom) with ◀ ▶ and Today buttons. | P1 |
| DSH-2 | Summary cards: **Total income**, **Total expense** and **Net cashflow** for the selected period. | P1 |
| DSH-3 | Each summary card shows the % change from the previous period, using the comparison rule in RPT-4. | P2 |
| DSH-4 | A list of account balances and the net balance. | P1 |
| DSH-5 | A credit card summary: amount owed, available credit and due date. | P1 |
| DSH-6 | Budget progress bars for the current period. | P3 |
| DSH-7 | Upcoming recurring transactions for the next 7 days. | P3 |
| DSH-8 | The 5 most recent transactions. | P1 |

### 7.6 Reports

| ID | Requirement | Priority |
|---|---|---|
| RPT-1 | **Income vs expense over time** as a bar or line chart. The bars are grouped by period: Week → 7 days, Month → each day, Year → 12 months, Custom → by day if the range is ≤ 31 days, by week if ≤ 6 months, otherwise by month. | P2 |
| RPT-2 | **Category breakdown**: a donut chart and a list ranked by amount and %. The user can switch between expenses and income. | P2 |
| RPT-3 | Clicking a category in the breakdown opens the transaction list filtered to that category and period. | P2 |
| RPT-4 | **Period comparison**: the current period next to the previous one (e.g. this month vs last month, this year vs last year), with the difference in amount and % for income, expense, net and each category. | P2 |
| RPT-4a | **Comparison rule:** while the current period is still in progress, it is compared with the **same number of days** at the start of the previous period (e.g. Oct 1–26 vs Sep 1–26, or Mon–Wed vs last Mon–Wed). If the previous period is shorter, the comparison stops at its last day. A toggle switches to comparing against the **full** previous period. Periods that are already over are always compared in full. A Custom period's previous period is the range of the same length just before it. | P2 |
| RPT-5 | Reports can be filtered by account. | P2 |
| RPT-6 | Transfers are never included in income or expense figures. | P1 |

### 7.7 Recurring transactions

| ID | Requirement | Priority |
|---|---|---|
| REC-1 | The user can create a recurring rule from a transaction template, with a frequency (daily, weekly, monthly, yearly), an interval (every *N*), a start date, and an optional end date or number of times. | P3 |
| REC-2 | Monthly rules on the 29th–31st fall on the last day of shorter months. | P3 |
| REC-3 | Each rule has a mode: **Auto-post** (the transaction is created automatically) or **Confirm** (it appears in a "Due" list to approve, edit or skip). | P3 |
| REC-4 | When the app starts, it creates or queues every occurrence that became due since it was last opened. No occurrence is ever missed or created twice. | P3 |
| REC-5 | The user can pause, edit or delete a rule. Editing a rule affects future occurrences only. | P3 |

### 7.8 Budgets

| ID | Requirement | Priority |
|---|---|---|
| BUD-1 | The user can set a spending limit for one expense category or for overall spending, for a weekly, monthly or yearly period. | P3 |
| BUD-2 | Each budget shows spent, remaining and % used for the current period. | P3 |
| BUD-3 | The progress bar turns amber at **80%** and red at **100%** or more. | P3 |
| BUD-4 | Unused budget never rolls over to the next period. | P3 |

### 7.9 Savings goals

| ID | Requirement | Priority |
|---|---|---|
| GOL-1 | The user can create a goal with a name, target amount and optional target date. | P3 |
| GOL-2 | Every goal is linked to one account, e.g. a dedicated savings account, and progress is that account's balance. Manual contributions aren't supported. | P3 |
| GOL-3 | Each goal shows % complete, the amount remaining and, if it has a target date, how much must be saved per month to reach it. | P3 |
| GOL-4 | A completed goal is marked as achieved and can be archived. | P3 |

### 7.10 Backup, restore and export

This section is in Phase 2 (see §11), except BAK-5, which is in the MVP because automatic updates (§7.13) can carry a database migration.

| ID | Requirement | Priority |
|---|---|---|
| BAK-1 | **Back up now** button: saves a copy of the database with a timestamp in the file name (e.g. `jobertsvault-2026-10-26_0930.db`) to a folder the user chooses. | P2 |
| BAK-2 | **Automatic daily backup**, made the first time the app is opened each day. | P2 |
| BAK-3 | Keeps the newest **30** automatic backups and deletes older ones. Manual backups are never deleted automatically. | P2 |
| BAK-4 | **Restore**: the user picks a backup file. The app checks that it is a valid database, makes a safety backup of the current data, then replaces it. | P2 |
| BAK-5 | A backup is always made automatically before a database migration (an app update that changes the schema). If the backup fails, the migration doesn't run and the app shows the startup error screen. | P1 |
| BAK-6 | **Export to CSV**: transactions for any period, with columns for date, type, amount, account, to-account, category, payee, payment method and note. | P2 |
| BAK-7 | Settings shows the date of the last backup. If the last successful backup is more than 7 days old, the app shows a warning. | P2 |

### 7.11 Settings

| ID | Requirement | Priority |
|---|---|---|
| SET-1 | Currency symbol and its position, and the number of decimal places (default 2, 0 allowed). | P1 |
| SET-2 | First day of week (default **Monday**) and month start day (default **1**). | P1 |
| SET-3 | Backup folder location and whether automatic backups are on. | P2 |
| SET-4 | Category and payment method management. | P1 |
| SET-5 | A switch for light, dark or system theme. Until it arrives, the app follows the Windows setting. | P5 |
| SET-6 | Show or hide the payment method field (shown by default). | P1 |
| SET-7 | Default comparison mode for RPT-4a: same number of days so far (default) or full previous period. | P2 |
| SET-8 | A **Reduce transparency** switch in Settings → Appearance makes every glass panel solid and turns off blur for this app, even when Windows transparency effects are on. | P5 |

### 7.12 First launch

| ID | Requirement | Priority |
|---|---|---|
| ONB-1 | On first launch, a plain setup form asks for the currency (₱ preselected, symbol before the amount, 2 decimals) and lets the user add their first accounts: bank, e-wallet and credit card with its limit, statement day and due day. Cash already exists. The form can be skipped with "Set up later". Closing the app without finishing shows the form again on the next launch. | P1 |
| ONB-2 | The setup is polished into a step-by-step onboarding with a progress indicator and a short tour of quick-add and the period bar. | P5 |

### 7.13 Updates

| ID | Requirement | Priority |
|---|---|---|
| UPD-1 | When automatic updates are on (the default), the app checks for a newer version at each start. If there is one, it shows "Updating Jobert's Vault to X…", installs it and reopens on the new version, with nothing for the user to do. | P1 |
| UPD-2 | If the check fails or there's no internet, the app starts normally and shows no error. The check gives up after **3 seconds**. | P1 |
| UPD-3 | Settings shows the current version, a **Check for updates** button that says "You're up to date" or names the newer version with an **Update now** button, and a switch for automatic updates. When the switch is off, the app never connects by itself. | P1 |
| UPD-4 | Every update is signed. The app refuses an update whose signature doesn't match, or one signed for a different version than announced. If an update is refused or fails, the app says why and nothing changes. | P1 |
| UPD-5 | An update never changes or removes the user's data. Any migration it brings is protected by BAK-5. | P1 |
| UPD-6 | The startup error screen also has a **Check for updates** button, so a newer version can fix an app that can't open its database. | P1 |
| UPD-7 | Development builds never check for or install updates. | P1 |

### 7.14 Loans

| ID | Requirement | Priority |
|---|---|---|
| LOAN-1 | The user can add a loan with a name, the account it's paid from, whether it's a **new loan** or **one I already have**, an optional amount borrowed and a date borrowed (required for a new loan, today by default, and optional for one the user already has). Its payments are typed one row at a time, each with a date and an amount > 0. Each new row starts one month after the previous one with the same amount, and both can be changed. On a loan the user already has, rows dated before today are saved as **paid before**: they count as paid but create no transaction, so past cashflow doesn't change. | P4 |
| LOAN-2 | The user can edit a loan's name and account, and add, edit or remove its unpaid payments. A loan with any payment marked paid in the app can't be deleted, only archived. Other loans can be deleted. A loan whose payments are all paid is marked **Paid off** and can be archived. | P4 |
| LOAN-3 | **Mark paid** records an expense in the loan's category (Loan by default) from the loan's account. The date starts as today and the amount as the scheduled one. Both can be changed, for example to add a late fee. A payment can be marked paid before its date. Deleting that transaction makes the payment unpaid again. An unpaid payment never counts toward a balance, total, report or budget. | P4 |
| LOAN-4 | From 5 days before an unpaid payment's date, the dashboard shows a banner for it, such as "E-wallet loan: ₱1,000.00 due Fri, Nov 20", with a **Pay** button that opens Mark paid. If the loan's account holds less than the payment, the banner says how much to move there first. After its date, an unpaid payment shows as **Overdue** in red until it's marked paid. | P4 |
| LOAN-5 | A **Debt-free** screen shows the debt-free date (the date of the last unpaid payment) with a countdown, the total left and the total paid, and for each loan: % paid, the amount left and the next payment. | P4 |
| LOAN-6 | **Interest cost**: for a loan with an amount borrowed, the screen shows the sum of all its payments minus the amount borrowed, and the total across loans. | P4 |
| LOAN-7 | **Money freed up**: month by month until the debt-free date, how much less goes to loans than in the current month. | P4 |
| LOAN-8 | **Loan-free streak**: the number of days since the newest date borrowed of any loan. | P4 |
| LOAN-9 | **Before you borrow**: the user types a loan they're thinking about (payment amount, number of payments, day of the month and first payment date) without saving it, and sees how much later the debt-free date would be and how Safe to spend would change in each pay window. | P4 |
| LOAN-10 | Adding a **new loan** first shows the LOAN-9 result for it, then asks "Add this loan anyway?". Saving it restarts the loan-free streak. A loan the user already has skips this step. | P4 |
| LOAN-11 | The dashboard shows "Loans left: ₱X · debt-free *date*" under the net balance. Loans never change the net balance. | P4 |

### 7.15 Payroll

| ID | Requirement | Priority |
|---|---|---|
| PAY-1 | An income recurring rule can be marked **This is a payday**. A payday rule is always in Confirm mode, and its amount is the user's estimate of the net pay (SAFE-4). | P4 |
| PAY-2 | Confirming a payday opens a **payslip** form: the gross pay and any number of deduction lines, each with a deduction category and an amount > 0. The net pay is shown as the user types and must be more than 0. The form starts from the last payslip in the same income category, gross and lines included. Saving records the gross as income in the rule's category and each line as an expense, on the rule's account and the payslip's date. The rows of one payslip are edited and deleted together. | P4 |
| PAY-3 | Preset expense categories for deductions: SSS, PhilHealth, Pag-IBIG, Withholding tax, Salary loan and Other deductions. They count like any other expense. A loan repaid through payroll is only a deduction line, never a loan in §7.14. | P4 |
| PAY-4 | A payday appears in the "Recurring transactions due" list up to 3 days before its date, with **Record now**. The payslip takes the real date, and that payday is then settled, so it can't be recorded twice. | P4 |

### 7.16 Safe to spend

| ID | Requirement | Priority |
|---|---|---|
| SAFE-1 | **Pay windows:** the dates of active payday rules split time into pay windows, each from a payday up to the day before the next one. A bill due on a payday belongs to the window that starts that day. Windows use the scheduled dates, even when pay is recorded early (PAY-4). | P4 |
| SAFE-2 | **Money** is today's balance of every active Cash, Bank and E-wallet account, except an account linked to a savings goal. | P4 |
| SAFE-3 | **Bills** are everything due from those accounts in the next 12 months: unpaid loan payments, future-dated expenses, recurring dates that are due but not settled, and the coming dates of recurring expense rules. Each credit card's statement balance counts as a bill on its due date, and a card purchase, including a recurring one, goes into the statement it falls in. An overdue loan payment or an unsettled recurring date counts as due today. Future-dated income and other recurring income count as money coming in. Transfers between counted accounts don't count, and a transfer to a card is covered by its statement balance. | P4 |
| SAFE-4 | **Expected pay:** a future payday counts as the lowest net pay of the last 3 payslips in its income category. Until there are 3, the rule's estimated amount counts too, and the lowest of them is used. | P4 |
| SAFE-5 | **Safe to spend** for the current pay window is money plus expected pay minus bills, less whatever must be held back so that no pay window in the next 12 months runs short. If even that isn't enough, it shows ₱0 and "Short by ₱X on *date*". | P4 |
| SAFE-6 | A **Safe to spend** card on the dashboard shows the amount until the next payday, the amount per day (over the days left in the window, counting today), and how much is held back and for which pay window, with a link to the Forecast screen. With no payday rule, it says how to set one up. | P4 |
| SAFE-7 | A **Forecast** screen lists each pay window in the next 12 months, or each month, with a **Pay windows / Months** switch (months follow the month start day). Each row shows the pay (expected or received), the bills, what's left, and a warning on a window that would run short. Opening a row lists its bills. | P4 |

---

## 8. Non-functional requirements

| Area | Requirement |
|---|---|
| **Privacy** | Financial data never leaves the computer. No analytics or telemetry. The only network requests are for app updates from this project's GitHub releases (§7.13), and turning automatic updates off stops them. The app works fully offline. The local error log never records amounts, payees or notes. |
| **Data integrity** | Money is stored as **integers in minor units** (e.g. cents), never as floating-point numbers. Every write is wrapped in a database transaction, so a crash can't leave data half-saved. |
| **Performance** | Cold start in < 2 s, not counting the time the update check waits on a slow connection (up to 3 s, UPD-2). With 50,000 transactions, each dashboard or report view, including the Debt-free and Forecast screens, loads in < 300 ms. |
| **Architecture** | All business logic (balances, period totals, comparisons, recurring rules) and all database access live in the **Rust backend**. The React frontend only displays data and calls backend commands. It never runs SQL itself. |
| **Storage location** | The database is stored in the user's app data folder, at a path based on the app's identifier (e.g. `%APPDATA%\com.jobertsvault.app\`). |
| **Platform** | Windows 10 and 11, 64-bit. Distributed as an `.exe` installer on GitHub Releases. Installed copies then update themselves (§7.13). |
| **Window** | A **fixed 1440 × 900** window that can't be resized or maximized (minimize and close only). It opens centered on the main monitor. It's sized for the owner's 1920 × 1080 and 2560 × 1080 monitors at 100% display scaling. If the monitor's usable area is too small (for example, if display scaling is raised), it opens at 1280 × 800 instead, showing the same layout zoomed to 88.9%. |
| **Usability** | Every main action works from the keyboard. Amounts use thousands separators, and negative or expense values are clearly distinguished by more than color alone. |
| **Reliability** | Recurring transactions and backups resume correctly after the app is closed for any length of time. |

## 9. Example user stories

1. *As a user, I press Ctrl+N, type "250", Tab to "Food", press Enter, and the expense is saved for today to the account I used last.*
2. *As a user, I open the Month view and see straight away that I spent 12% more than last month, mostly on Dining Out.*
3. *As a user, I pay my credit card bill from my bank account, and my cashflow doesn't count the purchases twice.*
4. *As a user, my salary is added automatically on the 25th, and I confirm my internet bill each month before it is recorded.*
5. *As a user, after my laptop's drive fails, I install the app on a new PC, restore my latest backup from OneDrive, and everything is back.*
6. *As a user, a banner reminds me 5 days before each loan payment, and one I haven't marked paid turns red, so I never miss a payment until every loan is paid off.*
7. *As a user, on payday I record my payslip and see what's safe to spend until the next payday, with money already held back for a short pay window next month.*
8. *As a user, before I take a new loan I try it in Before you borrow and see my debt-free date move months later.*

## 10. Acceptance criteria for the MVP (Phase 1)

- [x] I can create cash, bank, e-wallet and credit card accounts, each with an opening balance.
- [x] I can add, edit, delete and duplicate income, expense and transfer transactions.
- [x] Account balances and the amount owed on the card are always correct after any add, edit or delete.
- [x] The dashboard shows correct income, expense and net totals for Day, Week (Mon–Sun), Month, Year and Custom, and moving between periods works.
- [x] Transfers, including paying the card bill, never appear in income or expense totals.
- [x] I can filter and search the transaction list.
- [x] A future date is rejected with a clear message. (Phase 1 only: Phase 3 allows future dates, TXN-8.)
- [x] Ctrl+N opens quick-add from any screen, and I can log a typical transaction with only the keyboard in 5 seconds or less.
- [x] **Pay bill** opens a transfer pre-filled with the full amount owed, and a partial payment works.
- [x] A card's details show the amount owed, available credit, % of the limit used, cycle spending and next due date.
- [x] A refund on the card lowers the amount owed and counts as income.
- [x] I can manage categories and payment methods: add, edit, reorder, archive, unarchive and delete unused ones, plus merge categories.
- [x] Archiving an account with a balance other than 0 shows a warning, and my last active account can't be archived or deleted.
- [x] Settings for currency, week start, month start day and the payment method field work, and decimals lock once an amount is saved.
- [x] The first-launch form works, can be skipped, and comes back only if the app was closed before finishing it.
- [x] The app follows the Windows light or dark setting, and stays readable with Windows transparency effects off.
- [x] Cold start takes under 2 s, and the dashboard and transaction list load in under 300 ms with 50,000 test transactions.
- [x] The dashboard and transaction list scroll smoothly on both monitors.
- [x] The app runs fully offline on Windows 11, and starts normally with no internet.
- [x] An installed copy updates itself to a newer published version at its next start, and all my data is still there afterward.
- [x] With automatic updates off, the app doesn't connect by itself, and **Check for updates** in Settings still works.
- [x] A tampered update is refused, and nothing changes.
- [x] An update with a migration makes a backup of the database first, and a failed backup stops the migration.

## 11. Release plan

> **Data safety before Phase 2** (history: Phase 2 shipped in `v0.2.0` on 2026-11-07, and the app now makes its own backups, §7.10): the MVP only backs up the database before a migration (BAK-5). It has no regular backups and no Restore. Until Phase 2 ships, close the app now and then and copy `jobertsvault.db`, `jobertsvault.db-wal` and `jobertsvault.db-shm` from the app data folder by hand, all three together: recent changes can be kept only in the `-wal` file.

| Phase | Scope |
|---|---|
| **1: MVP** | Accounts (including the credit card), categories, transactions, dashboard totals by period, settings, basic first-launch setup, light and dark themes that follow Windows, automatic updates, a database backup before migrations |
| **2: Reports & backups** | Charts over time, category breakdown, period comparison, % change, payee suggestions, regular backups, restore, CSV export |
| **3: Planning** | Recurring transactions, budgets, savings goals, card due-date reminder, future-dated transactions |
| **4: Loans & forecast** | Loans with payment reminders and the Debt-free screen, payday rules and payslips with deductions, Safe to spend and the Forecast screen |
| **5: Polish** | The theme switch, the Reduce transparency switch, more keyboard shortcuts, polished first-launch onboarding, Windows installer polish |

## 12. How we'll measure success

- Logging a typical transaction takes **≤ 5 seconds**.
- Account balances in the app match the real bank and card statements at the end of each month.
- The app is used daily for at least 3 months without any data loss.

## 13. Open questions

None at the moment. All earlier questions have been answered and are recorded in §14.

## 14. Decision log

| Date | Decision |
|---|---|
| 2026-10-26 | Windows desktop app, used by one person, with data stored locally only |
| 2026-10-26 | Stack: Tauri 2, with a React + TypeScript frontend, a Rust backend that owns all logic and database access, and SQLite |
| 2026-10-26 | Transactions are entered by hand only, in a single currency |
| 2026-10-26 | Weeks start on Monday, and months are calendar months (the start day can be changed) |
| 2026-10-26 | Credit card purchases are counted as expenses when made, and paying the bill is a transfer |
| 2026-10-26 | App name: **Jobert's Vault** |
| 2026-10-26 | Payment method stays as an optional field and can be hidden |
| 2026-10-26 | Periods still in progress are compared with the same number of days of the previous period, with an option to compare against the full period |
| 2026-10-26 | Transactions record a date only, with no time |
| 2026-10-26 | Savings goals must be linked to an account |
| 2026-10-26 | Budgets never roll over |
| 2026-10-26 | Split transactions are not needed |
| 2026-10-26 | Quick-add defaults to the account used most recently |
| 2026-10-26 | Backup, restore and CSV export moved out of the MVP, into Phase 2 |
| 2026-10-26 | Refunds count as income: they offset net cashflow, not the expense total |
| 2026-10-26 | Archived accounts still count toward the net balance, and archiving one with a balance other than 0 shows a warning |
| 2026-10-26 | Currency: Philippine peso, shown with the official sign **₱** (U+20B1), before the amount, with 2 decimal places |
| 2026-10-26 | Dates always use US order (`10/26/2026`, `Oct 26`), whatever the Windows region setting |
| 2026-10-26 | Category colors are numbered separately for income and expense, so the 12 most common expense categories each have their own color (DATA_MODEL §7) |
| 2026-10-26 | Logo: **C · Line Door**, a vault door drawn in white lines on an indigo aurora tile (`assets/brand/logo.svg`) |
| 2026-10-26 | Added SET-8: a Reduce transparency switch (Phase 4) |
| 2026-10-26 | Added ONB-1: a basic first-launch setup in Phase 1, polished in Phase 4 (ONB-2) |
| 2026-10-26 | A duplicated transaction uses today's date |
| 2026-10-26 | Added CC-9: Pay bill suggests the full amount owed, editable |
| 2026-10-26 | The window is a fixed 1440 × 900 and can't be resized or maximized |
| 2026-10-26 | Visual style: **Glassmorphism 2.0**, with frosted glass panels over a soft colored "aurora" background, the Sora font and a deep indigo accent (DESIGN_SYSTEM.md) |
| 2026-11-01 | Phase 1 rejects future dates. Phase 3 lifts the rule when it adds TXN-8. |
| 2026-11-01 | On a Custom period, ◀ ▶ move by the length of the range, and comparisons use the same-length range just before it |
| 2026-11-01 | The app follows the Windows light or dark setting from Phase 1. Phase 4 adds only the switch. |
| 2026-11-01 | The 1280 × 800 fallback shows the same layout zoomed to 88.9%, with no second layout |
| 2026-11-01 | Opening balances are entered as 0 or more |
| 2026-11-01 | Unused accounts, categories and payment methods can be deleted. The last active account, and the last active category of each kind, can't be archived or deleted. |
| 2026-11-01 | Transfers have no payee or payment method |
| 2026-11-01 | Going past a card's limit or below 0 in an account is allowed, with no warning |
| 2026-11-01 | PRD §10 is expanded into the complete Phase 1 release checklist |
| 2026-11-01 | From Phase 3, future-dated transactions stay out of every total until their date arrives |
| 2026-11-01 | ▶ stops at the current period, and the date range picker can't choose future dates |
| 2026-11-01 | Development builds use a separate database, `jobertsvault-dev.db`, so they can't touch real data |
| 2026-11-01 | The installer's product name uses a curly apostrophe (`Jobert’s Vault`), because a straight one breaks the installer build |
| 2026-11-03 | Added automatic updates (UPD-1 to UPD-7) to the MVP, with the same release process as NBSBuddy. This reverses "no network" for updates only ([ADR 0010](decisions/0010-automatic-updates.md)). |
| 2026-11-03 | The pre-migration backup (BAK-5) moved from Phase 2 into the MVP, because updates can carry migrations |
| 2026-11-03 | When automatic updates are off, the app never connects by itself. This differs from NBSBuddy, which still checks. |
| 2026-11-03 | The update check at startup gives up after 3 seconds, rather than NBSBuddy's 8, to keep startup fast |
| 2026-11-03 | NBSBuddy's "What's new" tour and download page are not part of the MVP |
| 2026-11-03 | The preset expense categories follow DATA_MODEL §7's order: the 12 common ones first, then the 5 rarely used ones (so Personal Care comes before Education) |
| 2026-11-04 | Today on a Custom period jumps to the range, in the same sequence, that contains today, which is where ▶ stops (DATA_MODEL §5.3) |
| 2026-11-06 | RPT-1's chart bars are calendar-aligned: a Day period is one bar, a Year is 12 calendar months, and Custom weeks follow the week start and Custom months are calendar months, cut to the range (DATA_MODEL §5.9) |
| 2026-11-06 | The dashboard's small income-vs-expense chart (UI_FLOWS §2) is built with RPT-1, from `get_dashboard` |
| 2026-11-06 | SET-7's default comparison lives in Settings → General. Reports' Compare to changes only that view, kept in the URL (UI_FLOWS §8.1) |
| 2026-11-06 | Reports' Compare to sits in the Period comparison section's header, and RPT-5's account filter beside the period bar, since all three don't fit in one row (UI_FLOWS §8.1, §11) |
| 2026-11-06 | TXN-6's payee suggestions match the form's type, and fill in the category of the latest row of that type with the payee, unless that category is archived now (DATA_MODEL §3.4) |
| 2026-11-07 | Backups, restore and CSV export live in Settings → Backup (`/settings?tab=backup`). Export to CSV takes its own From and To dates there (UI_FLOWS §6) |
| 2026-11-07 | A restore keeps this computer's backup settings (the folder, the automatic backup switch and the last backup dates). Every other setting comes from the backup (DATA_MODEL §9) |
| 2026-11-07 | BAK-7's warning counts manual and automatic backups, compares local dates (more than 7 days before today), and also shows when there's never been a backup. The daily automatic backup runs on its own thread, so a slow backup folder never delays the window |
| 2026-11-11 | TXN-8: the transaction list's Upcoming section shows every future-dated row that matches the other filters, whatever its date, whenever the selected period hasn't ended, because ▶ stops at the current period. The summary line and day nets leave Upcoming out, and its days show no net (UI_FLOWS §3) |
| 2026-11-13 | Added Phase 4, Loans & forecast (`v0.4.0`): loans (LOAN-1 to LOAN-11), payroll (PAY-1 to PAY-4) and Safe to spend (SAFE-1 to SAFE-7). Polish moves to Phase 5 (`v1.0.0`), and its requirements become P5 |
| 2026-11-13 | A loan is a list of scheduled payments, not an account. Payments stay expenses in the Loan category, and an unpaid payment counts toward nothing until it's marked paid (ADR 0011) |
| 2026-11-13 | Loans are kept one per lender app and entered one payment row at a time. On a loan the user already has, past rows are saved as "paid before", with no transaction |
| 2026-11-13 | Loan reminders start 5 days before a payment, and an unpaid payment turns Overdue after its date. A payment can be marked paid early |
| 2026-11-13 | What's left on loans shows beside the net balance and never changes it, because it includes interest |
| 2026-11-13 | A new loan shows the Before you borrow result and asks to confirm. A loan the user already has doesn't, and doesn't restart the streak |
| 2026-11-13 | Payslips record the gross pay as income and each deduction as an expense in normal expense categories. A loan repaid through payroll is only a deduction line |
| 2026-11-13 | Paydays are income recurring rules marked as paydays, always in Confirm mode, and can be recorded up to 3 days early |
| 2026-11-13 | Safe to spend looks 12 months ahead and holds back money now for any pay window that would run short |
| 2026-11-13 | Expected pay is the lowest net of the last 3 payslips in the same income category. The rule's estimate also counts until there are 3 |
| 2026-11-13 | Safe to spend counts Cash, Bank and E-wallet accounts except those linked to a savings goal. A credit card counts as its statement balance on its due date |
| 2026-11-13 | Safe to spend shows on a dashboard card and on a Forecast screen with Pay windows and Months views |
| 2026-11-13 | A new loan is always added as a loan of its own, even from a lender app that already has one, so it goes through Before you borrow and restarts the streak (DATA_MODEL §4.5) |
| 2026-11-13 | Only a paid-off loan can be archived, so an archived loan never hides money still owed (DATA_MODEL §4.5) |
| 2026-11-13 | LOAN-4's banners are one per loan, for its next unpaid payment, saying how many others are overdue (UI_FLOWS §2) |
| 2026-11-13 | Before you borrow's payments are monthly from the first payment date, so its day of the month is the first payment's (DATA_MODEL §5.16) |
| 2026-11-13 | PAY-4's early paydays can also be recorded from the Safe to spend card, and the Due banner counts only dates that are due (UI_FLOWS §2, F10) |
| 2026-11-13 | In the forecast, every unpaid loan payment counts whatever its account, transfers to or from a card don't count (the card's bills stand in for them), and pay counts only into a counted account (DATA_MODEL §5.13–5.15) |

---

## Appendix A: Preset categories

**Income:** Salary, Bonus, Freelance/Side income, Interest, Gifts received, Refund, Other income

**Expense:** Food & Groceries, Dining Out, Transportation, Rent/Housing, Utilities (electricity, water, internet), Phone, Shopping, Health & Medical, Insurance, Entertainment, Subscriptions, Personal Care, Education, Gifts & Donations, Travel, Fees & Charges, Other expense

**Added in Phase 4 (expense):** Loan (the default category for loan payments, LOAN-3), and the deduction categories SSS, PhilHealth, Pag-IBIG, Withholding tax, Salary loan and Other deductions (PAY-3). An existing database gets any of these it doesn't already have.
