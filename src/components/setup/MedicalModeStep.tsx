import { useState } from "react";
import { useTranslation } from "react-i18next";

interface MedicalModeStepProps {
  onSelectionMade?: (enabled: boolean) => void;
  initialValue?: boolean;
}

export const MedicalModeStep: React.FC<MedicalModeStepProps> = ({
  onSelectionMade,
  initialValue = false,
}) => {
  const { t } = useTranslation();
  const [medicalModeEnabled, setMedicalModeEnabled] = useState(initialValue);

  const handleToggle = (enabled: boolean) => {
    setMedicalModeEnabled(enabled);
    if (onSelectionMade) {
      onSelectionMade(enabled);
    }
  };

  return (
    <div className="flex flex-col gap-6 max-w-lg mx-auto">
      <div className="text-center">
        <h2 className="text-2xl font-semibold mb-2">
          {t("setup.medical.title")}
        </h2>
        <p className="text-text/70">{t("setup.medical.description")}</p>
      </div>

      <div className="bg-background-dark border border-mid-gray rounded-lg p-6 space-y-4">
        <div>
          <h3 className="font-semibold mb-2">
            {t("setup.medical.consentTitle")}
          </h3>
          <p className="text-sm text-text/70">
            {t("setup.medical.consentText")}
          </p>
        </div>

        <div className="pt-4 border-t border-mid-gray">
          <label className="flex items-start gap-4 cursor-pointer group">
            <div className="relative flex items-center">
              <input
                type="checkbox"
                checked={medicalModeEnabled}
                onChange={(e) => handleToggle(e.target.checked)}
                className="w-5 h-5 rounded border-2 border-mid-gray bg-background checked:bg-logo-primary checked:border-logo-primary focus:ring-2 focus:ring-logo-primary focus:ring-offset-2 focus:ring-offset-background cursor-pointer transition-colors"
              />
              {medicalModeEnabled && (
                <svg
                  className="absolute inset-0 w-5 h-5 text-white pointer-events-none"
                  viewBox="0 0 20 20"
                  fill="currentColor"
                >
                  <path
                    fillRule="evenodd"
                    d="M16.707 5.293a1 1 0 010 1.414l-8 8a1 1 0 01-1.414 0l-4-4a1 1 0 011.414-1.414L8 12.586l7.293-7.293a1 1 0 011.414 0z"
                    clipRule="evenodd"
                  />
                </svg>
              )}
            </div>
            <div className="flex-1">
              <div className="font-medium group-hover:text-logo-primary transition-colors">
                {t("setup.medical.enableLabel")}
              </div>
              <div className="text-sm text-text/60 mt-1">
                {t("setup.medical.enableDescription")}
              </div>
            </div>
          </label>
        </div>

        <div className="text-xs text-text/50 pt-2 border-t border-mid-gray/50">
          {t("setup.medical.pipedaCompliance")}
        </div>
      </div>

      <div className="text-center">
        <p className="text-sm text-text/60">{t("setup.medical.learnMore")}</p>
      </div>
    </div>
  );
};
