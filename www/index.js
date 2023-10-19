import * as wasm from "wasm-backend";
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';
import { WebLinksAddon } from 'xterm-addon-web-links';
import { WebglAddon } from 'xterm-addon-webgl';
import { CanvasAddon } from 'xterm-addon-canvas';
var term = wasm.term({
  theme: {
    background: '#181818',
    foreground: '#EAFFE5',
    cursor: '#EAFFE5',
    black:	'#2e3436',
    brightBlack:	'#555753',
    red:	'#cc0000',
    brightRed:	'#ef2929',
    green:	'#4e9a06',
    brightGreen:	'#8ae234',
    yellow:	'#c4a000',
    brightYellow:	'#fce94f',
    blue:	'#3465a4',
    brightBlue:	'#729fcf',
    magenta:	'#75507b',
    brightMagenta:	'#ad7fa8',
    cyan:	'#06989a',
    brightCyan:	'#34e2e2',
    white:	'#d3d7cf',
    brightWhite:	'#e6e6e6',
  },
  fontFamily: 'Source Code Pro Variable',
  fontSize: 13
});
const fitAddon = new FitAddon();
term.loadAddon(new WebLinksAddon());
term.loadAddon(new CanvasAddon());
term.loadAddon(new WebglAddon());
term.loadAddon(fitAddon);
const is_chrome_mobile = window.chrome && navigator.userAgentData.mobile
console.log(is_chrome_mobile)

function init() {
  let domterm = document.getElementById('terminal');
  domterm.innerText = "";
  term.open(domterm);
  fitAddon.fit();
  wasm.init(term.rows, term.cols, window.location.pathname);
}

var loaded = document.readyState === "complete" || document.readyState === "interactive"
document.addEventListener('DOMContentLoaded', () => {
  loaded = true;
  if (font) init();
})

var font = false
document.fonts.ready.then(() => {
  font = true
  if (loaded) init();
});

addEventListener("resize", () => {
  fitAddon.fit();
  wasm.init(term.rows, term.cols, window.location.pathname);
});


term.onData(function(data) {
  wasm.readline(data);
});
