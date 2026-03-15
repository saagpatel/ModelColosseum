import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { Model } from "../types";

interface AppState {
  ollamaOnline: boolean | null;
  models: Model[];
  loading: boolean;
  init: () => Promise<void>;
  refresh: () => Promise<void>;
}

export const useAppStore = create<AppState>((set) => ({
  ollamaOnline: null,
  models: [],
  loading: true,

  init: async () => {
    try {
      const healthy = await invoke<boolean>("health_check");
      set({ ollamaOnline: healthy });

      if (healthy) {
        const models = await invoke<Model[]>("refresh_models");
        set({ models });
      }
    } catch (err) {
      console.error("init error:", err);
      set({ ollamaOnline: false });
    } finally {
      set({ loading: false });
    }
  },

  refresh: async () => {
    set({ loading: true });
    try {
      const models = await invoke<Model[]>("refresh_models");
      set({ models });
    } catch (err) {
      console.error("refresh error:", err);
    } finally {
      set({ loading: false });
    }
  },
}));
