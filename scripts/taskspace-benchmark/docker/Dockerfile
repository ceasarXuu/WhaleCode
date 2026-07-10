FROM ubuntu@sha256:4fbb8e6a8395de5a7550b33509421a2bafbc0aab6c06ba2cef9ebffbc7092d90

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        bash \
        ca-certificates \
        git \
        jq \
        python3 \
        python3-pytest \
        ripgrep \
    && ln -sf /usr/bin/python3 /usr/local/bin/python \
    && rm -rf /var/lib/apt/lists/*

COPY taskspace-container-entrypoint.sh /usr/local/bin/taskspace-container-entrypoint
RUN chmod 0755 /usr/local/bin/taskspace-container-entrypoint

WORKDIR /workspace
ENTRYPOINT ["/usr/local/bin/taskspace-container-entrypoint"]
