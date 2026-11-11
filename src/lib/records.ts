// Accounts, categories and payment methods offered in a list (DESIGN_SYSTEM §9.1: archived ones
// are hidden), except one that's already chosen, so its box can still name it.

export const offered = <T extends { id: number; archived: boolean }>(
  items: T[] | undefined,
  ...kept: (number | null)[]
) => (items ?? []).filter((item) => !item.archived || kept.includes(item.id));

export const named = (item: { name: string; archived: boolean }) =>
  item.archived ? `${item.name} (archived)` : item.name;

type Named = { id: number; name: string };

/** A row's name in the recurring lists: "Category · Payee", or "Bank → Card" for a transfer. */
export function rowTitle(
  t: {
    kind: string;
    accountId: number;
    toAccountId: number | null;
    categoryId: number | null;
    payee: string | null;
  },
  accounts: Named[],
  categories: Named[],
) {
  const name = (list: Named[], id: number | null) =>
    list.find((item) => item.id === id)?.name;
  return t.kind === "transfer"
    ? `${name(accounts, t.accountId) ?? ""} → ${name(accounts, t.toAccountId) ?? ""}`
    : [name(categories, t.categoryId), t.payee].filter(Boolean).join(" · ");
}

/**
 * A URL parameter's positive whole id, or null when it's missing, malformed or too big to be exact
 * (so it never reaches Rust as a number no `i64` holds).
 */
export const idParam = (text: string | null) => {
  const id = text && /^\d+$/.test(text) ? Number(text) : 0;
  return id > 0 && Number.isSafeInteger(id) ? id : null;
};
