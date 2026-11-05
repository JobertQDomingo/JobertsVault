import { expect, test } from "vitest";
import { queryKeys } from "./queryKeys";

test("every key except settings and startup starts with ledger", () => {
  const outside = Object.entries(queryKeys)
    .map(([name, key]) => {
      // The first element never depends on the argument.
      const built: readonly unknown[] =
        typeof key === "function" ? key(undefined as never) : key;
      return [name, built[0]];
    })
    .filter(([, first]) => first !== "ledger")
    .map(([name]) => name);
  // Only these two may sit outside the ledger refresh (ARCHITECTURE §5.2).
  expect(outside).toEqual(["settings", "startup"]);
});
