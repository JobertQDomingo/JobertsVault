import { afterEach, expect, test, vi } from "vitest";
import { followSystemTheme } from "./theme";

/** A stand-in for Windows' light or dark setting, which jsdom doesn't have. */
function fakeWindowsTheme(dark: boolean) {
  const listeners = new Set<() => void>();
  const query = {
    matches: dark,
    addEventListener: (_: string, listener: () => void) =>
      listeners.add(listener),
    removeEventListener: (_: string, listener: () => void) =>
      listeners.delete(listener),
  };
  vi.stubGlobal("matchMedia", (media: string) => {
    expect(media).toBe("(prefers-color-scheme: dark)");
    return query;
  });
  return {
    switchTo(next: boolean) {
      query.matches = next;
      listeners.forEach((listener) => listener());
    },
  };
}

afterEach(() => {
  vi.unstubAllGlobals();
  document.documentElement.classList.remove("dark");
});

test("adds dark when Windows is dark", () => {
  fakeWindowsTheme(true);
  followSystemTheme();
  expect(document.documentElement.classList.contains("dark")).toBe(true);
});

test("follows a change while the app is open", () => {
  const windows = fakeWindowsTheme(false);
  followSystemTheme();
  expect(document.documentElement.classList.contains("dark")).toBe(false);
  windows.switchTo(true);
  expect(document.documentElement.classList.contains("dark")).toBe(true);
  windows.switchTo(false);
  expect(document.documentElement.classList.contains("dark")).toBe(false);
});

test("stops following after cleanup", () => {
  const windows = fakeWindowsTheme(false);
  const stop = followSystemTheme();
  stop();
  windows.switchTo(true);
  expect(document.documentElement.classList.contains("dark")).toBe(false);
});
