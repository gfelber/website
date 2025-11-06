```Dockerfile
FROM debian:trixie-slim@sha256:1caf1c703c8f7e15dcf2e7769b35000c764e6f50e4d7401c355fb0248f3ddfdb

ADD --chmod=0755 --checksum=sha256:c125df9762b0c7233459087bb840c0e5dbfc4d9690ee227f1ed8994f4d51d2e0 \
  https://raw.githubusercontent.com/reproducible-containers/repro-sources-list.sh/v0.1.4/repro-sources-list.sh \
  /usr/local/bin/repro-sources-list.sh
RUN repro-sources-list.sh

# Install lighttpd with CGI support
RUN apt-get update && \
    apt install -y lighttpd spawn-fcgi libfcgi && \
    rm -rf /var/lib/apt/lists/* /var/cache/apt/*

RUN mkdir -p /var/www/bin

COPY ./share/hamal /var/www/bin/
RUN chmod 755 /var/www/bin/hamal
COPY ./share/lighttpd.conf /etc/lighttpd/lighttpd.conf
COPY ./share/static /var/www/html

RUN chown www-data:www-data -R /var/www/bin/ /var/www/html/ /var/run/

EXPOSE 8080

CMD printenv FLAG > /dev/flag; \
    lighttpd -D -f /etc/lighttpd/lighttpd.conf
```
