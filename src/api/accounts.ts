import type { Account } from "@/bindings/Account";
import type { AccountList } from "@/bindings/AccountList";
import type { AccountFields } from "@/bindings/AccountFields";
import type { CardSpend } from "@/bindings/CardSpend";
import type { CardSpendPreview } from "@/bindings/CardSpendPreview";
import type { NewAccount } from "@/bindings/NewAccount";
import { call } from "./client";

/** Every account, archived included, in display order, and the net balance. */
export const listAccounts = () => call<AccountList>("list_accounts");

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

/**
 * The spending level check before saving an expense on a card (CC-11, DATA_MODEL §5.18): none
 * when the save asks nothing.
 */
export const previewCardSpend = (spend: CardSpend) =>
  call<CardSpendPreview | null>("preview_card_spend", { spend });

/** Each id's position becomes its sort order. */
export const reorderAccounts = (ids: number[]) =>
  call<null>("reorder_accounts", { ids });

/**
 * Archives the account. A balance other than 0 is a `conflict` unless the user has
 * confirmed the warning (UI_FLOWS F6). The last active account is always a `conflict`.
 */
export const archiveAccount = ({
  id,
  acknowledgeBalance,
}: {
  id: number;
  acknowledgeBalance: boolean;
}) => call<null>("archive_account", { id, acknowledgeBalance });

export const unarchiveAccount = (id: number) =>
  call<null>("unarchive_account", { id });

/** Only an account no transaction, recurring rule or goal uses (`inUse` false) can be deleted. */
export const deleteAccount = (id: number) =>
  call<null>("delete_account", { id });
