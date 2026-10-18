import { invoke } from "@tauri-apps/api/core";
import type {
  BoardResult,
  ProxySettingsPayload,
  ProxySettingsUpdate,
  UpdateInfo,
} from "./types";

export function getCached() {
  return invoke<BoardResult[]>("get_cached");
}

export function refreshAll() {
  return invoke<BoardResult[]>("refresh_all");
}

export function getSettings() {
  return invoke<ProxySettingsPayload>("get_settings");
}

export function setSettings(payload: ProxySettingsUpdate) {
  return invoke<ProxySettingsPayload>("set_settings", { payload });
}

export function checkUpdate() {
  return invoke<UpdateInfo>("check_update");
}
