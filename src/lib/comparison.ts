import type { ComparisonMode } from "@/bindings/ComparisonMode";

/** The comparison choices (RPT-4a, SET-7), for Settings and Reports' Compare to. */
export const COMPARE = [
  { value: "to_date", label: "Same days so far" },
  { value: "full", label: "Full previous period" },
] as const satisfies { value: ComparisonMode; label: string }[];
