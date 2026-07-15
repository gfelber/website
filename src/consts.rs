// ANSI
pub const UP: &str = "\x1b\x5b\x41";
pub const DOWN: &str = "\x1b\x5b\x42";
pub const RIGHT: &str = "\x1b\x5b\x43";
pub const LEFT: &str = "\x1b\x5b\x44";
pub const PAGE_DOWN: &str = "\x1b\x5b\x36\x7e";
pub const PAGE_UP: &str = "\x1b\x5b\x35\x7e";
pub const PAGE_START: &str = "\x1b\x5b\x48";
pub const PAGE_END: &str = "\x1b\x5b\x46";
pub const INSERT: &str = "\x1b\x5b\x32\x7e";
pub const RETURN: &str = "\x1b\x5b\x44 \x1b\x5b\x44";
pub const NEWLINE: &str = "\n\r";
pub const PREFIX: &str = "$ ";
// kept short so lines don't wrap on mobile (~59 cols)
pub const BANNER: &[&str] = &[
  "[    0.000000] Linux version 6.12.27 (gfelber@website)",
  "[    0.000000] Kernel command line: console=ttyS0 quiet",
  "[    0.031337] random: crng init done",
  "[    0.133700] SCSI subsystem initialized",
  "[    0.267350] wasm0: xterm.js console attached on tty1",
  "[    0.313370] Freeing unused kernel memory: 1337K",
  "[    0.404096] Run /bin/sh as init process",
  "[    0.421337] tip: type 'help' to list commands",
];
// Function Keys
pub const F1: &str = "\x1b\x4f\x50";
pub const F2: &str = "\x1b\x4f\x51";
pub const F3: &str = "\x1b\x4f\x52";
pub const F4: &str = "\x1b\x4f\x53";
