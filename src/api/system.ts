import type { StartupStatus } from "@/bindings/StartupStatus";
import { call } from "./client";

export const getStartupStatus = () => call<StartupStatus>("get_startup_status");

export const openDataFolder = () => call<void>("open_data_folder");
