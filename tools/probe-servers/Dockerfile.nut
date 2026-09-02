FROM debian:bookworm-slim
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y nut-server && rm -rf /var/lib/apt/lists/*
RUN printf '[dummy]\n driver = dummy-ups\n port = dummy.dev\n desc = "probe"\n' > /etc/nut/ups.conf \
 && printf 'device.mfr: Probe\ndevice.model: Dummy\nups.status: OL\n' > /etc/nut/dummy.dev \
 && printf 'LISTEN 0.0.0.0 3493\n' > /etc/nut/upsd.conf \
 && printf 'MODE=standalone\n' > /etc/nut/nut.conf \
 && printf '[monuser]\n password = probe\n upsmon primary\n' > /etc/nut/upsd.users \
 && chown -R nut:nut /etc/nut && chmod 640 /etc/nut/upsd.users
EXPOSE 3493
CMD ["sh","-c","upsdrvctl start && exec upsd -D -u nut"]
