import type { StartupStatus } from "@/bindings/StartupStatus";
import { call } from "./client";

export const getStartupStatus = () => call<StartupStatus>("get_startup_status");

export const openDataFolder = () => call<void>("open_data_folder");

export const getAppVersion = () => call<string>("get_app_version");

/** The newer version, or null when up to date. Always null in development builds. */
export const checkUpdate = () => call<string | null>("check_update");

/** Doesn't resolve on success: the app closes and reopens on the new version. */
export const installUpdate = () => call<void>("install_update");
