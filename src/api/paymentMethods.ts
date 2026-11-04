import type { PaymentMethod } from "@/bindings/PaymentMethod";
import { call } from "./client";

/** Every payment method, archived included, in display order. */
export const listPaymentMethods = () =>
  call<PaymentMethod[]>("list_payment_methods");

export const createPaymentMethod = (name: string) =>
  call<PaymentMethod>("create_payment_method", { name });

export const updatePaymentMethod = ({
  id,
  name,
}: {
  id: number;
  name: string;
}) => call<PaymentMethod>("update_payment_method", { id, name });

/** Each id's position becomes its sort order. */
export const reorderPaymentMethods = (ids: number[]) =>
  call<null>("reorder_payment_methods", { ids });

export const archivePaymentMethod = (id: number) =>
  call<null>("archive_payment_method", { id });

export const unarchivePaymentMethod = (id: number) =>
  call<null>("unarchive_payment_method", { id });

/** Only a method no transaction uses (`inUse` false) can be deleted. */
export const deletePaymentMethod = (id: number) =>
  call<null>("delete_payment_method", { id });
