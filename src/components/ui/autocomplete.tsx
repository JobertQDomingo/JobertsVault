import { Command } from "cmdk";
import { Popover } from "radix-ui";
import { useLayoutEffect, useRef, useState, type ComponentProps } from "react";
import { fieldClasses } from "./input";

type InputProps = Pick<
  ComponentProps<"input">,
  "id" | "aria-invalid" | "aria-describedby" | "placeholder"
>;

/**
 * A text field that suggests what was entered before as you type (TXN-6), in the select's list
 * (DESIGN_SYSTEM §9.1). Nothing is highlighted until ↓ or ↑, so Enter still goes to the form with
 * the text as typed (UI_FLOWS §4); Enter on a highlighted suggestion, or a click, picks it. Esc
 * closes the list. The list stays shut after a pick until the next keystroke.
 */
export function Autocomplete({
  value,
  onChange,
  suggestions,
  onPick,
  ...inputProps
}: InputProps & {
  value: string;
  onChange: (text: string) => void;
  suggestions: string[];
  onPick: (suggestion: string) => void;
}) {
  const [typing, setTyping] = useState(false);
  const [highlight, setHighlight] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);
  // State, not a ref: Radix's portal mounts the list a render later, and that must re-run the effect.
  const [listEl, setListEl] = useState<HTMLDivElement | null>(null);
  const open = typing && value.trim() !== "" && suggestions.length > 0;
  const chosen = open && suggestions.includes(highlight) ? highlight : "";

  useLayoutEffect(() => {
    const list = open ? listEl?.querySelector("[role=listbox]") : null;
    const active = [
      ...(list?.querySelectorAll<HTMLElement>("[cmdk-item]") ?? []),
    ].find((item) => item.dataset.value === chosen);
    active?.scrollIntoView({ block: "nearest" });
    // Set here, not in JSX: cmdk only names its elements once they're in the DOM.
    const input = inputRef.current;
    for (const [name, id] of [
      ["aria-controls", list?.id],
      ["aria-activedescendant", active?.id],
    ] as const)
      if (id) input?.setAttribute(name, id);
      else input?.removeAttribute(name);
  }, [listEl, open, chosen]);

  const close = () => {
    setTyping(false);
    setHighlight("");
  };
  const pick = (suggestion: string) => {
    onPick(suggestion);
    close();
  };
  const move = (step: number) => {
    const at = suggestions.indexOf(chosen);
    const n = suggestions.length;
    setHighlight(
      suggestions[at === -1 ? (step > 0 ? 0 : n - 1) : (at + step + n) % n],
    );
  };

  return (
    <Popover.Root open={open} onOpenChange={(next) => !next && close()}>
      <Popover.Anchor asChild>
        <input
          ref={inputRef}
          {...inputProps}
          role="combobox"
          aria-expanded={open}
          aria-autocomplete="list"
          autoComplete="off"
          spellCheck={false}
          className={fieldClasses}
          value={value}
          onChange={(e) => {
            onChange(e.target.value);
            setTyping(true);
            setHighlight("");
          }}
          onBlur={close}
          onKeyDown={(e) => {
            if (e.key === "ArrowDown" || e.key === "ArrowUp") {
              if (!value.trim() || suggestions.length === 0) return;
              // As in the select, ↓ / ↑ also bring back a list closed by Esc or a pick.
              e.preventDefault();
              setTyping(true);
              move(e.key === "ArrowDown" ? 1 : -1);
            } else if (e.key === "Enter" && chosen) {
              // Only a highlighted suggestion keeps Enter from saving.
              e.preventDefault();
              pick(chosen);
            }
          }}
        />
      </Popover.Anchor>
      <Popover.Portal>
        <Popover.Content
          ref={setListEl}
          align="start"
          sideOffset={4}
          // Focus stays in the input while the list is open.
          onOpenAutoFocus={(e) => e.preventDefault()}
          onInteractOutside={(e) => {
            if (e.target === inputRef.current) e.preventDefault();
          }}
          className="glass-raised z-50 w-(--radix-popover-trigger-width) rounded-toolbar p-1"
        >
          {/* cmdk draws the list; the input above owns the keys, so cmdk never sees them. It
              isn't told of cmdk's own choices either: cmdk highlights the first item, and a
              suggestion counts as chosen only after ↓ or ↑. */}
          <Command shouldFilter={false} value={chosen}>
            <Command.List
              // Keeps focus in the input when a suggestion is clicked.
              onMouseDown={(e) => e.preventDefault()}
              className="max-h-72 overflow-y-auto"
            >
              {suggestions.map((suggestion) => (
                <Command.Item
                  key={suggestion}
                  value={suggestion}
                  onSelect={() => pick(suggestion)}
                  className="cursor-pointer truncate rounded-chip px-2 py-1.5 hover:bg-wash data-[selected=true]:bg-wash"
                >
                  {suggestion}
                </Command.Item>
              ))}
            </Command.List>
          </Command>
        </Popover.Content>
      </Popover.Portal>
    </Popover.Root>
  );
}
