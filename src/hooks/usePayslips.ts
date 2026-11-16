import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { useEffect, useState } from "react";
import {
  getPayslip,
  getPayslipDefaults,
  previewPayslip,
  recordPayslip,
  updatePayslip,
} from "@/api/payslips";
import type { PayslipFields } from "@/bindings/PayslipFields";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

export const usePayslipDefaults = (
  ruleId: number,
  date: string,
  enabled = true,
) =>
  useQuery({
    queryKey: queryKeys.payslipDefaults(ruleId, date),
    queryFn: () => getPayslipDefaults({ ruleId, date }),
    enabled,
  });

export const usePayslip = (id: number | undefined) =>
  useQuery({
    queryKey: queryKeys.payslip(id ?? 0),
    queryFn: () => getPayslip(id ?? 0),
    enabled: id !== undefined,
  });

/** The net for what's typed, asked 250 ms after the typing stops; the last answer stays meanwhile. */
export function usePayslipNet(payslip: PayslipFields) {
  const [asked, setAsked] = useState(payslip);
  const key = JSON.stringify(payslip);
  useEffect(() => {
    const timer = setTimeout(
      () => setAsked(JSON.parse(key) as PayslipFields),
      250,
    );
    return () => clearTimeout(timer);
  }, [key]);
  return useQuery({
    queryKey: queryKeys.payslipNet(asked),
    queryFn: () => previewPayslip(asked),
    // An empty gross goes as 0 and has no net worth asking about; its ₱0.00 answer would show,
    // with its error, while the first gross is typed.
    enabled: asked.gross > 0,
    placeholderData: keepPreviousData,
  });
}

// Ledger writes: a payslip's rows are transactions, and it settles a payday.
export const useRecordPayslip = () => useLedgerMutation(recordPayslip);
export const useUpdatePayslip = () => useLedgerMutation(updatePayslip);
