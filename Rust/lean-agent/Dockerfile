# syntax=docker/dockerfile:1
ARG BASE_IMAGE=ubuntu@sha256:d1e2e92c075e5ca139d51a140fff46f84315c0fdce203eab2807c7e495eff4f9
FROM ${BASE_IMAGE} AS builder
ARG TARGETARCH
RUN test "$TARGETARCH" = amd64
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl git zstd build-essential libgmp-dev libuv1-dev libssl-dev pkg-config && rm -rf /var/lib/apt/lists/*
RUN pkg-config --atleast-version=6.3.0 gmp
WORKDIR /opt
RUN curl --fail --location --retry 3 --proto '=https' --tlsv1.2 https://github.com/leanprover/lean4/releases/download/v4.35.0/lean-4.35.0-linux.tar.zst -o lean.tar.zst \
 && echo '7289597a44543246c9227b8cae501767e095ce17f6d448b95fc4a8dc97268f6d  lean.tar.zst' | sha256sum -c - \
 && mkdir lean && tar --zstd -xf lean.tar.zst --strip-components=1 -C lean && rm lean.tar.zst
ENV PATH="/opt/lean/bin:${PATH}"
RUN git init mathlib && cd mathlib \
 && git remote add origin https://github.com/leanprover-community/mathlib4.git \
 && git fetch --depth 1 origin 56a005855ea17e0b0958f73ec801ae140b0a1679 \
 && git checkout --detach FETCH_HEAD \
 && test "$(cat lean-toolchain)" = 'leanprover/lean4:v4.35.0' \
 && echo '6fcb8417bf0d6056cebf3c9f9bacd4569f21f3b9c571c60649150b9118b55147  lake-manifest.json' | sha256sum -c -
WORKDIR /opt/mathlib
RUN lake exe cache get
RUN lake env printenv LEAN_PATH > /opt/mathlib-path \
 && printf 'import Mathlib\nexample (x y : Nat) : x + y = y + x := by omega\n' > /tmp/Smoke.lean \
 && lake env lean /tmp/Smoke.lean \
 && find /opt/mathlib -type d -name .git -prune -exec rm -rf '{}' + \
 && rm -rf /root/.cache /opt/mathlib/.lake/build/bin /opt/mathlib/.lake/build/ir \
 && find /opt/mathlib/.lake/packages -type d -path '*/.lake/build/ir' -prune -exec rm -rf '{}' + \
 && chmod -R a+rX /opt/lean /opt/mathlib
COPY docker/primitives.txt /opt/lean-agent-primitives.txt
RUN leanexport Init -- $(cat /opt/lean-agent-primitives.txt) > /opt/primitive-baseline.ndjson
# The release statically bundles GMP. A system dpkg version check alone does not
# establish the archive's version; interrogate the checksum-pinned bundled archive.
RUN printf '%s\n' '#include <stdio.h>' 'extern const char * const __gmp_version;' 'int main(void) { puts(__gmp_version); return 0; }' > /tmp/gmp-probe.c \
 && cc /tmp/gmp-probe.c /opt/lean/lib/libgmp.a -o /tmp/gmp-probe \
 && test "$(/tmp/gmp-probe)" = 6.3.0 \
 && rm /tmp/gmp-probe /tmp/gmp-probe.c
# Deliberately keep .olean/.olean.private/.olean.server/.ir and shared libraries:
# those are part of the tested checking, interpretation and LSP closure.
FROM ${BASE_IMAGE} AS runtime
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends ca-certificates libgmp10 libuv1t64 libssl3t64 git bubblewrap time tzdata \
 && dpkg --compare-versions "$(dpkg-query -W -f='${Version}' libgmp10)" ge 2:6.3.0 \
 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /opt/lean /opt/lean
COPY --from=builder /opt/mathlib /opt/mathlib
COPY --from=builder /opt/mathlib-path /opt/mathlib-path
COPY --from=builder /opt/primitive-baseline.ndjson /opt/primitive-baseline.ndjson
COPY docker/primitives.txt /opt/lean-agent-primitives.txt
RUN printf '%s\n' '{"challenge_module":"Init","solution_module":"Challenge","theorem_names":[],"definition_names":["Nat.add"],"permitted_axioms":["propext","Classical.choice","Quot.sound"]}' > /opt/primitive-guard.json
COPY docker/pins.json /opt/lean-agent-pins.json
COPY --chmod=755 docker/entrypoint.sh /usr/local/bin/lean-container
RUN mkdir /work && chmod 1777 /work
ENV PATH="/opt/lean/bin:${PATH}" HOME=/tmp
# docker exec does not inherit variables exported dynamically by ENTRYPOINT.
ENV LEAN_PATH="/opt/mathlib/.lake/packages/Cli/.lake/build/lib/lean:/opt/mathlib/.lake/packages/batteries/.lake/build/lib/lean:/opt/mathlib/.lake/packages/Qq/.lake/build/lib/lean:/opt/mathlib/.lake/packages/aesop/.lake/build/lib/lean:/opt/mathlib/.lake/packages/proofwidgets/.lake/build/lib/lean:/opt/mathlib/.lake/packages/importGraph/.lake/build/lib/lean:/opt/mathlib/.lake/packages/LeanSearchClient/.lake/build/lib/lean:/opt/mathlib/.lake/packages/plausible/.lake/build/lib/lean:/opt/mathlib/.lake/build/lib/lean:/opt/lean/lib/lean"
USER 65532:65532
WORKDIR /work
ENTRYPOINT ["lean-container"]
CMD ["version"]
