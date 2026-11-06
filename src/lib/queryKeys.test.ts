import { expect, test } from "vitest";
import { queryKeys } from "./queryKeys";

test("every key except settings, startup and appVersion starts with ledger", () => {
  const outside = Object.entries(queryKeys)
    .map(([name, key]) => {
      // The first element never depends on the argument.
      const built: readonly unknown[] =
        typeof key === "function" ? key(undefined as never) : key;
      return [name, built[0]];
    })
    .filter(([, first]) => first !== "ledger")
    .map(([name]) => name);
  // Only these may sit outside the ledger refresh (ARCHITECTURE §5.2).
  expect(outside).toEqual(["settings", "startup", "appVersion"]);
});

// Every source file except tests, as text.
const sources = import.meta.glob<string>(
  ["/src/**/*.{ts,tsx}", "!**/*.test.*"],
  {
    query: "?raw",
    import: "default",
    eager: true,
  },
);

test("queries take their keys from queryKeys, never an inline literal", () => {
  const inline = Object.entries(sources)
    .filter(([, text]) => /queryKey:(?!\s*queryKeys\.)/.test(text))
    .map(([path]) => path);
  expect(inline).toEqual([]);
});
