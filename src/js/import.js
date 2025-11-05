import {Terminal} from '@xterm/xterm';

var term;

export function create_term(options) {
    term = new Terminal(options);
    return term
}

export function term_write(out) {
    term.write(out)
}
