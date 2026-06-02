import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import type { Config } from "./bindings";

let shortcutInput: HTMLInputElement | null;
let themeSelect: HTMLSelectElement | null;
let maxResultsInput: HTMLInputElement | null;
let windowWidthInput: HTMLInputElement | null;
let saveButton: HTMLButtonElement | null;
let cancelButton: HTMLButtonElement | null;
let updateButton: HTMLButtonElement | null;
let messageDiv: HTMLElement | null;

type SelfUpdateResult = {
  updated: boolean;
  current_version: string;
  latest_version: string;
  message: string;
  release_url: string;
};

async function loadConfig() {
  try {
    const config: Config = await invoke("get_config");

    if (shortcutInput) shortcutInput.value = config.shortcut;
    if (themeSelect) themeSelect.value = config.theme;
    if (maxResultsInput) maxResultsInput.value = config.max_results.toString();
    if (windowWidthInput) windowWidthInput.value = config.window_width.toString();
  } catch (error) {
    console.error("Failed to load config:", error);
    showMessage("Failed to load configuration", "error");
  }
}

function showMessage(text: string, type: "success" | "error") {
  if (!messageDiv) return;

  messageDiv.textContent = text;
  messageDiv.className = `message ${type}`;
  messageDiv.style.display = "block";

  setTimeout(() => {
    if (messageDiv) {
      messageDiv.style.display = "none";
    }
  }, 3000);
}

async function saveConfig() {
  if (!shortcutInput || !themeSelect || !maxResultsInput || !windowWidthInput) {
    return;
  }

  const shortcut = shortcutInput.value.trim();
  const theme = themeSelect.value;
  const maxResults = parseInt(maxResultsInput.value);
  const windowWidth = parseInt(windowWidthInput.value);

  // Validate inputs
  if (!shortcut) {
    showMessage("Shortcut cannot be empty", "error");
    return;
  }

  if (maxResults < 5 || maxResults > 100) {
    showMessage("Max results must be between 5 and 100", "error");
    return;
  }

  if (windowWidth < 400 || windowWidth > 2000) {
    showMessage("Window width must be between 400 and 2000", "error");
    return;
  }

  if (saveButton) saveButton.disabled = true;

  try {
    await invoke("update_config", {
      shortcut,
      theme,
      maxResults,
      windowWidth,
    });

    showMessage("Settings saved successfully! Restart the app to apply shortcut changes.", "success");

    // Reload main window config if it exists
    // This will update theme and other settings without restart
    setTimeout(() => {
      if (saveButton) saveButton.disabled = false;
    }, 1000);
  } catch (error) {
    console.error("Failed to save config:", error);
    showMessage(`Failed to save settings: ${error}`, "error");
    if (saveButton) saveButton.disabled = false;
  }
}

async function selfUpdate() {
  if (updateButton) updateButton.disabled = true;
  showMessage("Checking GitHub releases...", "success");

  try {
    const result: SelfUpdateResult = await invoke("self_update");
    showMessage(result.message, "success");
    if (!result.updated && updateButton) updateButton.disabled = false;
  } catch (error) {
    console.error("Failed to update:", error);
    showMessage(`Failed to update: ${error}`, "error");
    if (updateButton) updateButton.disabled = false;
  }
}

function cancelSettings() {
  const window = getCurrentWebviewWindow();
  window.close();
}

window.addEventListener("DOMContentLoaded", async () => {
  shortcutInput = document.querySelector("#shortcut");
  themeSelect = document.querySelector("#theme");
  maxResultsInput = document.querySelector("#max_results");
  windowWidthInput = document.querySelector("#window_width");
  saveButton = document.querySelector("#save-button");
  cancelButton = document.querySelector("#cancel-button");
  updateButton = document.querySelector("#update-button");
  messageDiv = document.querySelector("#message");

  await loadConfig();

  saveButton?.addEventListener("click", saveConfig);
  cancelButton?.addEventListener("click", cancelSettings);
  updateButton?.addEventListener("click", selfUpdate);

  // Handle Enter key to save
  document.addEventListener("keydown", (e) => {
    if (e.key === "Enter" && e.metaKey) {
      saveConfig();
    }
  });
});
