import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize } from "@tauri-apps/api/dpi";

function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

let atuinInputEl: HTMLInputElement | null;
let atuinResultsEl: HTMLElement | null;
let filterToggleEl: HTMLButtonElement | null;
let filterPanelEl: HTMLElement | null;
let filterDirectoryEl: HTMLInputElement | null;
let filterExitEl: HTMLSelectElement | null;
let filterTimeEl: HTMLSelectElement | null;
let commandPopupEl: HTMLElement | null;
let selectedIndex = -1;
let currentResults: AtuinResult[] = [];
let filtersVisible = false;
let popupVisible = false;

const BASE_HEIGHT = 60;
const FILTER_PANEL_HEIGHT = 72;
const RESULT_HEIGHT = 48;
const CONTAINER_PADDING = 16;
const WINDOW_WIDTH = 700;
const POPUP_WIDTH = 900;

let maxVisibleResults = 20;

interface SearchFilters {
  directory?: string;
  exit_filter?: string;
  time_range?: string;
}

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

function getFilters(): SearchFilters | undefined {
  const filters: SearchFilters = {};
  
  if (filterDirectoryEl?.value) {
    filters.directory = filterDirectoryEl.value;
  }
  if (filterExitEl?.value) {
    filters.exit_filter = filterExitEl.value;
  }
  if (filterTimeEl?.value) {
    filters.time_range = filterTimeEl.value;
  }
  
  return Object.keys(filters).length > 0 ? filters : undefined;
}

function hasActiveFilters(): boolean {
  return !!(
    filterDirectoryEl?.value ||
    filterExitEl?.value ||
    filterTimeEl?.value
  );
}

async function resizeWindow(resultCount: number) {
  if (!isTauri()) return;

  const visibleCount = Math.min(resultCount, maxVisibleResults);
  const resultsHeight = visibleCount > 0 ? visibleCount * RESULT_HEIGHT + CONTAINER_PADDING : 0;
  const filterHeight = filtersVisible ? FILTER_PANEL_HEIGHT : 0;
  const newHeight = BASE_HEIGHT + filterHeight + resultsHeight;

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

function showPopup(result: AtuinResult, rowEl: Element) {
  if (!commandPopupEl) return;
  
  const popupCommand = commandPopupEl.querySelector(".popup-command");
  const popupMeta = commandPopupEl.querySelector(".popup-meta");
  
  if (popupCommand) {
    popupCommand.textContent = result.command;
  }
  
  if (popupMeta) {
    const exitClass = result.exit === "0" ? "exit-success" : "exit-failure";
    const folderIcon = `<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor"><path d="M.54 3.87.5 3a2 2 0 0 1 2-2h3.672a2 2 0 0 1 1.414.586l.828.828A2 2 0 0 0 9.828 3H13.5a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H2.5a2 2 0 0 1-2-2V3.87z"/></svg>`;
    const clockIcon = `<svg width="12" height="12" viewBox="0 0 16 16" fill="currentColor"><path d="M8 3.5a.5.5 0 0 0-1 0V9a.5.5 0 0 0 .252.434l3.5 2a.5.5 0 0 0 .496-.868L8 8.71V3.5z"/><path d="M8 16A8 8 0 1 0 8 0a8 8 0 0 0 0 16zm7-8A7 7 0 1 1 1 8a7 7 0 0 1 14 0z"/></svg>`;
    popupMeta.innerHTML = `
      <span class="popup-meta-item">${folderIcon} ${result.directory}</span>
      <span class="popup-meta-item">${clockIcon} ${result.time}</span>
      <span class="popup-meta-item ${exitClass}">Exit: ${result.exit}</span>
    `;
  }
  
  const rowRect = rowEl.getBoundingClientRect();
  commandPopupEl.style.top = `${rowRect.bottom + 4}px`;
  commandPopupEl.classList.remove("hidden");
  popupVisible = true;
  
  resizeWindowForPopup();
}

function hidePopup() {
  if (!commandPopupEl || !popupVisible) return;
  commandPopupEl.classList.add("hidden");
  popupVisible = false;
  resizeWindow(currentResults.length);
}

async function resizeWindowForPopup() {
  if (!isTauri() || !commandPopupEl) return;
  
  try {
    const window = getCurrentWebviewWindow();
    const popupRect = commandPopupEl.getBoundingClientRect();
    const newHeight = popupRect.bottom + 12;
    await window.setSize(new LogicalSize(POPUP_WIDTH, Math.max(newHeight, 200)));
  } catch (error) {
    console.error("Failed to resize window for popup:", error);
  }
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
    
    row.addEventListener("mouseenter", () => {
      showPopup(result, row);
    });
    row.addEventListener("mouseleave", () => {
      hidePopup();
    });
    
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
    const filters = getFilters();
    const output: string = await invoke("atuin_search_command", { query, filters });
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

function toggleFilters() {
  filtersVisible = !filtersVisible;
  filterPanelEl?.classList.toggle("hidden", !filtersVisible);
  filterToggleEl?.classList.toggle("active", filtersVisible || hasActiveFilters());
  resizeWindow(currentResults.length);
}

function updateFilterToggleState() {
  filterToggleEl?.classList.toggle("active", filtersVisible || hasActiveFilters());
}

async function loadConfig() {
  if (!isTauri()) return;
  
  try {
    const theme: string = await invoke("get_theme");
    if (theme === "light") {
      document.documentElement.classList.add("light");
    } else {
      document.documentElement.classList.remove("light");
    }
    
    const configMaxResults: number = await invoke("get_max_results");
    maxVisibleResults = configMaxResults;
    
    if (atuinResultsEl) {
      atuinResultsEl.style.maxHeight = `${maxVisibleResults * RESULT_HEIGHT}px`;
    }
  } catch (error) {
    console.error("Failed to load config:", error);
  }
}

window.addEventListener("DOMContentLoaded", async () => {
  atuinInputEl = document.querySelector("#atuin-input");
  atuinResultsEl = document.querySelector("#atuin-results");
  filterToggleEl = document.querySelector("#filter-toggle");
  filterPanelEl = document.querySelector("#filter-panel");
  filterDirectoryEl = document.querySelector("#filter-directory");
  filterExitEl = document.querySelector("#filter-exit");
  filterTimeEl = document.querySelector("#filter-time");
  commandPopupEl = document.querySelector("#command-popup");

  await loadConfig();

  if (atuinInputEl) {
    atuinInputEl.addEventListener("input", debounceSearch);
    atuinInputEl.focus();
  }

  filterToggleEl?.addEventListener("click", toggleFilters);
  
  filterDirectoryEl?.addEventListener("input", () => {
    updateFilterToggleState();
    debounceSearch();
  });
  filterExitEl?.addEventListener("change", () => {
    updateFilterToggleState();
    debounceSearch();
  });
  filterTimeEl?.addEventListener("change", () => {
    updateFilterToggleState();
    debounceSearch();
  });

  document.querySelector("#atuin-form")?.addEventListener("submit", (e) => {
    e.preventDefault();
    searchAtuin();
  });

  document.addEventListener("keydown", async (e) => {
    if (e.key === "Escape" && isTauri()) {
      e.preventDefault();
      e.stopPropagation();
      
      if (popupVisible) {
        hidePopup();
        return;
      }
      
      try {
        const window = getCurrentWebviewWindow();
        if (atuinInputEl) atuinInputEl.value = "";
        if (atuinResultsEl) atuinResultsEl.innerHTML = "";
        if (filterDirectoryEl) filterDirectoryEl.value = "";
        if (filterExitEl) filterExitEl.value = "";
        if (filterTimeEl) filterTimeEl.value = "";
        if (filterPanelEl) filterPanelEl.classList.add("hidden");
        filtersVisible = false;
        updateFilterToggleState();
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
      hidePopup();
    }

    if (e.key === "ArrowUp" && currentResults.length > 0) {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      updateSelection();
      hidePopup();
    }

    if (e.key === "h" && selectedIndex >= 0 && selectedIndex < currentResults.length) {
      e.preventDefault();
      const selectedRow = atuinResultsEl?.querySelectorAll(".result-row")[selectedIndex];
      if (selectedRow && !popupVisible) {
        showPopup(currentResults[selectedIndex], selectedRow);
      } else {
        hidePopup();
      }
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
