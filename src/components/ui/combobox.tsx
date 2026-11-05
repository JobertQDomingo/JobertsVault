import { Command, useCommandState } from "cmdk";
import { ChevronDown } from "lucide-react";
import { Popover } from "radix-ui";
import {
  useId,
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

/**
 * The select of DESIGN_SYSTEM §9.1: type to filter, ↑/↓ to move, Enter to pick, Esc to close the
 * list. Enter on a closed combobox submits the form, so the quick-add keyboard path
 * (`250` Tab `din` Enter Enter) saves (UI_FLOWS §4). Leave archived options out of `options`.
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
  const listId = useId();
  const selected = options.find((o) => o.value === value);
  const query = search.trim().toLowerCase();
  const matches = options.filter((o) => o.label.toLowerCase().includes(query));

  const show = (text: string) => {
    const next = text.trim().toLowerCase();
    setSearch(text);
    setHighlight(
      options.find((o) => o.label.toLowerCase().includes(next))?.value ?? "",
    );
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

  return (
    // cmdk only moves the highlight and picks with Enter; the options are filtered here.
    <Command
      shouldFilter={false}
      vimBindings={false}
      loop
      value={highlight}
      onValueChange={setHighlight}
      onKeyDown={(e) => {
        if (open) return;
        // Closed: nothing reaches cmdk. Enter saves the form; ↓ opens the list.
        e.stopPropagation();
        if (e.key === "Enter") {
          e.preventDefault();
          inputRef.current?.form?.requestSubmit();
        } else if (e.key === "ArrowDown") {
          e.preventDefault();
          show("");
        }
      }}
      className={cn("relative", className)}
    >
      <Popover.Root open={open} onOpenChange={(next) => !next && close()}>
        <Popover.Anchor asChild>
          <div className="relative">
            <ComboboxInput
              ref={inputRef}
              {...inputProps}
              open={open}
              listId={listId}
              value={open ? search : (selected?.label ?? "")}
              placeholder={selected?.label ?? placeholder}
              onChange={(e) => show(e.target.value)}
              onClick={() => !open && show("")}
              onFocus={(e) => e.target.select()}
              onBlur={close}
            />
            <ChevronDown className="pointer-events-none absolute top-1/2 right-3 size-4 -translate-y-1/2 text-muted-foreground" />
          </div>
        </Popover.Anchor>
        <Popover.Portal>
          <Popover.Content
            align="start"
            sideOffset={4}
            // Focus stays in the input while the list is open.
            onOpenAutoFocus={(e) => e.preventDefault()}
            onInteractOutside={(e) => {
              if (inputRef.current?.contains(e.target as Node))
                e.preventDefault();
            }}
            className="glass-raised z-50 w-(--radix-popover-trigger-width) rounded-toolbar p-1"
          >
            <Command.List
              id={listId}
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
          </Popover.Content>
        </Popover.Portal>
      </Popover.Root>
    </Command>
  );
}

function ComboboxInput({
  open,
  listId,
  className,
  ...props
}: ComponentProps<"input"> & { open: boolean; listId: string }) {
  // The highlighted option, so a screen reader announces it as ↑/↓ move.
  const active = useCommandState((state) => state.selectedItemId);
  return (
    <input
      role="combobox"
      aria-expanded={open}
      aria-controls={open ? listId : undefined}
      aria-autocomplete="list"
      aria-activedescendant={open ? active : undefined}
      autoComplete="off"
      spellCheck={false}
      className={cn(fieldClasses, "pr-8", className)}
      {...props}
    />
  );
}
