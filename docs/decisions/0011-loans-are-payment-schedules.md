# 0011. Loans are lists of scheduled payments, not accounts

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-11-13 |
| **Related** | PRD §7.14 (LOAN-1 to LOAN-11), DATA_MODEL §4.5–4.6, §5.11 and §5.15, ADR 0006 |

## Context

The owner repays several loans from lender apps. Each has its own schedule of exact amounts on exact dates, and the amounts are uneven: they differ by a few centavos, and they step down as stacked loans finish. The lenders show what to pay and when, but not how much of it is interest.

Before Phase 4, those payments were entered as future-dated transactions and recurring rules in a "Loan" category. That can't keep anyone from missing a payment: a future-dated transaction becomes a real one on its date whether or not it was paid (TXN-8), so the app shows it as paid either way. Nothing groups the payments into a loan, adds up what's left or says when the last one is due.

## Decision

- **A loan is a list of scheduled payments** (a date and an amount each), kept in `loans` and `loan_payments`. It isn't an account and has no balance.
- **An unpaid payment is not a transaction.** It counts toward no balance, total, report or budget. **Mark paid** records an ordinary expense in the loan's category (Loan to start with), and deleting that expense makes the payment unpaid again.
- **What's left on a loan** is the sum of its unpaid payments, interest included. It's shown beside the net balance, never in it (LOAN-11).
- **The forecast counts every unpaid payment** as a bill on its date (SAFE-3).

## Alternatives considered

| Option | Why not |
|---|---|
| **A liability account, like the credit card (ADR 0006)** | Each payment would become a transfer, so loan payments would vanish from cashflow, although the owner records them as spending. The account would need an opening balance of principal owed, which the lender apps don't show, and every payment would need splitting into interest and principal (a PRD §3 non-goal). |
| **Future-dated transactions and recurring rules, as before** | They count as paid on their date whether or not they were, so the app can't remind, warn or show Overdue. Nothing ties them to a loan. |
| **One recurring rule per loan** | A rule repeats one amount on one schedule. Real loan schedules have uneven amounts, several payment days a month and stepped totals. |

## Consequences

- **Good:** payments stay in cashflow as expenses, as the owner already records them. Unpaid is a real state, so reminders, Overdue and the debt-free date work. The forecast can count each payment on the day it's due.
- **Trade-off:** what's left includes future interest, so it isn't a true liability. That's why it stays out of the net balance.
- **Rule:** an unpaid payment never changes a balance or total. A service test guards this, and another checks that deleting a payment's expense makes it unpaid again.
- **The money received from a new loan isn't part of the loan.** The user records it as income where it landed, in any income category, so the account's balance stays right. Over a loan's life, its payments minus that income is what it cost.
- **Loans repaid through payroll aren't loans here.** They're a deduction line on the payslip (PAY-3).
