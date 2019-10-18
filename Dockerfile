FROM debian:stretch-slim
RUN mkdir -p /opt/pmetrics/bin
ADD target/release/pmetrics /opt/pmetrics/bin/pmetrics
EXPOSE 1337
CMD /opt/pmetrics/bin/pmetrics server json