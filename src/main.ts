import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize } from "@tauri-apps/api/dpi";

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

let atuinInputEl: HTMLInputElement | null;
let atuinResultsEl: HTMLElement | null;
let selectedIndex = -1;
let currentResults: AtuinResult[] = [];

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

function updateSelection() {
  if (!atuinResultsEl) return;
  const rows = atuinResultsEl.querySelectorAll(".result-row");
  rows.forEach((row, index) => {
    row.classList.toggle("selected", index === selectedIndex);
  });
}

function renderResults(results: AtuinResult[]) {
  if (!atuinResultsEl) return;

  const resultsContainer = atuinResultsEl;
  resultsContainer.innerHTML = "";
  currentResults = results;
  selectedIndex = results.length > 0 ? 0 : -1;

  if (results.length === 0) return;

  results.forEach((result, index) => {
    const row = document.createElement("div");
    row.className = "result-row" + (index === 0 ? " selected" : "");

    const commandEl = document.createElement("span");
    commandEl.className = "result-command";
    commandEl.textContent = result.command;

    const metaEl = document.createElement("span");
    metaEl.className = "result-meta";
    
    const exitClass = result.exit === "0" ? "exit-success" : "exit-failure";
    metaEl.innerHTML = `<span class="result-time">${result.time}</span> <span class="${exitClass}">${result.exit}</span>`;

    row.appendChild(commandEl);
    row.appendChild(metaEl);
    resultsContainer.appendChild(row);
  });

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
        currentResults = [];
        selectedIndex = -1;
        await resizeWindow(0);
        await window.hide();
      } catch (error) {
        console.error("Failed to hide window:", error);
      }
    }

    if (e.key === "ArrowDown" && currentResults.length > 0) {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, currentResults.length - 1);
      updateSelection();
    }

    if (e.key === "ArrowUp" && currentResults.length > 0) {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      updateSelection();
    }

    if (e.key === "Enter" && selectedIndex >= 0 && selectedIndex < currentResults.length) {
      e.preventDefault();
      const selected = currentResults[selectedIndex];
      try {
        await invoke("copy_to_clipboard", { text: selected.command });
        const window = getCurrentWebviewWindow();
        if (atuinInputEl) atuinInputEl.value = "";
        if (atuinResultsEl) atuinResultsEl.innerHTML = "";
        currentResults = [];
        selectedIndex = -1;
        await resizeWindow(0);
        await window.hide();
      } catch (error) {
        console.error("Failed to copy to clipboard:", error);
      }
    }
  });

  resizeWindow(0);
});
