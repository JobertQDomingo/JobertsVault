// Accounts, categories and payment methods offered in a list (DESIGN_SYSTEM §9.1: archived ones
// are hidden), except one that's already chosen, so its box can still name it.

export const offered = <T extends { id: number; archived: boolean }>(
  items: T[] | undefined,
  ...kept: (number | null)[]
) => (items ?? []).filter((item) => !item.archived || kept.includes(item.id));

export const named = (item: { name: string; archived: boolean }) =>
  item.archived ? `${item.name} (archived)` : item.name;

/** A URL parameter's positive whole id, or null when it's missing or malformed. */
export const idParam = (text: string | null) =>
  text && /^\d+$/.test(text) && Number(text) > 0 ? Number(text) : null;
