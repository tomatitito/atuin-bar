/**
 * Zed-Style Command Palette Integration for Tauri + Atuin
 * 
 * This JavaScript file shows how to integrate the Zed-style design
 * with your Tauri app and Atuin shell history backend.
 */

// Import Tauri API (for Tauri apps)
// const { invoke } = window.__TAURI__.tauri;

class ZedCommandPalette {
  constructor(options = {}) {
    this.options = {
      maxResults: 50,
      debounceMs: 150,
      ...options
    };
    
    this.selectedIndex = 0;
    this.commands = [];
    this.filteredCommands = [];
    this.isOpen = false;
    this.searchDebouncer = null;
    
    this.init();
  }

  init() {
    // Create DOM structure
    this.createElements();
    
    // Bind event handlers
    this.bindEvents();
    
    // Load initial data from Atuin
    this.loadHistory();
  }

  createElements() {
    // Create overlay
    this.overlay = document.createElement('div');
    this.overlay.className = 'command-palette-overlay';
    this.overlay.style.display = 'none';
    
    // Create palette container
    const palette = document.createElement('div');
    palette.className = 'command-palette';
    
    // Create header with input
    const header = document.createElement('div');
    header.className = 'command-palette-header';
    
    this.input = document.createElement('input');
    this.input.type = 'text';
    this.input.className = 'command-palette-input';
    this.input.placeholder = 'Search shell history...';
    
    header.appendChild(this.input);
    
    // Create results container
    this.resultsContainer = document.createElement('div');
    this.resultsContainer.className = 'command-palette-results';
    
    // Create footer (optional)
    this.footer = document.createElement('div');
    this.footer.className = 'command-palette-footer';
    this.footer.innerHTML = `
      <button class="footer-button" data-action="copy">
        Copy <span style="opacity: 0.6; margin-left: 4px;">⌘C</span>
      </button>
      <button class="footer-button primary" data-action="run">
        Run <span style="opacity: 0.8; margin-left: 4px;">↵</span>
      </button>
    `;
    
    // Assemble palette
    palette.appendChild(header);
    palette.appendChild(this.resultsContainer);
    palette.appendChild(this.footer);
    
    // Add to overlay
    this.overlay.appendChild(palette);
    
    // Add to document
    document.body.appendChild(this.overlay);
  }

  bindEvents() {
    // Input events
    this.input.addEventListener('input', (e) => this.handleSearch(e.target.value));
    this.input.addEventListener('keydown', (e) => this.handleKeydown(e));
    
    // Click outside to close
    this.overlay.addEventListener('click', (e) => {
      if (e.target === this.overlay) {
        this.close();
      }
    });
    
    // Footer button events
    this.footer.addEventListener('click', (e) => {
      const button = e.target.closest('[data-action]');
      if (button) {
        const action = button.dataset.action;
        if (action === 'run') {
          this.executeSelected();
        } else if (action === 'copy') {
          this.copySelected();
        }
      }
    });
    
    // Global keyboard shortcut to open (Cmd/Ctrl + Shift + P)
    document.addEventListener('keydown', (e) => {
      if ((e.metaKey || e.ctrlKey) && e.shiftKey && e.key === 'P') {
        e.preventDefault();
        this.toggle();
      }
    });
  }

  handleKeydown(e) {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        this.selectNext();
        break;
      case 'ArrowUp':
        e.preventDefault();
        this.selectPrevious();
        break;
      case 'Enter':
        e.preventDefault();
        if (e.shiftKey && e.metaKey) {
          // Secondary action - edit keybinding
          console.log('Edit keybinding for:', this.getSelectedCommand());
        } else {
          this.executeSelected();
        }
        break;
      case 'Escape':
        e.preventDefault();
        this.close();
        break;
      case 'c':
        if (e.metaKey || e.ctrlKey) {
          e.preventDefault();
          this.copySelected();
        }
        break;
    }
  }

  handleSearch(query) {
    // Debounce search
    clearTimeout(this.searchDebouncer);
    this.searchDebouncer = setTimeout(() => {
      this.search(query);
    }, this.options.debounceMs);
  }

  async search(query) {
    if (!query.trim()) {
      // Show recent history
      this.filteredCommands = this.commands.slice(0, this.options.maxResults);
    } else {
      // Call Tauri backend to search Atuin
      try {
        // For Tauri integration:
        // const results = await invoke('search_history', { query });
        // this.filteredCommands = results;
        
        // For demo: client-side filter
        this.filteredCommands = this.fuzzySearch(query);
      } catch (error) {
        console.error('Search error:', error);
        this.filteredCommands = [];
      }
    }
    
    this.selectedIndex = 0;
    this.render();
  }

  fuzzySearch(query) {
    const lowerQuery = query.toLowerCase();
    const results = [];
    
    for (const cmd of this.commands) {
      const lowerCmd = cmd.command.toLowerCase();
      if (lowerCmd.includes(lowerQuery)) {
        // Calculate match positions for highlighting
        const matchPositions = [];
        let index = lowerCmd.indexOf(lowerQuery);
        while (index !== -1) {
          matchPositions.push({ start: index, end: index + lowerQuery.length });
          index = lowerCmd.indexOf(lowerQuery, index + 1);
        }
        
        results.push({
          ...cmd,
          matchPositions,
          score: this.calculateScore(cmd, lowerQuery)
        });
      }
    }
    
    // Sort by score (relevance)
    results.sort((a, b) => b.score - a.score);
    
    return results.slice(0, this.options.maxResults);
  }

  calculateScore(cmd, query) {
    let score = 0;
    
    // Boost for recent commands
    const daysSinceUse = (Date.now() - cmd.timestamp) / (1000 * 60 * 60 * 24);
    score += Math.max(0, 100 - daysSinceUse);
    
    // Boost for frequency
    score += cmd.frequency * 10;
    
    // Boost for exact match
    if (cmd.command.toLowerCase() === query) {
      score += 1000;
    }
    
    // Boost for starts with query
    if (cmd.command.toLowerCase().startsWith(query)) {
      score += 500;
    }
    
    return score;
  }

  render() {
    this.resultsContainer.innerHTML = '';
    
    if (this.filteredCommands.length === 0) {
      this.resultsContainer.innerHTML = `
        <div class="command-palette-empty">
          No matching commands found
        </div>
      `;
      return;
    }
    
    // Group by sections (recent, frequent, all)
    const recent = [];
    const frequent = [];
    const all = [];
    
    const now = Date.now();
    const recentThreshold = 24 * 60 * 60 * 1000; // 24 hours
    
    for (const cmd of this.filteredCommands) {
      if (now - cmd.timestamp < recentThreshold) {
        recent.push(cmd);
      } else if (cmd.frequency > 5) {
        frequent.push(cmd);
      } else {
        all.push(cmd);
      }
    }
    
    // Render sections
    let itemIndex = 0;
    
    if (recent.length > 0) {
      this.renderSection('Recent', recent.slice(0, 5), itemIndex);
      itemIndex += Math.min(recent.length, 5);
    }
    
    if (frequent.length > 0) {
      this.renderSection('Frequently Used', frequent.slice(0, 5), itemIndex);
      itemIndex += Math.min(frequent.length, 5);
    }
    
    if (all.length > 0) {
      this.renderSection('All Commands', all, itemIndex);
    }
    
    // Set up click handlers
    this.setupItemHandlers();
  }

  renderSection(title, commands, startIndex) {
    // Add section divider
    const divider = document.createElement('div');
    divider.className = 'command-section-divider';
    divider.textContent = title;
    this.resultsContainer.appendChild(divider);
    
    // Add items
    commands.forEach((cmd, i) => {
      const item = this.createCommandItem(cmd, startIndex + i);
      this.resultsContainer.appendChild(item);
    });
  }

  createCommandItem(cmd, index) {
    const item = document.createElement('div');
    item.className = 'command-item';
    item.dataset.index = index;
    
    if (index === this.selectedIndex) {
      item.classList.add('selected');
    }
    
    // Create label
    const label = document.createElement('div');
    label.className = 'command-item-label';
    
    // Create text with highlighting
    const text = document.createElement('span');
    text.className = 'command-item-text';
    
    if (cmd.matchPositions && cmd.matchPositions.length > 0) {
      // Highlight matches
      let html = '';
      let lastEnd = 0;
      
      for (const pos of cmd.matchPositions) {
        html += this.escapeHtml(cmd.command.substring(lastEnd, pos.start));
        html += '<mark>' + this.escapeHtml(cmd.command.substring(pos.start, pos.end)) + '</mark>';
        lastEnd = pos.end;
      }
      html += this.escapeHtml(cmd.command.substring(lastEnd));
      
      text.innerHTML = html;
    } else {
      text.textContent = cmd.command;
    }
    
    label.appendChild(text);
    item.appendChild(label);
    
    // Add keybinding hint (if item is in top 9)
    if (index < 9) {
      const keybinding = document.createElement('div');
      keybinding.className = 'command-item-keybinding';
      
      const key = document.createElement('span');
      key.className = 'keybinding-key';
      key.textContent = (index + 1).toString();
      
      keybinding.appendChild(key);
      item.appendChild(keybinding);
    }
    
    return item;
  }

  setupItemHandlers() {
    const items = this.resultsContainer.querySelectorAll('.command-item');
    
    items.forEach((item) => {
      item.addEventListener('click', () => {
        this.selectedIndex = parseInt(item.dataset.index);
        this.executeSelected();
      });
      
      item.addEventListener('mouseenter', () => {
        this.selectedIndex = parseInt(item.dataset.index);
        this.updateSelection();
      });
    });
  }

  selectNext() {
    const items = this.resultsContainer.querySelectorAll('.command-item');
    if (this.selectedIndex < items.length - 1) {
      this.selectedIndex++;
      this.updateSelection();
    }
  }

  selectPrevious() {
    if (this.selectedIndex > 0) {
      this.selectedIndex--;
      this.updateSelection();
    }
  }

  updateSelection() {
    const items = this.resultsContainer.querySelectorAll('.command-item');
    
    items.forEach((item, index) => {
      if (index === this.selectedIndex) {
        item.classList.add('selected');
        item.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
      } else {
        item.classList.remove('selected');
      }
    });
  }

  getSelectedCommand() {
    if (this.filteredCommands[this.selectedIndex]) {
      return this.filteredCommands[this.selectedIndex].command;
    }
    return null;
  }

  async executeSelected() {
    const command = this.getSelectedCommand();
    if (command) {
      console.log('Executing:', command);
      
      // For Tauri:
      // await invoke('execute_command', { command });
      
      // Update frequency in Atuin
      // await invoke('update_frequency', { command });
      
      this.close();
    }
  }

  async copySelected() {
    const command = this.getSelectedCommand();
    if (command) {
      try {
        await navigator.clipboard.writeText(command);
        console.log('Copied:', command);
        
        // Show brief feedback
        this.showFeedback('Copied to clipboard');
      } catch (error) {
        console.error('Copy failed:', error);
      }
    }
  }

  showFeedback(message) {
    // Create temporary feedback element
    const feedback = document.createElement('div');
    feedback.style.cssText = `
      position: fixed;
      bottom: 20px;
      right: 20px;
      padding: 8px 16px;
      background: var(--text-accent);
      color: white;
      border-radius: 4px;
      font-size: 13px;
      z-index: 10000;
      animation: slideIn 200ms ease-out;
    `;
    feedback.textContent = message;
    
    document.body.appendChild(feedback);
    
    setTimeout(() => {
      feedback.style.animation = 'fadeOut 200ms ease-out';
      setTimeout(() => feedback.remove(), 200);
    }, 2000);
  }

  async loadHistory() {
    // Load from Atuin via Tauri
    try {
      // For Tauri:
      // this.commands = await invoke('get_history', { limit: 1000 });
      
      // For demo: mock data
      this.commands = this.generateMockData();
      
      // Show recent by default
      this.filteredCommands = this.commands.slice(0, this.options.maxResults);
      this.render();
    } catch (error) {
      console.error('Failed to load history:', error);
    }
  }

  generateMockData() {
    // Mock data for demonstration
    return [
      { command: 'git commit -m "fix: update dependencies"', timestamp: Date.now() - 1000 * 60 * 5, frequency: 15 },
      { command: 'npm run dev', timestamp: Date.now() - 1000 * 60 * 30, frequency: 50 },
      { command: 'cargo build --release', timestamp: Date.now() - 1000 * 60 * 60, frequency: 20 },
      { command: 'docker ps -a', timestamp: Date.now() - 1000 * 60 * 120, frequency: 10 },
      { command: 'kubectl get pods --all-namespaces', timestamp: Date.now() - 1000 * 60 * 180, frequency: 8 },
      { command: 'git push origin main', timestamp: Date.now() - 1000 * 60 * 240, frequency: 30 },
      { command: 'npm install', timestamp: Date.now() - 1000 * 60 * 360, frequency: 25 },
      { command: 'docker-compose up -d', timestamp: Date.now() - 1000 * 60 * 480, frequency: 12 },
      { command: 'ssh user@server.com', timestamp: Date.now() - 1000 * 60 * 600, frequency: 5 },
      { command: 'git status', timestamp: Date.now() - 1000 * 60 * 720, frequency: 100 },
    ];
  }

  escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
  }

  open() {
    this.isOpen = true;
    this.overlay.style.display = 'flex';
    
    // Reset state
    this.input.value = '';
    this.selectedIndex = 0;
    
    // Load fresh data
    this.loadHistory();
    
    // Focus input
    setTimeout(() => this.input.focus(), 50);
  }

  close() {
    this.isOpen = false;
    this.overlay.style.display = 'none';
    this.input.value = '';
  }

  toggle() {
    if (this.isOpen) {
      this.close();
    } else {
      this.open();
    }
  }
}

// Initialize when DOM is ready
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', () => {
    window.commandPalette = new ZedCommandPalette();
  });
} else {
  window.commandPalette = new ZedCommandPalette();
}

// Export for module usage
if (typeof module !== 'undefined' && module.exports) {
  module.exports = ZedCommandPalette;
}