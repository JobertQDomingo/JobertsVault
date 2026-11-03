import { invoke } from "@tauri-apps/api/core";
import { beforeEach, expect, test, vi } from "vitest";
import { ApiError, call } from "./client";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const mockedInvoke = vi.mocked(invoke);

beforeEach(() => mockedInvoke.mockReset());

async function rejection(error: unknown): Promise<ApiError> {
  mockedInvoke.mockRejectedValueOnce(error);
  const caught: unknown = await call("anything").catch((e: unknown) => e);
  expect(caught).toBeInstanceOf(ApiError);
  return caught as ApiError;
}

test("passes the arguments through and returns the result", async () => {
  mockedInvoke.mockResolvedValueOnce(42);
  await expect(call<number>("get_answer", { id: 7 })).resolves.toBe(42);
  expect(mockedInvoke).toHaveBeenCalledWith("get_answer", { id: 7 });
});

test("turns a validation error into an ApiError with its field", async () => {
  const detail = {
    code: "validation",
    field: "amount",
    message: "Amount must be more than 0",
  };
  const error = await rejection(detail);
  expect(error.detail).toEqual(detail);
  expect(error.message).toBe("Amount must be more than 0");
});

test("gives not_found errors a readable message", async () => {
  const error = await rejection({
    code: "not_found",
    entity: "account",
    id: 7,
  });
  expect(error.message).toBe("account 7 not found");
});

test("wraps anything else as unknown", async () => {
  const error = await rejection("command foo not found");
  expect(error.detail).toEqual({
    code: "unknown",
    message: "command foo not found",
  });
});
