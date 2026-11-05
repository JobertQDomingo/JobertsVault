import { expect, test } from "vitest";
import { queryClient } from "./queryClient";

test("failed loads aren't retried: an error from Rust is the same the second time", () => {
  expect(queryClient.getDefaultOptions().queries?.retry).toBe(false);
});
