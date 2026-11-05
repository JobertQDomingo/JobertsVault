import { type ClassValue, clsx } from "clsx";
import { extendTailwindMerge } from "tailwind-merge";

// The design system's type styles and radii (DESIGN_SYSTEM §4, §5). Without them tailwind-merge
// reads `text-small` as a color and drops it next to `text-income`.
const twMerge = extendTailwindMerge({
  extend: {
    theme: {
      text: ["display", "h1", "h2", "h3", "body", "body-strong", "small"],
      radius: ["chip", "control", "toolbar", "panel", "dialog", "pill"],
    },
  },
});

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
