import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";

// Check if we're running in Tauri context
function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

let greetInputEl: HTMLInputElement | null;
let greetMsgEl: HTMLElement | null;
let clipboardInputEl: HTMLInputElement | null;
let clipboardMsgEl: HTMLElement | null;
let atuinInputEl: HTMLInputElement | null;
let atuinMsgEl: HTMLElement | null;
let atuinResultsEl: HTMLElement | null;

interface AtuinResult {
  command: string;
  exit: string;
  directory: string;
  time: string;
}

async function greet() {
  if (greetMsgEl && greetInputEl) {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    greetMsgEl.textContent = await invoke("greet", {
      name: greetInputEl.value,
    });
  }
}

async function copyToClipboard() {
  if (clipboardMsgEl && clipboardInputEl) {
    try {
      await invoke("copy_to_clipboard", {
        text: clipboardInputEl.value,
      });
      clipboardMsgEl.textContent = "✓ Copied to clipboard!";
      clipboardMsgEl.style.color = "green";
    } catch (error) {
      clipboardMsgEl.textContent = `Error: ${error}`;
      clipboardMsgEl.style.color = "red";
    }
  }
}

function parseAtuinLine(line: string): AtuinResult | null {
  // Format: {command}|{exit}|{directory}|{time}
  // Note: command can contain '|' characters, so we split from the right
  const parts = line.split("|");
  if (parts.length >= 4) {
    // Join all parts except the last 3 to handle pipes in commands
    const command = parts.slice(0, -3).join("|");
    const exit = parts[parts.length - 3];
    const directory = parts[parts.length - 2];
    const time = parts[parts.length - 1];
    return { command, exit, directory, time };
  }
  return null;
}

async function searchAtuin() {
  if (atuinMsgEl && atuinInputEl && atuinResultsEl) {
    if (!isTauri()) {
      atuinMsgEl.textContent =
        "Error: Not running in Tauri context. Please run with 'npm run tauri dev'";
      atuinMsgEl.style.color = "red";
      return;
    }

    try {
      const query = atuinInputEl.value;
      atuinMsgEl.textContent = "Searching...";
      atuinMsgEl.style.color = "blue";
      atuinResultsEl.innerHTML = "";

      const output: string = await invoke("atuin_search_command", { query });

      if (!output || output.trim() === "") {
        atuinMsgEl.textContent = "No results found";
        atuinMsgEl.style.color = "gray";
        return;
      }

      const lines = output.split("\n").filter((line) => line.trim() !== "");
      const results: AtuinResult[] = [];

      for (const line of lines) {
        const parsed = parseAtuinLine(line);
        if (parsed) {
          results.push(parsed);
        }
      }

      if (results.length === 0) {
        atuinMsgEl.textContent = "No parseable results found";
        atuinMsgEl.style.color = "gray";
        return;
      }

      atuinMsgEl.textContent = `Found ${results.length} command(s)`;
      atuinMsgEl.style.color = "green";

      // Display results
      const resultsList = document.createElement("ul");
      resultsList.style.textAlign = "left";
      resultsList.style.maxHeight = "400px";
      resultsList.style.overflowY = "auto";

      for (const result of results) {
        const li = document.createElement("li");
        li.style.marginBottom = "10px";
        li.style.padding = "8px";
        li.style.backgroundColor = "#f5f5f5";
        li.style.borderRadius = "4px";

        const commandEl = document.createElement("code");
        commandEl.textContent = result.command;
        commandEl.style.display = "block";
        commandEl.style.marginBottom = "4px";

        const metaEl = document.createElement("small");
        metaEl.style.color = "#666";
        metaEl.textContent = `${result.directory} • ${result.time} • exit: ${result.exit}`;

        li.appendChild(commandEl);
        li.appendChild(metaEl);
        resultsList.appendChild(li);
      }

      atuinResultsEl.appendChild(resultsList);
    } catch (error) {
      atuinMsgEl.textContent = `Error: ${error}`;
      atuinMsgEl.style.color = "red";
      atuinResultsEl.innerHTML = "";
    }
  }
}

window.addEventListener("DOMContentLoaded", () => {
  greetInputEl = document.querySelector("#greet-input");
  greetMsgEl = document.querySelector("#greet-msg");
  document.querySelector("#greet-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    greet();
  });

  clipboardInputEl = document.querySelector("#clipboard-input");
  clipboardMsgEl = document.querySelector("#clipboard-msg");
  document.querySelector("#clipboard-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    copyToClipboard();
  });

  atuinInputEl = document.querySelector("#atuin-input");
  atuinMsgEl = document.querySelector("#atuin-msg");
  atuinResultsEl = document.querySelector("#atuin-results");
  document.querySelector("#atuin-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    searchAtuin();
  });

  // Hide window on Escape key
  document.addEventListener("keydown", async (e) => {
    if (e.key === "Escape" && isTauri()) {
      e.preventDefault();
      e.stopPropagation();
      try {
        const window = getCurrentWebviewWindow();
        await window.hide();
      } catch (error) {
        console.error("Failed to hide window:", error);
      }
    }
  });
});
