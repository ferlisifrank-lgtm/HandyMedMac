import React from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { FileText } from "lucide-react";

export const OpenCustomVocab: React.FC<{
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}> = ({ descriptionMode = "inline", grouped = false }) => {
  const { t } = useTranslation();

  const handleOpenFile = async () => {
    try {
      await invoke("open_custom_vocab_file");
    } catch (error) {
      console.error("Failed to open custom vocabulary file:", error);
    }
  };

  return (
    <div className={grouped ? "flex items-center justify-between py-3" : ""}>
      <div className="flex-1">
        <label className="block text-sm font-medium">
          {t("settings.advanced.customVocabulary.title")}
        </label>
        {descriptionMode === "inline" && (
          <p className="text-sm text-gray-600 dark:text-gray-400 mt-1">
            {t("settings.advanced.customVocabulary.description")}
          </p>
        )}
      </div>
      <button
        onClick={handleOpenFile}
        className="inline-flex items-center gap-2 px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
      >
        <FileText size={16} />
        {t("settings.advanced.customVocabulary.openFile")}
      </button>
    </div>
  );
};
