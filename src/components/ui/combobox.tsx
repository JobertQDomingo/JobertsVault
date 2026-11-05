import { Command } from "cmdk";
import { ChevronDown } from "lucide-react";
import { Popover } from "radix-ui";
import {
  useLayoutEffect,
  useRef,
  useState,
  type ComponentProps,
  type ReactNode,
} from "react";
import { cn } from "@/lib/utils";
import { fieldClasses } from "./input";

export type ComboboxOption = { value: string; label: string; icon?: ReactNode };

type InputProps = Pick<
  ComponentProps<"input">,
  "id" | "aria-label" | "aria-invalid" | "aria-describedby" | "disabled"
>;

/** Options whose name starts with the text first, then a later word, then anywhere. */
function rank(options: ComboboxOption[], text: string) {
  const query = text.trim().toLowerCase();
  if (!query) return options;
  const tiers: ComboboxOption[][] = [[], [], []];
  for (const option of options) {
    const label = option.label.toLowerCase();
    if (label.startsWith(query)) tiers[0].push(option);
    else if (label.includes(` ${query}`)) tiers[1].push(option);
    else if (label.includes(query)) tiers[2].push(option);
  }
  return tiers.flat();
}

/**
 * The select of DESIGN_SYSTEM §9.1: type to filter, ↑/↓ to move, Enter to pick, Esc to close the
 * list. While the list is closed, keys behave as in any input: Enter (and Shift+Enter) go to the
 * form, so the quick-add path `250` Tab `din` Enter Enter saves (UI_FLOWS §4), and app shortcuts
 * still work. Leave archived options out of `options`.
 */
export function Combobox({
  options,
  value,
  onChange,
  placeholder,
  className,
  ...inputProps
}: InputProps & {
  options: ComboboxOption[];
  value: string | null;
  onChange: (value: string) => void;
  placeholder?: string;
  className?: string;
}) {
  const [open, setOpen] = useState(false);
  const [search, setSearch] = useState("");
  const [highlight, setHighlight] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  // State, not a ref: Radix's portal mounts the list a render later, and that must re-run the effect.
  const [listEl, setListEl] = useState<HTMLDivElement | null>(null);
  const selected = options.find((o) => o.value === value);
  const matches = rank(options, search);

  useLayoutEffect(() => {
    const list = open ? listEl?.querySelector("[role=listbox]") : null;
    // cmdk writes each option's value to data-value before this runs.
    const active = [
      ...(list?.querySelectorAll<HTMLElement>("[cmdk-item]") ?? []),
    ].find((item) => item.dataset.value === highlight);
    active?.scrollIntoView({ block: "nearest" });
    const input = inputRef.current;
    // Once the list closes, the name is selected, so typing starts a new search.
    if (!open && input === document.activeElement) input?.select();
    // Set here, not in JSX: cmdk only names its elements once they're in the DOM.
    for (const [name, id] of [
      ["aria-controls", list?.id],
      ["aria-activedescendant", active?.id],
    ] as const)
      if (id) input?.setAttribute(name, id);
      else input?.removeAttribute(name);
  }, [listEl, open, highlight, search]);

  const show = (text: string) => {
    const ranked = rank(options, text);
    const keep = !text.trim() && ranked.some((o) => o.value === value);
    setSearch(text);
    setHighlight(keep && value ? value : (ranked[0]?.value ?? ""));
    setOpen(true);
  };
  const close = () => {
    setOpen(false);
    setSearch("");
  };
  const pick = (option: ComboboxOption) => {
    onChange(option.value);
    close();
  };
  const move = (step: number) => {
    if (matches.length === 0) return;
    const at = matches.findIndex((o) => o.value === highlight);
    setHighlight(matches[(at + step + matches.length) % matches.length].value);
  };

  return (
    <Popover.Root open={open} onOpenChange={(next) => !next && close()}>
      <Popover.Anchor asChild>
        <div className={cn("relative", className)}>
          <input
            ref={inputRef}
            {...inputProps}
            role="combobox"
            aria-expanded={open}
            aria-autocomplete="list"
            autoComplete="off"
            spellCheck={false}
            className={cn(fieldClasses, "pr-8")}
            value={open ? search : (selected?.label ?? "")}
            placeholder={selected?.label ?? placeholder}
            onChange={(e) => show(e.target.value)}
            onClick={() => !open && show("")}
            onFocus={(e) => e.target.select()}
            onBlur={close}
            onKeyDown={(e) => {
              if (e.key === "ArrowDown" || e.key === "ArrowUp") {
                e.preventDefault();
                if (open) move(e.key === "ArrowDown" ? 1 : -1);
                else show("");
              } else if (e.key === "Enter" && open) {
                // Enter only saves when no list is open (UI_FLOWS §4).
                e.preventDefault();
                const option = matches.find((o) => o.value === highlight);
                if (option) pick(option);
                else close();
              }
            }}
          />
          <ChevronDown className="pointer-events-none absolute top-1/2 right-3 size-4 -translate-y-1/2 text-muted-foreground" />
        </div>
      </Popover.Anchor>
      <Popover.Portal>
        <Popover.Content
          ref={setListEl}
          align="start"
          sideOffset={4}
          // Focus stays in the input while the list is open.
          onOpenAutoFocus={(e) => e.preventDefault()}
          // A click in the input isn't outside: it keeps the list open.
          onInteractOutside={(e) => {
            if (e.target === inputRef.current) e.preventDefault();
          }}
          className="glass-raised z-50 w-(--radix-popover-trigger-width) rounded-toolbar p-1"
        >
          {/* cmdk draws the list; the input above owns the keys, so cmdk never sees them. */}
          <Command
            shouldFilter={false}
            value={highlight}
            onValueChange={setHighlight}
          >
            <Command.List
              // Keeps focus in the input when an option is clicked.
              onMouseDown={(e) => e.preventDefault()}
              className="max-h-72 overflow-y-auto"
            >
              {matches.length === 0 && (
                <p className="px-2 py-1.5 text-muted-foreground">No matches</p>
              )}
              {matches.map((option) => (
                <Command.Item
                  key={option.value}
                  value={option.value}
                  onSelect={() => pick(option)}
                  className="flex cursor-pointer items-center gap-2 rounded-chip px-2 py-1.5 data-[selected=true]:bg-wash"
                >
                  {option.icon}
                  <span className="truncate">{option.label}</span>
                </Command.Item>
              ))}
            </Command.List>
          </Command>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
