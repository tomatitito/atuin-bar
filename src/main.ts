import { invoke } from "@tauri-apps/api/core";

let greetInputEl: HTMLInputElement | null;
let greetMsgEl: HTMLElement | null;
let clipboardInputEl: HTMLInputElement | null;
let clipboardMsgEl: HTMLElement | null;

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
});
