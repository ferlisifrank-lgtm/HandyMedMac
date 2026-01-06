import { useState } from "react";
import { useTranslation } from "react-i18next";
import { PermissionsStep } from "./PermissionsStep";
import { MedicalModeStep } from "./MedicalModeStep";
import HandyTextLogo from "../icons/HandyTextLogo";
import { useSettings } from "@/hooks/useSettings";
import { commands } from "@/bindings";

interface SetupWizardProps {
  onComplete: () => void;
}

const TOTAL_STEPS = 2;

export const SetupWizard: React.FC<SetupWizardProps> = ({ onComplete }) => {
  const { t } = useTranslation();
  const { updateSetting } = useSettings();
  const [currentStep, setCurrentStep] = useState(1);
  const [permissionGranted, setPermissionGranted] = useState(false);
  const [medicalModeEnabled, setMedicalModeEnabled] = useState(false);
  const [completing, setCompleting] = useState(false);

  const handleNext = () => {
    if (currentStep < TOTAL_STEPS) {
      setCurrentStep(currentStep + 1);
    }
  };

  const handleFinish = async () => {
    setCompleting(true);
    try {
      // Save medical mode preference
      await updateSetting("medical_mode_enabled", medicalModeEnabled);

      // Mark setup as completed
      await commands.markSetupCompleted();

      // Complete the setup wizard
      onComplete();
    } catch (error) {
      console.error("Error completing setup:", error);
    } finally {
      setCompleting(false);
    }
  };

  const canProceed = currentStep === 1 ? permissionGranted : true;

  return (
    <div className="h-screen w-screen flex flex-col items-center justify-center p-6 bg-background">
      <div className="w-full max-w-2xl">
        {/* Header */}
        <div className="text-center mb-8">
          <div className="mb-4">
            <HandyTextLogo width={180} />
          </div>
          <h1 className="text-3xl font-bold mb-2">{t("setup.welcome")}</h1>
          <p className="text-text/70">{t("setup.subtitle")}</p>
        </div>

        {/* Progress indicator */}
        <div className="mb-8">
          <div className="flex items-center justify-center gap-2">
            {Array.from({ length: TOTAL_STEPS }).map((_, index) => (
              <div
                key={index}
                className={`h-2 flex-1 max-w-[100px] rounded-full transition-colors ${
                  index + 1 <= currentStep
                    ? "bg-logo-primary"
                    : "bg-mid-gray/30"
                }`}
              />
            ))}
          </div>
          <p className="text-center text-sm text-text/50 mt-2">
            {t("setup.step", { current: currentStep, total: TOTAL_STEPS })}
          </p>
        </div>

        {/* Step content */}
        <div className="mb-8">
          {currentStep === 1 && (
            <PermissionsStep
              onPermissionGranted={() => setPermissionGranted(true)}
            />
          )}
          {currentStep === 2 && (
            <MedicalModeStep
              onSelectionMade={setMedicalModeEnabled}
              initialValue={medicalModeEnabled}
            />
          )}
        </div>

        {/* Navigation buttons */}
        <div className="flex justify-center gap-3">
          {currentStep < TOTAL_STEPS ? (
            <>
              <button
                onClick={handleNext}
                disabled={!canProceed}
                className="px-8 py-3 bg-logo-primary text-white rounded-lg hover:bg-logo-primary/90 font-semibold transition-colors disabled:opacity-50 disabled:cursor-not-allowed min-w-[140px]"
              >
                {t("setup.next")}
              </button>
              {currentStep === 1 && !permissionGranted && (
                <button
                  onClick={handleNext}
                  className="px-8 py-3 bg-transparent border border-mid-gray text-text/70 rounded-lg hover:bg-mid-gray/10 font-medium transition-colors min-w-[140px]"
                >
                  {t("setup.skip")}
                </button>
              )}
            </>
          ) : (
            <button
              onClick={handleFinish}
              disabled={completing}
              className="px-8 py-3 bg-logo-primary text-white rounded-lg hover:bg-logo-primary/90 font-semibold transition-colors disabled:opacity-50 min-w-[140px]"
            >
              {completing ? t("common.loading") : t("setup.finish")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
};
