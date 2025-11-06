```Dockerfile
FROM ubuntu:24.04@sha256:7c06e91f61fa88c08cc74f7e1b7c69ae24910d745357e0dfe1d2c0322aaf20f9

ADD --chmod=0755 --checksum=sha256:c125df9762b0c7233459087bb840c0e5dbfc4d9690ee227f1ed8994f4d51d2e0 \
    https://raw.githubusercontent.com/reproducible-containers/repro-sources-list.sh/v0.1.4/repro-sources-list.sh \
    /usr/local/bin/repro-sources-list.sh
RUN repro-sources-list.sh

RUN apt-get update && apt -y install socat qemu-system-aarch64 python3 python3-requests xz-utils iproute2 iptables

RUN useradd --create-home --shell /bin/bash ctf
WORKDIR /home/ctf

COPY share/run.sh share/Image share/rootfs.cpio.gz share/proxy.py share/hamal.ext4.xz share/mesarthim /home/ctf/
RUN unxz /home/ctf/hamal.ext4.xz

RUN chmod 555 /home/ctf && \
    chown -R root:root /home/ctf/ && \
    chmod 000 /home/ctf/* && \
    chmod 555 /home/ctf/run.sh && \
    chmod 555 /home/ctf/proxy.py && \
    chmod 555 /home/ctf/mesarthim && \
    chmod 444 /home/ctf/Image && \
    chmod 444 /home/ctf/hamal.ext4 && \
    chmod 444 /home/ctf/rootfs.cpio.gz

EXPOSE 1337

RUN rm /usr/lib/systemd/system/*.service /etc/systemd/system-generators/systemd-gpt-auto-generator
USER ctf
RUN ! find / -writable -or -user $(id -un) -or -group $(id -Gn|sed -e 's/ / -or -group /g') 2> /dev/null | grep -Ev -m 1 '^(/dev/|/run/|/proc/|/sys/|/tmp|/var/tmp|/var/lock)'

USER root

CMD printenv FLAG3 > flag; \
    ip tuntap add dev tap0 mode tap user ctf; \
    ip addr add 192.168.111.1/24 dev tap0; \
    ip link set tap0 up; \
    iptables -t mangle -A OUTPUT -m owner --uid-owner ctf -d 192.168.111.0/24 -j MARK --set-mark 0x6f; \
    iptables -A OUTPUT -m mark --mark 0x6f ! -o tap0 -j DROP; \
    su ctf -c './mesarthim' & \
    su ctf -c 'python3 proxy.py' & \
    su ctf -c './run.sh'
```
