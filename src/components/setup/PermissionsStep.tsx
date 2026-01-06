import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  checkAccessibilityPermission,
  requestAccessibilityPermission,
} from "tauri-plugin-macos-permissions-api";

interface PermissionsStepProps {
  onPermissionGranted?: () => void;
}

export const PermissionsStep: React.FC<PermissionsStepProps> = ({
  onPermissionGranted,
}) => {
  const { t } = useTranslation();
  const [hasPermission, setHasPermission] = useState(false);
  const [checking, setChecking] = useState(false);

  const checkPermission = async () => {
    setChecking(true);
    try {
      const granted = await checkAccessibilityPermission();
      setHasPermission(granted);
      if (granted && onPermissionGranted) {
        onPermissionGranted();
      }
    } catch (error) {
      console.error("Error checking permission:", error);
    } finally {
      setChecking(false);
    }
  };

  const openSettings = async () => {
    try {
      await requestAccessibilityPermission();
    } catch (error) {
      console.error("Error opening settings:", error);
    }
  };

  useEffect(() => {
    checkPermission();
  }, []);

  return (
    <div className="flex flex-col gap-6 max-w-lg mx-auto">
      <div className="text-center">
        <h2 className="text-2xl font-semibold mb-2">
          {t("setup.permissions.title")}
        </h2>
        <p className="text-text/70">{t("setup.permissions.description")}</p>
      </div>

      {hasPermission ? (
        <div className="bg-green-500/10 border border-green-500/30 rounded-lg p-6 text-center">
          <svg
            className="w-16 h-16 mx-auto mb-3 text-green-400"
            viewBox="0 0 20 20"
            fill="currentColor"
          >
            <path
              fillRule="evenodd"
              d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
              clipRule="evenodd"
            />
          </svg>
          <p className="text-green-400 font-medium">
            {t("setup.permissions.granted")}
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          <div className="bg-background-dark border border-mid-gray rounded-lg p-6">
            <ol className="space-y-3 text-sm text-text/80">
              <li className="flex gap-3">
                <span className="flex-shrink-0 w-6 h-6 rounded-full bg-logo-primary text-white flex items-center justify-center text-xs font-semibold">
                  1
                </span>
                <span>{t("setup.permissions.step1")}</span>
              </li>
              <li className="flex gap-3">
                <span className="flex-shrink-0 w-6 h-6 rounded-full bg-logo-primary text-white flex items-center justify-center text-xs font-semibold">
                  2
                </span>
                <span>{t("setup.permissions.step2")}</span>
              </li>
              <li className="flex gap-3">
                <span className="flex-shrink-0 w-6 h-6 rounded-full bg-logo-primary text-white flex items-center justify-center text-xs font-semibold">
                  3
                </span>
                <span>{t("setup.permissions.step3")}</span>
              </li>
            </ol>
          </div>

          <div className="flex flex-col gap-2">
            <button
              onClick={openSettings}
              className="w-full px-6 py-3 bg-logo-primary text-white rounded-lg hover:bg-logo-primary/90 font-semibold transition-colors"
            >
              {t("setup.permissions.openSettings")}
            </button>
            <button
              onClick={checkPermission}
              disabled={checking}
              className="w-full px-6 py-3 bg-background-dark border border-mid-gray text-text rounded-lg hover:bg-mid-gray/10 font-medium transition-colors disabled:opacity-50"
            >
              {checking
                ? t("common.loading")
                : t("setup.permissions.checkStatus")}
            </button>
          </div>

          {!hasPermission && (
            <p className="text-sm text-text/50 text-center">
              {t("setup.permissions.notGranted")}
            </p>
          )}
        </div>
      )}
    </div>
  );
};
