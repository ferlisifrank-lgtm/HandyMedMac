import { useTranslation } from "react-i18next";
import { useSettings } from "@/hooks/useSettings";

/**
 * Privacy notice informing users about ephemeral mode
 * PIPEDA Compliance: Transparency requirement (Section 4.4)
 */
export default function EphemeralModeNotice() {
  const { t } = useTranslation();
  const { getSetting, updateSetting } = useSettings();

  const hidePrivacyNotice = getSetting("hide_privacy_notice") ?? false;

  if (hidePrivacyNotice) {
    return null;
  }

  const handleDismiss = () => {
    updateSetting("hide_privacy_notice", true);
  };

  return (
    <div className="rounded-lg border border-blue-500/30 bg-blue-500/10 p-4 mb-4">
      <div className="flex items-start gap-3">
        <svg
          className="h-5 w-5 text-blue-500 mt-0.5 flex-shrink-0"
          fill="none"
          viewBox="0 0 24 24"
          stroke="currentColor"
        >
          <path
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
            d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
          />
        </svg>
        <div className="flex-1">
          <h3 className="text-sm font-semibold text-blue-700 dark:text-blue-300 mb-1">
            {t("privacy.ephemeralMode.title")}
          </h3>
          <p className="text-xs text-blue-600 dark:text-blue-400 mb-2">
            {t("privacy.ephemeralMode.description")}
          </p>
          <ul className="text-xs text-blue-600 dark:text-blue-400 space-y-1 list-disc list-inside">
            <li>{t("privacy.ephemeralMode.noAudioSaved")}</li>
            <li>{t("privacy.ephemeralMode.noTranscriptsSaved")}</li>
            <li>{t("privacy.ephemeralMode.clipboardOnly")}</li>
            <li>{t("privacy.ephemeralMode.localProcessing")}</li>
          </ul>
          <p className="text-xs text-blue-600 dark:text-blue-400 mt-2 italic">
            {t("privacy.ephemeralMode.warning")}
          </p>
        </div>
        <button
          onClick={handleDismiss}
          className="text-blue-500 hover:text-blue-600 dark:text-blue-400 dark:hover:text-blue-300 flex-shrink-0"
          aria-label={t("common.dismiss")}
        >
          <svg
            className="w-5 h-5"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={2}
              d="M6 18L18 6M6 6l12 12"
            />
          </svg>
        </button>
      </div>
    </div>
  );
}
