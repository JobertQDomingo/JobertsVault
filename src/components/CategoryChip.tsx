import { ArrowLeftRight, Ellipsis, type LucideIcon } from "lucide-react";
import type { CSSProperties } from "react";
import { categoryIcons, type CategoryIconName } from "@/lib/categoryIcons";
import { cn } from "@/lib/utils";

type Size = "md" | "sm";

/** A 32 px (or 24 px) square: the color tinted at `--tint`, a 1 px inner edge, the icon in full color. */
function Chip({
  color,
  Icon,
  size,
}: {
  color: string;
  Icon: LucideIcon;
  size: Size;
}) {
  return (
    <span
      style={{ "--chip": `var(--${color})` } as CSSProperties}
      className={cn(
        "grid shrink-0 place-items-center bg-[color-mix(in_srgb,var(--chip)_var(--tint),transparent)] text-(color:--chip) shadow-[inset_0_0_0_1px_var(--edge)]",
        size === "md"
          ? "size-8 rounded-chip [&_svg]:size-4"
          : "size-6 rounded-[8px] [&_svg]:size-3.5",
      )}
    >
      <Icon aria-hidden />
    </span>
  );
}

/** A category's icon chip (DESIGN_SYSTEM §6), with its name beside it if given. */
export function CategoryChip({
  color,
  icon,
  size = "md",
  name,
}: {
  /** A palette key, `cat-1` … `cat-12`. */
  color: string;
  icon: string;
  size?: Size;
  name?: string;
}) {
  const chip = (
    <Chip
      color={color}
      Icon={categoryIcons[icon as CategoryIconName] ?? Ellipsis}
      size={size}
    />
  );
  if (!name) return chip;
  return (
    <span className="inline-flex min-w-0 items-center gap-2">
      {chip}
      <span className="truncate">{name}</span>
    </span>
  );
}

/** Transfers have no category: a neutral chip with the transfer icon. */
export function TransferChip({ size = "md" }: { size?: Size }) {
  return <Chip color="transfer" Icon={ArrowLeftRight} size={size} />;
}
