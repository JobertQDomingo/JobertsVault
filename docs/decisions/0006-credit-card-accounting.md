# 0006. Card purchases are expenses when made; bill payments are transfers

| | |
|---|---|
| **Status** | Accepted |
| **Date** | 2026-10-26 |
| **Related** | PRD §7.2 (CC-1 to CC-9), DATA_MODEL §5.1–5.2 and §5.5 |

## Context

The owner has one credit card. Spending on it has to show up in cashflow, but paying the bill later must not count the same spending twice. The card's amount owed must also match the bank's statement.

## Decision

- **The card is an account** with a signed balance. A **negative balance means money owed**, and `amount_owed = max(0, −balance)`. In forms the user types amounts owed as positive numbers.
- **A purchase on the card is an expense on the day it's made**, so cashflow shows spending when it happens.
- **Paying the bill is a transfer** from a bank account to the card. Transfers are never income or expense, so the payment lowers the amount owed without counting the spending twice.
- **Refunds and cashback are income** in the "Refund" category, recorded on the card. They lower the amount owed and offset net cashflow, but they don't reduce the expense total.
- **Pay bill** pre-fills a transfer with the full amount owed, and the user can edit it (CC-9).

## Alternatives considered

| Option | Why not |
|---|---|
| **Count spending when the bill is paid (cash basis)** | Cashflow would lag weeks behind real spending, and you couldn't see what the card was spent on in the month you spent it. |
| **Record the bill payment as an expense** | Every purchase would be counted twice: once when made and again when paid. |
| **Refunds as negative expenses** | Breaks the "amounts are always positive" rule and complicates every total. A Refund income category is simpler and keeps net cashflow correct. |

## Consequences

- **Good:** cashflow, category breakdowns and budgets reflect real spending dates, and the amount owed matches the statement.
- **Rule:** transfers are excluded from every income and expense total. There's a required test for this: "a card payment never changes the expense total".
- **Trade-off:** because refunds count as income, a refunded purchase still appears in that category's spending. This was accepted on 2026-10-26.
- **Ready for more cards:** a second card is just another account.
