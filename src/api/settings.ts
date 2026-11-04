import type { Settings } from "@/bindings/Settings";
import type { SettingsView } from "@/bindings/SettingsView";
import { call } from "./client";

export const getSettings = () => call<SettingsView>("get_settings");

/** Saves the whole set and returns it as stored. */
export const updateSettings = (settings: Settings) =>
  call<SettingsView>("update_settings", { settings });
