import * as wasm from "wasm-backend";
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';

var term = new Terminal({
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
  'fontFamily': 'Source Code Pro',
  'fontSize': 13
});
const fitAddon = new FitAddon();

term.loadAddon(fitAddon);
term.open(document.getElementById('terminal'));
fitAddon.fit();

term.write(wasm.init(term.rows, term.cols, window.location.pathname));
term.onData(function(data) {
  term.write(wasm.readline(data));
});
