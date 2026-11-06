import { expect, test } from "vitest";
import { idParam } from "./records";

test("an id from the URL is a positive whole number that fits exactly", () => {
  expect(idParam("2")).toBe(2);
  for (const bad of [null, "", "0", "-1", "1.5", "x", "99999999999999999999"])
    expect(idParam(bad)).toBeNull();
});
