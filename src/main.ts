import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { LogicalSize } from "@tauri-apps/api/dpi";
import type { AtuinResult, ExitFilter, SearchFilters, TimeRange } from "./bindings";

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
let commandPopupCommandEl: HTMLElement | null;
let selectedIndex = -1;
let currentResults: AtuinResult[] = [];
let filtersVisible = false;
let searchTimeout: ReturnType<typeof setTimeout> | null = null;
let searchGeneration = 0;

const BASE_HEIGHT = 38;
const FILTER_PANEL_HEIGHT = 56;
const RESULT_HEIGHT = 32;
const CONTAINER_PADDING = 8;
const COMMAND_POPUP_EXTRA_WIDTH = 160;

let maxVisibleResults = 20;
let windowWidth = 700;
let commandPopupVisible = false;

function formatRelativeTime(timestamp: string): string {
  try {
    const date = new Date(timestamp);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffSecs = Math.floor(diffMs / 1000);
    const diffMins = Math.floor(diffSecs / 60);
    const diffHours = Math.floor(diffMins / 60);
    const diffDays = Math.floor(diffHours / 24);
    const diffMonths = Math.floor(diffDays / 30);
    const diffYears = Math.floor(diffDays / 365);

    if (diffYears > 0) return `${diffYears}y`;
    if (diffMonths > 0) return `${diffMonths}mo`;
    if (diffDays > 0) return `${diffDays}d`;
    if (diffHours > 0) return `${diffHours}h`;
    if (diffMins > 0) return `${diffMins}m`;
    return `${diffSecs}s`;
  } catch {
    return timestamp;
  }
}

function getFilters(): SearchFilters | undefined {
  const filters: SearchFilters = {
    directory: null,
    exit_filter: null,
    time_range: null,
  };

  if (filterDirectoryEl?.value) {
    filters.directory = filterDirectoryEl.value;
  }
  if (filterExitEl?.value) {
    filters.exit_filter = filterExitEl.value as ExitFilter;
  }
  if (filterTimeEl?.value) {
    filters.time_range = filterTimeEl.value as TimeRange;
  }

  return hasActiveFilters() ? filters : undefined;
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
  const resultsHeight =
    visibleCount > 0 ? visibleCount * RESULT_HEIGHT + CONTAINER_PADDING : 0;
  const filterHeight = filtersVisible ? FILTER_PANEL_HEIGHT : 0;
  const newHeight = BASE_HEIGHT + filterHeight + resultsHeight;
  const newWidth = commandPopupVisible
    ? windowWidth + COMMAND_POPUP_EXTRA_WIDTH
    : windowWidth;

  try {
    const window = getCurrentWebviewWindow();
    await window.setSize(new LogicalSize(newWidth, newHeight));
  } catch (error) {
    console.error("Failed to resize window:", error);
  }
}

function updateCommandPopup(showPopup: boolean) {
  if (!commandPopupEl || !commandPopupCommandEl || !atuinResultsEl) return;

  const selected = currentResults[selectedIndex];
  const selectedRow = atuinResultsEl.querySelectorAll(".result-row")[
    selectedIndex
  ] as HTMLElement | undefined;
  commandPopupVisible = showPopup && !!selected && !!selectedRow;
  commandPopupEl.classList.toggle("hidden", !commandPopupVisible);

  if (!commandPopupVisible || !selected || !selectedRow) {
    commandPopupCommandEl.textContent = "";
    return;
  }

  commandPopupCommandEl.textContent = selected.command;
  const rowRect = selectedRow.getBoundingClientRect();
  commandPopupEl.style.top = `${rowRect.top}px`;
  commandPopupEl.style.left = "0px";
}

function updateSelection(showPopup = commandPopupVisible) {
  if (!atuinResultsEl) return;
  const rows = atuinResultsEl.querySelectorAll(".result-row");
  rows.forEach((row, index) => {
    const isSelected = index === selectedIndex;
    row.classList.toggle("selected", isSelected && !showPopup);
    row.classList.toggle("preview-source", isSelected && showPopup);
    if (isSelected) {
      row.scrollIntoView({ block: "nearest" });
    }
  });
  updateCommandPopup(showPopup);
  void resizeWindow(currentResults.length);
}

async function clearResults() {
  if (atuinResultsEl) {
    atuinResultsEl.innerHTML = "";
  }
  currentResults = [];
  selectedIndex = -1;
  updateCommandPopup(false);
  await resizeWindow(0);
}

function clearPendingSearch() {
  if (searchTimeout) {
    clearTimeout(searchTimeout);
    searchTimeout = null;
  }
}

function invalidateSearch(): number {
  searchGeneration += 1;
  clearPendingSearch();
  return searchGeneration;
}

function isCurrentSearch(generation: number): boolean {
  return generation === searchGeneration;
}

function renderResults(results: AtuinResult[]) {
  if (!atuinResultsEl) return;

  const resultsContainer = atuinResultsEl;
  resultsContainer.innerHTML = "";
  currentResults = results;
  selectedIndex = results.length > 0 ? 0 : -1;
  updateCommandPopup(false);

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
    const relativeTime = formatRelativeTime(result.time);
    metaEl.innerHTML = `<span class="${exitClass}">${result.duration}</span> <span class="result-time">${relativeTime}</span>`;

    row.appendChild(commandEl);
    row.appendChild(metaEl);

    resultsContainer.appendChild(row);
  });

  resizeWindow(results.length);
}

async function searchAtuin(generation = invalidateSearch()) {
  if (!atuinInputEl || !atuinResultsEl) return;

  const query = atuinInputEl.value.trim();
  console.log("searchAtuin called with query:", query);

  if (!query) {
    await clearResults();
    return;
  }

  if (!isTauri()) {
    console.error("Not running in Tauri context");
    return;
  }

  try {
    console.log("Invoking atuin_search_command...");
    const filters = getFilters();
    const results: AtuinResult[] = await invoke("atuin_search_command", {
      query,
      filters,
    });

    if (!isCurrentSearch(generation)) {
      return;
    }

    console.log("Got results:", results.length);

    if (results.length === 0) {
      await clearResults();
      return;
    }

    renderResults(results.reverse());
  } catch (error) {
    console.error("Atuin search error:", error);
    if (isCurrentSearch(generation)) {
      await clearResults();
    }
  }
}

function debounceSearch() {
  const generation = invalidateSearch();
  updateCommandPopup(false);
  void resizeWindow(currentResults.length);
  if (!atuinInputEl?.value.trim()) {
    void clearResults();
    return;
  }

  searchTimeout = setTimeout(() => {
    searchTimeout = null;
    void searchAtuin(generation);
  }, 150);
}

function toggleFilters() {
  filtersVisible = !filtersVisible;
  filterPanelEl?.classList.toggle("hidden", !filtersVisible);
  filterToggleEl?.classList.toggle(
    "active",
    filtersVisible || hasActiveFilters(),
  );
  resizeWindow(currentResults.length);
}

function updateFilterToggleState() {
  filterToggleEl?.classList.toggle(
    "active",
    filtersVisible || hasActiveFilters(),
  );
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

    const configWindowWidth: number = await invoke("get_window_width");
    windowWidth = configWindowWidth;

    document.documentElement.style.setProperty(
      "--atuin-window-width",
      `${windowWidth}px`,
    );
    document.documentElement.style.setProperty(
      "--command-popup-width",
      `${windowWidth + COMMAND_POPUP_EXTRA_WIDTH}px`,
    );

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
  commandPopupCommandEl = document.querySelector("#command-popup-command");
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
    void searchAtuin();
  });

  document.addEventListener("keydown", async (e) => {
    if (e.key === "Escape" && isTauri()) {
      e.preventDefault();
      e.stopPropagation();

      try {
        const window = getCurrentWebviewWindow();
        invalidateSearch();
        if (atuinInputEl) atuinInputEl.value = "";
        if (filterDirectoryEl) filterDirectoryEl.value = "";
        if (filterExitEl) filterExitEl.value = "";
        if (filterTimeEl) filterTimeEl.value = "";
        if (filterPanelEl) filterPanelEl.classList.add("hidden");
        filtersVisible = false;
        updateFilterToggleState();
        await clearResults();
        await window.hide();
      } catch (error) {
        console.error("Failed to hide window:", error);
      }
    }

    if (e.key === "ArrowDown" && currentResults.length > 0) {
      e.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, currentResults.length - 1);
      updateSelection(true);
    }

    if (e.key === "ArrowUp" && currentResults.length > 0) {
      e.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
      updateSelection(true);
    }

    if (
      e.key === "Enter" &&
      selectedIndex >= 0 &&
      selectedIndex < currentResults.length
    ) {
      e.preventDefault();
      const selected = currentResults[selectedIndex];
      try {
        await invoke("copy_to_clipboard", { text: selected.command });
        const window = getCurrentWebviewWindow();
        invalidateSearch();
        if (atuinInputEl) atuinInputEl.value = "";
        await clearResults();
        await window.hide();
      } catch (error) {
        console.error("Failed to copy to clipboard:", error);
      }
    }
  });

  void clearResults();
});
