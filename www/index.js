import * as wasm from "wasm-backend";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import { CanvasAddon } from "@xterm/addon-canvas";

// Detect mobile devices
const isMobile = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent) ||
                 (window.navigator.userAgentData && window.navigator.userAgentData.mobile) ||
                 window.innerWidth <= 768;

var term = wasm.term({
  scrollback: 0,
  theme: {
    background: "#181818",
    foreground: "#EAFFE5",
    cursor: "#EAFFE5",
    black: "#2e3436",
    brightBlack: "#555753",
    red: "#cc0000",
    brightRed: "#ef2929",
    green: "#4e9a06",
    brightGreen: "#8ae234",
    yellow: "#c4a000",
    brightYellow: "#fce94f",
    blue: "#3465a4",
    brightBlue: "#729fcf",
    magenta: "#75507b",
    brightMagenta: "#ad7fa8",
    cyan: "#06989a",
    brightCyan: "#34e2e2",
    white: "#d3d7cf",
    brightWhite: "#e6e6e6",
  },
  fontFamily: "Source Code Pro Variable",
  fontSize: isMobile ? 12 : 15,
  cols: 80,
});
window.term = term;

const fitAddon = new FitAddon();
term.loadAddon(new WebLinksAddon());
term.loadAddon(new CanvasAddon());
term.loadAddon(new WebglAddon());
term.loadAddon(fitAddon);

// Touch swipe handling for mobile
let touchStartX = 0;
let touchStartY = 0;
let touchEndX = 0;
let touchEndY = 0;
const swipeThreshold = 50; // Minimum swipe distance in pixels
let lastTouchY = 0;
let scrollAccumulator = 0;
let isScrolling = false;
let touchMoveCount = 0;
const minScrollDistance = 10; // Minimum distance before starting scroll
let lastScrollTop = 0;

function handleTouchStart(e) {
  const touch = e.touches[0];
  touchStartX = touch.clientX;
  touchStartY = touch.clientY;
  lastTouchY = touch.clientY;
  scrollAccumulator = 0;
  isScrolling = false;
  touchMoveCount = 0;
}

function handleTouchMove(e) {
  const touch = e.touches[0];
  const currentY = touch.clientY;
  touchMoveCount++;
  
  // Check if we've moved enough to start scrolling
  const totalDeltaY = Math.abs(currentY - touchStartY);
  if (!isScrolling && totalDeltaY < minScrollDistance) {
    return; // Not enough movement yet
  }
  isScrolling = true;
  
  const deltaY = lastTouchY - currentY;
  lastTouchY = currentY;
  
  scrollAccumulator += deltaY;
  
  // Variable scroll sensitivity based on speed
  const absDelta = Math.abs(deltaY);
  let scrollSensitivity;
  if (absDelta > 30) {
    scrollSensitivity = 8; // Fast scroll - more responsive
  } else if (absDelta > 15) {
    scrollSensitivity = 15; // Medium scroll
  } else {
    scrollSensitivity = 25; // Slow scroll - more precise
  }
  
  // Send scroll based on accumulated movement
  while (Math.abs(scrollAccumulator) >= scrollSensitivity) {
    if (scrollAccumulator > 0) {
      // Scrolling down
      wasm.scroll(-1);
      scrollAccumulator -= scrollSensitivity;
    } else {
      // Scrolling up
      wasm.scroll(1);
      scrollAccumulator += scrollSensitivity;
    }
  }
}

function handleTouchEnd(e) {
  touchEndX = e.changedTouches[0].screenX;
  touchEndY = e.changedTouches[0].screenY;
  
  // Only handle swipe if we weren't scrolling
  if (!isScrolling) {
    handleSwipe();
  }
  
  scrollAccumulator = 0;
  isScrolling = false;
  touchMoveCount = 0;
}

function handleSwipe() {
  const deltaX = touchEndX - touchStartX;
  const deltaY = touchEndY - touchStartY;

  // Determine if horizontal or vertical swipe
  if (Math.abs(deltaX) > Math.abs(deltaY)) {
    // Horizontal swipe
    if (Math.abs(deltaX) > swipeThreshold) {
      if (deltaX > 0) {
        // Swipe right - send right arrow
        term.input('\x1b[C', false);
      } else {
        // Swipe left - send left arrow
        term.input('\x1b[D', false);
      }
    }
  }
}

function updateAutocomplete() {
  if (!isMobile) return;

  const suggestionsBar = document.getElementById('command-suggestions');
  if (!suggestionsBar) return;

  try {
    const options = wasm.autocomplete();

    // Clear existing buttons except static ones if no options
    if (!options || options.length === 0) {
      // Smart autocomplete: empty list means auto-submit
      term.input('\r');
      // Show default commands after submit
      suggestionsBar.innerHTML = `
        <button class="cmd-btn" data-cmd="help">help</button>
        <button class="cmd-btn" data-cmd="ls">ls</button>
        <button class="cmd-btn" data-cmd="cd ">cd</button>
        <button class="cmd-btn" data-cmd="cat ">cat</button>
        <button class="cmd-btn" data-cmd="less ">less</button>
        <button class="cmd-btn" data-cmd="clear">clear</button>
      `;
    } else {
      // Show autocomplete options
      suggestionsBar.innerHTML = options
        .slice(0, 10) // Limit to 10 options
        .map(opt => `<button class="cmd-btn autocomplete-option" data-complete="${opt}">${opt}</button>`)
        .join('');
    }

    // Re-attach event listeners
    attachCommandListeners();

    // Center the scrollable content
    setTimeout(() => {
      suggestionsBar.scrollLeft = (suggestionsBar.scrollWidth - suggestionsBar.clientWidth) / 2;
    }, 50);
  } catch (e) {
    console.error('Autocomplete error:', e);
  }
}

function attachCommandListeners() {
  const suggestionsBar = document.getElementById('command-suggestions');
  if (!suggestionsBar) return;

  const commandButtons = suggestionsBar.querySelectorAll('.cmd-btn');
  commandButtons.forEach(btn => {
    btn.addEventListener('click', (e) => {
      e.preventDefault();

      // Check if it's a completion or command
      const complete = btn.getAttribute('data-complete');
      const command = btn.getAttribute('data-cmd');

      if (complete) {
        // It's an autocomplete option - type it directly
        // Don't add space if it's a directory (ends with /)
        if (complete.endsWith('/')) {
          term.input(complete);
        } else {
          term.input(complete + " ");
        }
      } else if (command) {
        // It's a command - type it out
        term.input(command);
      }

      // Don't focus on mobile to prevent keyboard
      if (!isMobile) {
        term.focus();
      }
    });
  });
}

function init() {
  let domterm = document.getElementById("terminal");
  domterm.innerText = "";
  term.open(domterm);
  fitAddon.fit();
  wasm.init(term.rows, term.cols, window.location.pathname);

  // Don't focus on mobile to prevent keyboard
  if (!isMobile) {
    term.focus();
  }

  // Add touch listeners for mobile
  if (isMobile) {
    // Disable the hidden textarea to prevent keyboard
    const textarea = domterm.querySelector('.xterm-helper-textarea');
    if (textarea) {
      textarea.setAttribute('readonly', 'readonly');
      textarea.setAttribute('inputmode', 'none');
      textarea.style.display = 'none';
    }

    // Add touch listeners to terminal for scroll and swipe gestures
    domterm.addEventListener('touchstart', handleTouchStart, { passive: true });
    domterm.addEventListener('touchmove', handleTouchMove, { passive: true });
    domterm.addEventListener('touchend', handleTouchEnd, { passive: true });

    // Show command suggestions bar on mobile
    const suggestionsBar = document.getElementById('command-suggestions');
    if (suggestionsBar) {
      suggestionsBar.classList.add('show');
      attachCommandListeners();
      updateAutocomplete();

      // Center the scrollable content initially
      setTimeout(() => {
        suggestionsBar.scrollLeft = (suggestionsBar.scrollWidth - suggestionsBar.clientWidth) / 2;
      }, 150);
    }


  }
}

var loaded =
  document.readyState === "complete" || document.readyState === "interactive";
document.addEventListener("DOMContentLoaded", () => {
  loaded = true;
  if (font) init();
});

var font = false;
document.fonts.ready.then(() => {
  font = true;
  if (loaded) init();
});

addEventListener("resize", () => {
  fitAddon.fit();
  wasm.init(term.rows, term.cols, window.location.pathname);
});

term.onData(function (data) {
  wasm.readline(data);
  setTimeout(() => updateAutocomplete(), 10);
});

term.onScroll(function (newScrollTop) {
  const delta = newScrollTop - lastScrollTop;
  lastScrollTop = newScrollTop;

  if (delta !== 0) {
    wasm.scroll(delta);
  }
});
