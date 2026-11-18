import { afterEach, expect, test } from "vitest";
import { renderWithSettings } from "@/test/render";
import { FollowReduceTransparency } from "./FollowReduceTransparency";

const solid = () =>
  document.documentElement.classList.contains("reduce-transparency");

afterEach(() => {
  document.documentElement.classList.remove("reduce-transparency");
});

test("the switch on puts the class on <html> (SET-8)", () => {
  renderWithSettings(<FollowReduceTransparency />, {
    reduceTransparency: true,
  });
  expect(solid()).toBe(true);
});

test("the switch off takes it away", () => {
  document.documentElement.classList.add("reduce-transparency");
  renderWithSettings(<FollowReduceTransparency />);
  expect(solid()).toBe(false);
});
