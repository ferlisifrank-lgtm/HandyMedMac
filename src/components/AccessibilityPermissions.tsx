import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  checkAccessibilityPermission,
  requestAccessibilityPermission,
} from "tauri-plugin-macos-permissions-api";
import { commands } from "@/bindings";

// Define permission state type
type PermissionState = "request" | "verify" | "granted";

// Define button configuration type
interface ButtonConfig {
  text: string;
  className: string;
}

const AccessibilityPermissions: React.FC = () => {
  const { t } = useTranslation();
  const [hasAccessibility, setHasAccessibility] = useState<boolean>(false);
  const [permissionState, setPermissionState] =
    useState<PermissionState>("request");

  // Check permissions without requesting
  const checkPermissions = async (): Promise<boolean> => {
    const hasPermissions: boolean = await checkAccessibilityPermission();
    setHasAccessibility(hasPermissions);
    setPermissionState(hasPermissions ? "granted" : "verify");
    return hasPermissions;
  };

  // Handle the unified button action based on current state
  const handleButtonClick = async (): Promise<void> => {
    if (permissionState === "request") {
      try {
        await requestAccessibilityPermission();
        // After system prompt, transition to verification state
        setPermissionState("verify");
      } catch (error) {
        console.error("Error requesting permissions:", error);
        setPermissionState("verify");
      }
    } else if (permissionState === "verify") {
      // State is "verify" - check if permission was granted
      await checkPermissions();
    }
  };

  // On app boot - check permissions
  useEffect(() => {
    const initialSetup = async (): Promise<void> => {
      const hasPermissions: boolean = await checkAccessibilityPermission();
      setHasAccessibility(hasPermissions);
      setPermissionState(hasPermissions ? "granted" : "request");
    };

    initialSetup();
  }, []);

  if (hasAccessibility) {
    return null;
  }

  // Configure button text and style based on state
  const buttonConfig: Record<PermissionState, ButtonConfig | null> = {
    request: {
      text: t("accessibility.openSettings"),
      className:
        "px-2 py-1 text-sm font-semibold bg-mid-gray/10 border  border-mid-gray/80 hover:bg-logo-primary/10 rounded cursor-pointer hover:border-logo-primary",
    },
    verify: {
      text: t("accessibility.openSettings"),
      className:
        "bg-gray-100 hover:bg-gray-200 text-gray-800 font-medium py-1 px-3 rounded text-sm flex items-center justify-center cursor-pointer",
    },
    granted: null,
  };

  const config = buttonConfig[permissionState] as ButtonConfig;

  const handleRestartClick = async (): Promise<void> => {
    try {
      await commands.restartApp();
    } catch (error) {
      console.error("Error restarting app:", error);
    }
  };

  return (
    <div className="p-4 w-full rounded-lg border border-mid-gray">
      <div className="flex flex-col gap-3">
        <div>
          <p className="text-sm font-semibold mb-2">
            {t("accessibility.permissionsDescription")}
          </p>
          <div className="text-sm text-text/70 space-y-1">
            <p>{t("accessibility.instructionsStep1")}</p>
            <p>{t("accessibility.instructionsStep2")}</p>
            <p>{t("accessibility.instructionsStep3")}</p>
          </div>
        </div>
        <div className="flex gap-2">
          <button onClick={handleButtonClick} className={config.className}>
            {config.text}
          </button>
          <button
            onClick={handleRestartClick}
            className="px-3 py-1 text-sm font-semibold bg-logo-primary text-white hover:bg-logo-primary/90 rounded cursor-pointer"
          >
            {t("accessibility.restartApp")}
          </button>
        </div>
      </div>
    </div>
  );
};

export default AccessibilityPermissions;
