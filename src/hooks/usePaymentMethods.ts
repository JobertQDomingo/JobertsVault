import { useQuery } from "@tanstack/react-query";
import {
  archivePaymentMethod,
  createPaymentMethod,
  deletePaymentMethod,
  listPaymentMethods,
  reorderPaymentMethods,
  unarchivePaymentMethod,
  updatePaymentMethod,
} from "@/api/paymentMethods";
import { queryKeys } from "@/lib/queryKeys";
import { useLedgerMutation } from "./useLedgerMutation";

export const usePaymentMethods = () =>
  useQuery({ queryKey: queryKeys.paymentMethods, queryFn: listPaymentMethods });

export const useCreatePaymentMethod = () =>
  useLedgerMutation(createPaymentMethod);
export const useUpdatePaymentMethod = () =>
  useLedgerMutation(updatePaymentMethod);
export const useReorderPaymentMethods = () =>
  useLedgerMutation(reorderPaymentMethods);
export const useArchivePaymentMethod = () =>
  useLedgerMutation(archivePaymentMethod);
export const useUnarchivePaymentMethod = () =>
  useLedgerMutation(unarchivePaymentMethod);
export const useDeletePaymentMethod = () =>
  useLedgerMutation(deletePaymentMethod);
