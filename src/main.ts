import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize } from "@tauri-apps/api/dpi";

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

let atuinInputEl: HTMLInputElement | null;
let atuinResultsEl: HTMLElement | null;

const BASE_HEIGHT = 60;
const RESULT_HEIGHT = 48;
const CONTAINER_PADDING = 16;
const MAX_VISIBLE_RESULTS = 10;
const WINDOW_WIDTH = 700;

interface AtuinResult {
  command: string;
  exit: string;
  directory: string;
  time: string;
}

function parseAtuinLine(line: string): AtuinResult | null {
  const parts = line.split("|");
  if (parts.length >= 4) {
    const command = parts.slice(0, -3).join("|");
    const exit = parts[parts.length - 3];
    const directory = parts[parts.length - 2];
    const time = parts[parts.length - 1];
    return { command, exit, directory, time };
  }
  return null;
}

async function resizeWindow(resultCount: number) {
  if (!isTauri()) return;

  const visibleCount = Math.min(resultCount, MAX_VISIBLE_RESULTS);
  const resultsHeight = visibleCount > 0 ? visibleCount * RESULT_HEIGHT + CONTAINER_PADDING : 0;
  const newHeight = BASE_HEIGHT + resultsHeight;

  try {
    const window = getCurrentWebviewWindow();
    await window.setSize(new LogicalSize(WINDOW_WIDTH, newHeight));
  } catch (error) {
    console.error("Failed to resize window:", error);
  }
}

function renderResults(results: AtuinResult[]) {
  if (!atuinResultsEl) return;

  atuinResultsEl.innerHTML = "";

  if (results.length === 0) return;

  for (const result of results) {
    const row = document.createElement("div");
    row.className = "result-row";

    const commandEl = document.createElement("span");
    commandEl.className = "result-command";
    commandEl.textContent = result.command;

    const metaEl = document.createElement("span");
    metaEl.className = "result-meta";
    
    const exitClass = result.exit === "0" ? "exit-success" : "exit-failure";
    metaEl.innerHTML = `<span class="result-time">${result.time}</span> <span class="${exitClass}">${result.exit}</span>`;

    row.appendChild(commandEl);
    row.appendChild(metaEl);
    atuinResultsEl.appendChild(row);
  }

  resizeWindow(results.length);
}

async function searchAtuin() {
  if (!atuinInputEl || !atuinResultsEl) return;

  const query = atuinInputEl.value.trim();
  console.log("searchAtuin called with query:", query);

  if (!query) {
    atuinResultsEl.innerHTML = "";
    resizeWindow(0);
    return;
  }

  if (!isTauri()) {
    console.error("Not running in Tauri context");
    return;
  }

  try {
    console.log("Invoking atuin_search_command...");
    const output: string = await invoke("atuin_search_command", { query });
    console.log("Got output:", output);

    if (!output || output.trim() === "") {
      console.log("Empty output");
      atuinResultsEl.innerHTML = "";
      resizeWindow(0);
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

    console.log("Parsed results:", results.length);
    renderResults(results.reverse());
  } catch (error) {
    console.error("Atuin search error:", error);
    atuinResultsEl.innerHTML = "";
    resizeWindow(0);
  }
}

let searchTimeout: ReturnType<typeof setTimeout> | null = null;

function debounceSearch() {
  if (searchTimeout) {
    clearTimeout(searchTimeout);
  }
  searchTimeout = setTimeout(searchAtuin, 150);
}

window.addEventListener("DOMContentLoaded", () => {
  atuinInputEl = document.querySelector("#atuin-input");
  atuinResultsEl = document.querySelector("#atuin-results");

  if (atuinInputEl) {
    atuinInputEl.addEventListener("input", debounceSearch);
    atuinInputEl.focus();
  }

  document.querySelector("#atuin-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    searchAtuin();
  });

  document.addEventListener("keydown", async (e) => {
    if (e.key === "Escape" && isTauri()) {
      e.preventDefault();
      e.stopPropagation();
      try {
        const window = getCurrentWebviewWindow();
        if (atuinInputEl) atuinInputEl.value = "";
        if (atuinResultsEl) atuinResultsEl.innerHTML = "";
        await resizeWindow(0);
        await window.hide();
      } catch (error) {
        console.error("Failed to hide window:", error);
      }
    }
  });

  resizeWindow(0);
});
