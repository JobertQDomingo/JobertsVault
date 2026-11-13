import type { Loan } from "@/bindings/Loan";
import type { LoanFields } from "@/bindings/LoanFields";
import type { LoanList } from "@/bindings/LoanList";
import type { NewLoan } from "@/bindings/NewLoan";
import { call } from "./client";

/** Every loan in the order it was added, archived ones included, with its payments' status (LOAN-1). */
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
