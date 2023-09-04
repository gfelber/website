import * as wasm from "wasm-backend";
import { Terminal } from 'xterm';
import { FitAddon } from 'xterm-addon-fit';

var term = new Terminal();
const fitAddon = new FitAddon();

term.loadAddon(fitAddon);
term.open(document.getElementById('terminal'));
fitAddon.fit();

term.write(wasm.init(term.rows, term.cols));
term.onData(function(data) {
  term.write(wasm.readline(data));
});
