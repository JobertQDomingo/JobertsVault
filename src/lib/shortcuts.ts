/**
 * The keyboard shortcuts (UI_FLOWS §10): the one list the `?` dialog shows. Keys are named as
 * `useShortcuts` names them, and it only takes names from here, so a handler can't bind a key
 * the dialog doesn't list. Keys a dialog or field handles itself (quick-add's, the date input's,
 * Esc) are listed too, so the table is whole.
 */
export const SHORTCUTS = [
  {
    keys: ["Ctrl+N"],
    action: "Add transaction (quick-add)",
    where: "Everywhere",
  },
  { keys: ["Ctrl+F"], action: "Focus the search box", where: "Transactions" },
  {
    keys: ["Ctrl+1", "Ctrl+2", "Ctrl+3", "Ctrl+4"],
    action: "Dashboard, Transactions, Accounts, Reports",
    where: "Everywhere",
  },
  { keys: ["Ctrl+,"], action: "Settings", where: "Everywhere" },
  {
    keys: ["Alt+ArrowLeft", "Alt+ArrowRight"],
    action: "Previous / next period",
    where: "Screens with a period switcher",
  },
  {
    keys: ["Alt+Home"],
    action: "Go to today",
    where: "Screens with a period switcher",
  },
  {
    keys: ["Alt+E", "Alt+I", "Alt+T"],
    action: "Switch type to Expense / Income / Transfer",
    where: "Quick-add",
  },
  {
    keys: ["Enter", "Shift+Enter"],
    action: "Save / save and add another",
    where: "Quick-add",
  },
  {
    keys: ["ArrowUp", "ArrowDown"],
    action: "Previous / next day",
    where: "Date input",
  },
  {
    keys: ["Escape"],
    action: "Close a dialog or popover",
    where: "Everywhere",
  },
  {
    keys: ["Enter", "Alt+ArrowLeft"],
    action: "Next step (Finish on the last) / previous step",
    where: "Check-in",
  },
  {
    keys: ["ArrowUp", "ArrowDown"],
    action: "Move between the offered focuses",
    where: "Check-in, Pick a focus",
  },
  {
    keys: ["?"],
    action: "Show these shortcuts",
    where: "Everywhere, the check-in included",
  },
] as const;

export type ShortcutKey = (typeof SHORTCUTS)[number]["keys"][number];

/** UI_FLOWS §10's sentence under the table. */
export const CHECKIN_NOTE =
  "While a check-in is due, Ctrl+N and ? still work, and Ctrl+1 … Ctrl+4, Ctrl+, and Esc do nothing.";

const SHOWN: Record<string, string> = {
  ArrowLeft: "←",
  ArrowRight: "→",
  ArrowUp: "↑",
  ArrowDown: "↓",
  Escape: "Esc",
};

/** How a key reads on screen: `Alt+ArrowLeft` is `Alt+←`. */
export const showKey = (key: ShortcutKey) =>
  key
    .split("+")
    .map((part) => SHOWN[part] ?? part)
    .join("+");
