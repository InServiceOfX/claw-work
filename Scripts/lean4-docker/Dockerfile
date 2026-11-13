# Lean 4 build/proof-check image for "Lean reference model -> proof -> emitted C" projects.
#
# Why not leanprovercommunity/lean4 from Docker Hub? Its tags were last updated
# 2024-08-20 (checked via hub.docker.com/v2 API on 2026-11-01), far behind current
# toolchains. This image installs elan and pins the toolchain at build time instead.
#
# Untested with docker on the bot box (no docker there); the same steps were run
# directly with elan on Debian trixie and built hargup's spec (Lean v4.35.0).

FROM debian:bookworm-slim

ARG LEAN_TOOLCHAIN=leanprover/lean4:v4.35.0
ARG UID=1000
ARG GID=1000

RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates curl git build-essential xz-utils python3 \
 && rm -rf /var/lib/apt/lists/*

RUN groupadd -g ${GID} lean && useradd -m -u ${UID} -g ${GID} -s /bin/bash lean
USER lean
ENV ELAN_HOME=/home/lean/.elan
ENV PATH=/home/lean/.elan/bin:${PATH}

# elan installs per-project toolchains from each repo's lean-toolchain file; we
# pre-fetch the pinned one so `lake build` works offline / without a cold start.
RUN curl -sSfL https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh -o /tmp/elan-init.sh \
 && sh /tmp/elan-init.sh -y --default-toolchain none \
 && rm /tmp/elan-init.sh \
 && elan toolchain install ${LEAN_TOOLCHAIN} \
 && elan default ${LEAN_TOOLCHAIN}

COPY --chown=lean:lean check.sh /usr/local/bin/lean-check
WORKDIR /work
CMD ["lean-check"]
