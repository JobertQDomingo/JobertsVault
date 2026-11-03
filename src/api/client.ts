import { invoke, type InvokeArgs } from "@tauri-apps/api/core";
import type { AppError } from "@/bindings/AppError";

/** An AppError from Rust, or something Tauri itself rejected with (e.g. an unknown command). */
export type ApiErrorDetail = AppError | { code: "unknown"; message: string };

// A Record, so the compiler flags a code that's missing here or no longer exists in Rust.
const CODES: Record<AppError["code"], true> = {
  validation: true,
  not_found: true,
  conflict: true,
  database: true,
  io: true,
};

function messageFor(detail: ApiErrorDetail): string {
  switch (detail.code) {
    case "not_found":
      return `${detail.entity} ${detail.id} not found`;
    // The raw text is SQLite's, which means nothing to the user (ARCHITECTURE §4.4).
    case "database":
      return "Something went wrong with the database. The details are in the error log.";
    default:
      return detail.message;
  }
}

export class ApiError extends Error {
  readonly detail: ApiErrorDetail;

  constructor(detail: ApiErrorDetail) {
    super(messageFor(detail));
    this.name = "ApiError";
    this.detail = detail;
  }
}

function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    typeof value.code === "string" &&
    Object.prototype.hasOwnProperty.call(CODES, value.code)
  );
}

/** The only way the app calls a Tauri command. Rejects with an ApiError. */
export async function call<T>(command: string, args?: InvokeArgs): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw new ApiError(
      isAppError(error) ? error : { code: "unknown", message: String(error) },
    );
  }
}
