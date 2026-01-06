import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface MedicalModeToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const MedicalModeToggle: React.FC<MedicalModeToggleProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { t } = useTranslation();
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const medicalModeEnabled = getSetting("medical_mode_enabled") ?? true;

    return (
      <ToggleSwitch
        checked={medicalModeEnabled}
        onChange={(enabled) => updateSetting("medical_mode_enabled", enabled)}
        isUpdating={isUpdating("medical_mode_enabled")}
        label="Medical Mode"
        description="Apply Canadian medical vocabulary corrections to transcriptions"
        descriptionMode={descriptionMode}
        grouped={grouped}
        tooltipPosition="bottom"
      />
    );
  },
);
