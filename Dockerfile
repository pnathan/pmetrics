FROM debian:stretch-slim
RUN mkdir -p /opt/pmetrics/bin
ADD target/release/pmetrics /opt/pmetrics/bin/pmetrics
# info messages, not just crisis
CMD /opt/pmetrics/bin/pmetrics -vv server http
