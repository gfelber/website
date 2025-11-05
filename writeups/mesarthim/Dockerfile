```Dockerfile
FROM alpine:3.22@sha256:4bcff63911fcb4448bd4fdacec207030997caf25e9bea4045fa6c8c44de311d1

RUN mkdir -p /opt/mesarthim

WORKDIR /opt/mesarthim
COPY share/run.sh .
COPY share/mesarthim .
COPY share/mesarthim_deploy .

RUN touch flag
RUN adduser -HD ctf
RUN chown ctf:ctf flag
RUN chattr +a flag

USER ctf
EXPOSE 5000
CMD ["./run.sh"]
```
