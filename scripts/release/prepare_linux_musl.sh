#!/usr/bin/env bash
set -euo pipefail

: "${TARGET:?TARGET is required}"
: "${GITHUB_ENV:?GITHUB_ENV is required}"

case "$TARGET" in
  x86_64-unknown-linux-musl) arch=x86_64 ;;
  aarch64-unknown-linux-musl) arch=aarch64 ;;
  *) echo "unsupported Whale Linux target: $TARGET" >&2; exit 1 ;;
esac

sudo apt-get update -y
sudo DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
  ca-certificates curl musl-tools pkg-config make xz-utils

linker="$(command -v "${arch}-linux-musl-gcc" || command -v musl-gcc)"
tools_root="${RUNNER_TEMP}/whale-musl-${TARGET}"
prefix="${tools_root}/libcap"
pc_dir="${prefix}/lib/pkgconfig"
mkdir -p "$pc_dir" "${prefix}/include/linux" "${prefix}/include/sys"

version=2.75
archive="${tools_root}/libcap-${version}.tar.xz"
source_dir="${tools_root}/libcap-${version}"
curl -fsSL "https://mirrors.edge.kernel.org/pub/linux/libs/security/linux-privs/libcap2/libcap-${version}.tar.xz" -o "$archive"
echo "de4e7e064c9ba451d5234dd46e897d7c71c96a9ebf9a0c445bc04f4742d83632  ${archive}" | sha256sum -c -
tar -xJf "$archive" -C "$tools_root"
make -C "${source_dir}/libcap" -j"$(nproc)" CC="$linker" AR=ar RANLIB=ranlib
cp "${source_dir}/libcap/libcap.a" "${prefix}/lib/libcap.a"
cp "${source_dir}/libcap/include/uapi/linux/capability.h" "${prefix}/include/linux/capability.h"
cp "${source_dir}/libcap/include/sys/capability.h" "${prefix}/include/sys/capability.h"
cat > "${pc_dir}/libcap.pc" <<EOF
prefix=${prefix}
libdir=\${prefix}/lib
includedir=\${prefix}/include
Name: libcap
Description: Linux capabilities
Version: ${version}
Libs: -L\${libdir} -lcap
Cflags: -I\${includedir}
EOF

linker_var="CARGO_TARGET_${TARGET^^}_LINKER"
linker_var="${linker_var//-/_}"
target_cc_var="CC_${TARGET//-/_}"
target_pc_var="PKG_CONFIG_LIBDIR_${TARGET//-/_}"
{
  echo "${linker_var}=${linker}"
  echo "CC=${linker}"
  echo "TARGET_CC=${linker}"
  echo "${target_cc_var}=${linker}"
  echo "PKG_CONFIG_ALLOW_CROSS=1"
  echo "PKG_CONFIG_PATH=${pc_dir}"
  echo "${target_pc_var}=${pc_dir}"
  echo "AWS_LC_SYS_NO_JITTER_ENTROPY=1"
} >> "$GITHUB_ENV"
