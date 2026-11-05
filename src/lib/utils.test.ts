import { expect, test } from "vitest";
import { cn } from "./utils";

test("a type style and a text color both stay", () => {
  expect(cn("text-small", "text-income")).toBe("text-small text-income");
  expect(cn("text-body-strong text-muted-foreground")).toBe(
    "text-body-strong text-muted-foreground",
  );
});

test("a later type style or radius replaces the earlier one", () => {
  expect(cn("text-h1", "text-body")).toBe("text-body");
  expect(cn("rounded-panel", "rounded-toolbar")).toBe("rounded-toolbar");
});
