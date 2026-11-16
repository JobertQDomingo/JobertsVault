import type { Loan } from "@/bindings/Loan";
import type { LoanDraft } from "@/bindings/LoanDraft";
import type { LoanFields } from "@/bindings/LoanFields";
import type { LoanList } from "@/bindings/LoanList";
import type { LoanPreview } from "@/bindings/LoanPreview";
import type { NewLoan } from "@/bindings/NewLoan";
import type { Transaction } from "@/bindings/Transaction";
import type { TransactionFields } from "@/bindings/TransactionFields";
import { call } from "./client";

/** Every loan in the order it was added, archived ones included, with its payments' status and figures, and the Debt-free summary (LOAN-1, LOAN-5 to LOAN-8). */
export const listLoans = () => call<LoanList>("list_loans");

/** On a loan the user already has, rows dated before today are saved as paid before (LOAN-1). */
export const createLoan = (loan: NewLoan) =>
  call<Loan>("create_loan", { loan });

/** The rows replace every payment without a transaction; a paid one is kept (LOAN-2). */
export const updateLoan = ({ id, loan }: { id: number; loan: LoanFields }) =>
  call<Loan>("update_loan", { id, loan });

/** A paid-off loan only (LOAN-2). */
export const archiveLoan = (id: number) => call<null>("archive_loan", { id });

export const unarchiveLoan = (id: number) =>
  call<null>("unarchive_loan", { id });

/** Only a loan with no payment marked paid (LOAN-2). */
export const deleteLoan = (id: number) => call<null>("delete_loan", { id });

/** Records the expense confirmed and links it: only an unpaid payment (LOAN-3). */
export const markLoanPaymentPaid = ({
  paymentId,
  transaction,
}: {
  paymentId: number;
  transaction: TransactionFields;
}) => call<Transaction>("mark_loan_payment_paid", { paymentId, transaction });

/** Deletes the payment's expense, so it's unpaid again (LOAN-3). */
export const markLoanPaymentUnpaid = (paymentId: number) =>
  call<null>("mark_loan_payment_unpaid", { paymentId });

/** What a loan would do to the debt-free date, Safe to spend and each pay window. Saves nothing (LOAN-9, LOAN-10). */
export const previewLoan = (draft: LoanDraft) =>
  call<LoanPreview>("preview_loan", { draft });
