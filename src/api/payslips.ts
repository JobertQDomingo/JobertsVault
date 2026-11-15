import type { Payslip } from "@/bindings/Payslip";
import type { PayslipDefaults } from "@/bindings/PayslipDefaults";
import type { PayslipFields } from "@/bindings/PayslipFields";
import type { PayslipNet } from "@/bindings/PayslipNet";
import type { Transaction } from "@/bindings/Transaction";
import { call } from "./client";

/** Where the form starts: the rule's last payslip, else its category's, else its amount (PAY-2). */
export const getPayslipDefaults = ({
  ruleId,
  date,
}: {
  ruleId: number;
  date: string;
}) => call<PayslipDefaults>("get_payslip_defaults", { ruleId, date });

/** The net pay as the user types, so the frontend never adds up money (F14). */
export const previewPayslip = (payslip: PayslipFields) =>
  call<PayslipNet>("preview_payslip", { payslip });

/** Settles the payday on `date`: a due one, or one 1–3 days ahead (PAY-4). The gross row. */
export const recordPayslip = ({
  ruleId,
  date,
  payslip,
}: {
  ruleId: number;
  date: string;
  payslip: PayslipFields;
}) => call<Transaction>("record_payslip", { ruleId, date, payslip });

/** The whole payslip, from any of its rows' ids. */
export const getPayslip = (id: number) => call<Payslip>("get_payslip", { id });

/** Edits the gross row in place and replaces the deductions (DATA_MODEL §4.7). */
export const updatePayslip = ({
  id,
  payslip,
}: {
  id: number;
  payslip: PayslipFields;
}) => call<Payslip>("update_payslip", { id, payslip });
