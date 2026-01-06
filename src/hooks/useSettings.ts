import { useEffect } from "react";
import { useShallow } from "zustand/react/shallow";
import { useSettingsStore } from "../stores/settingsStore";
import type { AppSettings as Settings, AudioDevice } from "@/bindings";

interface UseSettingsReturn {
  // State
  settings: Settings | null;
  isLoading: boolean;
  isUpdating: (key: string) => boolean;
  audioDevices: AudioDevice[];
  outputDevices: AudioDevice[];
  audioFeedbackEnabled: boolean;

  // Actions
  updateSetting: <K extends keyof Settings>(
    key: K,
    value: Settings[K],
  ) => Promise<void>;
  resetSetting: (key: keyof Settings) => Promise<void>;
  refreshSettings: () => Promise<void>;
  refreshAudioDevices: () => Promise<void>;
  refreshOutputDevices: () => Promise<void>;

  // Binding-specific actions
  updateBinding: (id: string, binding: string) => Promise<void>;
  resetBinding: (id: string) => Promise<void>;

  // Convenience getters
  getSetting: <K extends keyof Settings>(key: K) => Settings[K] | undefined;

  // EPHEMERAL MODE: Post-processing helpers removed - LLM features disabled
}

export const useSettings = (): UseSettingsReturn => {
  // Use shallow comparison to prevent unnecessary re-renders when state object reference changes
  // but actual values haven't changed
  const store = useSettingsStore(
    useShallow((state) => ({
      settings: state.settings,
      isLoading: state.isLoading,
      isUpdatingKey: state.isUpdatingKey,
      audioDevices: state.audioDevices,
      outputDevices: state.outputDevices,
      updateSetting: state.updateSetting,
      resetSetting: state.resetSetting,
      refreshSettings: state.refreshSettings,
      refreshAudioDevices: state.refreshAudioDevices,
      refreshOutputDevices: state.refreshOutputDevices,
      updateBinding: state.updateBinding,
      resetBinding: state.resetBinding,
      getSetting: state.getSetting,
      initialize: state.initialize,
    })),
  );

  // Initialize on first mount
  // Note: store.isLoading and store.initialize are stable references from useShallow
  useEffect(() => {
    if (store.isLoading) {
      store.initialize();
    }
    // Only depend on isLoading since initialize is a stable reference
  }, [store.isLoading, store.initialize]);

  return {
    settings: store.settings,
    isLoading: store.isLoading,
    isUpdating: store.isUpdatingKey,
    audioDevices: store.audioDevices,
    outputDevices: store.outputDevices,
    audioFeedbackEnabled: store.settings?.audio_feedback || false,
    updateSetting: store.updateSetting,
    resetSetting: store.resetSetting,
    refreshSettings: store.refreshSettings,
    refreshAudioDevices: store.refreshAudioDevices,
    refreshOutputDevices: store.refreshOutputDevices,
    updateBinding: store.updateBinding,
    resetBinding: store.resetBinding,
    getSetting: store.getSetting,
    // EPHEMERAL MODE: Post-processing methods removed - LLM features disabled
  };
};
