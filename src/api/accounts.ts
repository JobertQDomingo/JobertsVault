import type { Account } from "@/bindings/Account";
import type { AccountFields } from "@/bindings/AccountFields";
import type { NewAccount } from "@/bindings/NewAccount";
import { call } from "./client";

/** Every account, archived included, in display order. */
export const listAccounts = () => call<Account[]>("list_accounts");

export const getAccount = (id: number) => call<Account>("get_account", { id });

export const createAccount = (account: NewAccount) =>
  call<Account>("create_account", { account });

export const updateAccount = ({
  id,
  account,
}: {
  id: number;
  account: AccountFields;
}) => call<Account>("update_account", { id, account });

/** Each id's position becomes its sort order. */
export const reorderAccounts = (ids: number[]) =>
  call<null>("reorder_accounts", { ids });
