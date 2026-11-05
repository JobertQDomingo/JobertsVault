import { RadioGroup } from "radix-ui";
import { useRef, useState, type CSSProperties } from "react";
import type { Category } from "@/bindings/Category";
import type { CategoryKind } from "@/bindings/CategoryKind";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogBody,
  DialogContent,
  DialogFooter,
} from "@/components/ui/dialog";
import { Field, Input } from "@/components/ui/input";
import { notify } from "@/components/ui/toast";
import { useCreateCategory, useUpdateCategory } from "@/hooks/useCategories";
import { categoryIcons, type CategoryIconName } from "@/lib/categoryIcons";
import { showSaveError } from "@/lib/saveError";
import { cn } from "@/lib/utils";

// DESIGN_SYSTEM §3.4: the picker offers only the 12 palette swatches.
const COLORS = Array.from({ length: 12 }, (_, i) => `cat-${i + 1}`);
const ICONS = Object.keys(categoryIcons) as CategoryIconName[];
const KIND_NAMES: Record<CategoryKind, string> = {
  expense: "expense",
  income: "income",
};

type Errors = Partial<Record<"name" | "color" | "icon", string>>;

/**
 * The new and edit category dialog (CAT-2): a name, one of the 12 colors and one of the curated
 * icons. The kind is the list it was opened from and can't change (CAT-3).
 */
export function CategoryDialog({
  category,
  kind,
  defaultColor,
  onClose,
}: {
  category?: Category;
  kind: CategoryKind;
  defaultColor: string;
  onClose: () => void;
}) {
  const [name, setName] = useState(category?.name ?? "");
  const [color, setColor] = useState(category?.color ?? defaultColor);
  const [icon, setIcon] = useState(category?.icon ?? "ellipsis");
  const [errors, setErrors] = useState<Errors>({});
  const create = useCreateCategory();
  const update = useUpdateCategory();
  const saving = useRef(false);

  const save = async () => {
    if (saving.current) return;
    saving.current = true;
    try {
      const fields = { name, color, icon };
      if (category)
        await update.mutateAsync({ id: category.id, category: fields });
      else await create.mutateAsync({ kind, ...fields });
      notify.success("Saved");
      onClose();
    } catch (error) {
      showSaveError(error, ["name", "color", "icon"] as const, (f, e) =>
        setErrors({ [f]: e.message }),
      );
    } finally {
      saving.current = false;
    }
  };

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()}>
      <DialogContent
        title={category ? "Edit category" : `New ${KIND_NAMES[kind]} category`}
      >
        <form
          noValidate
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <DialogBody>
            <Field label="Name" htmlFor="cat-name" error={errors.name}>
              <Input
                id="cat-name"
                autoComplete="off"
                autoFocus
                value={name}
                onChange={(e) => {
                  setName(e.target.value);
                  setErrors({});
                }}
              />
            </Field>
            <Picker label="Color" error={errors.color}>
              <RadioGroup.Root
                aria-label="Color"
                value={color}
                onValueChange={setColor}
                className="flex flex-wrap gap-2"
              >
                {COLORS.map((c, i) => (
                  <RadioGroup.Item
                    key={c}
                    value={c}
                    aria-label={`Color ${i + 1}`}
                    style={{ "--swatch": `var(--${c})` } as CSSProperties}
                    className="size-7 rounded-full bg-(--swatch) shadow-[inset_0_0_0_1px_var(--edge)] outline-none focus-visible:ring-3 focus-visible:ring-ring/40 data-[state=checked]:ring-2 data-[state=checked]:ring-foreground data-[state=checked]:ring-offset-2 data-[state=checked]:ring-offset-(--glass-strong)"
                  />
                ))}
              </RadioGroup.Root>
            </Picker>
            <Picker label="Icon" error={errors.icon}>
              <RadioGroup.Root
                aria-label="Icon"
                value={icon}
                onValueChange={setIcon}
                style={{ "--chip": `var(--${color})` } as CSSProperties}
                className="grid max-h-44 grid-cols-10 gap-1 overflow-y-auto"
              >
                {ICONS.map((name) => {
                  const Icon = categoryIcons[name];
                  return (
                    <RadioGroup.Item
                      key={name}
                      value={name}
                      aria-label={name}
                      title={name}
                      className={cn(
                        "grid size-8 place-items-center rounded-chip text-muted-foreground outline-none hover:bg-wash focus-visible:ring-3 focus-visible:ring-ring/40",
                        "data-[state=checked]:bg-[color-mix(in_srgb,var(--chip)_var(--tint),transparent)] data-[state=checked]:text-(color:--chip) data-[state=checked]:shadow-[inset_0_0_0_1px_var(--chip)]",
                      )}
                    >
                      <Icon aria-hidden className="size-4" />
                    </RadioGroup.Item>
                  );
                })}
              </RadioGroup.Root>
            </Picker>
          </DialogBody>
          <DialogFooter>
            <Button onClick={onClose}>Cancel</Button>
            <Button variant="primary" type="submit">
              {category ? "Save" : "Create category"}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}

function Picker({
  label,
  error,
  children,
}: {
  label: string;
  error?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="grid gap-1.5">
      <span className="text-small font-semibold text-muted-foreground">
        {label}
      </span>
      {children}
      {error && (
        <p role="alert" className="text-small text-danger">
          {error}
        </p>
      )}
    </div>
  );
}
