#!/usr/bin/env bash
set -euo pipefail

readonly DFU_UTIL_VERSION="0.11"
readonly DFU_UTIL_SHA256="b4b53ba21a82ef7e3d4c47df2952adf5fa494f499b6b0b57c58c5d04ae8ff19e"
readonly DFU_UTIL_URL="https://dfu-util.sourceforge.net/releases/dfu-util-${DFU_UTIL_VERSION}.tar.gz"
readonly LIBUSB_VERSION="1.0.30"
readonly LIBUSB_SHA256="fea36f34f9156400209595e300840767ab1a385ede1dc7ee893015aea9c6dbaf"
readonly LIBUSB_URL="https://github.com/libusb/libusb/releases/download/v${LIBUSB_VERSION}/libusb-${LIBUSB_VERSION}.tar.bz2"
readonly TARGET_TRIPLE="aarch64-apple-darwin"
readonly ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
readonly WORK_DIR="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/touch-manager-dfu-util"
readonly ARCHIVE="${WORK_DIR}/dfu-util-${DFU_UTIL_VERSION}.tar.gz"
readonly LIBUSB_ARCHIVE="${WORK_DIR}/libusb-${LIBUSB_VERSION}.tar.bz2"
readonly SOURCE_DIR="${WORK_DIR}/dfu-util-${DFU_UTIL_VERSION}"
readonly LIBUSB_SOURCE_DIR="${WORK_DIR}/libusb-${LIBUSB_VERSION}"
readonly LIBUSB_PREFIX="${WORK_DIR}/libusb-static"
readonly OUTPUT_DIR="${ROOT_DIR}/src-tauri/binaries"
readonly OUTPUT="${OUTPUT_DIR}/dfu-util-${TARGET_TRIPLE}"
readonly SOURCE_ASSET_DIR="${ROOT_DIR}/release-assets"

if [[ "$(uname -m)" != "arm64" ]]; then
  echo "The Apple Silicon release helper must run on an arm64 macOS runner." >&2
  exit 1
fi

rm -rf "${WORK_DIR}"
mkdir -p "${WORK_DIR}" "${OUTPUT_DIR}" "${SOURCE_ASSET_DIR}"

curl --fail --location --retry 3 --silent --show-error "${DFU_UTIL_URL}" --output "${ARCHIVE}"
echo "${DFU_UTIL_SHA256}  ${ARCHIVE}" | shasum -a 256 --check
curl --fail --location --retry 3 --silent --show-error "${LIBUSB_URL}" --output "${LIBUSB_ARCHIVE}"
echo "${LIBUSB_SHA256}  ${LIBUSB_ARCHIVE}" | shasum -a 256 --check
tar -xzf "${ARCHIVE}" -C "${WORK_DIR}"
tar -xjf "${LIBUSB_ARCHIVE}" -C "${WORK_DIR}"

cd "${LIBUSB_SOURCE_DIR}"
./configure --disable-shared --enable-static --prefix="${LIBUSB_PREFIX}"
make -j"$(sysctl -n hw.logicalcpu)"
make install

cd "${SOURCE_DIR}"
USB_CFLAGS="-I${LIBUSB_PREFIX}/include/libusb-1.0" \
USB_LIBS="${LIBUSB_PREFIX}/lib/libusb-1.0.a -framework IOKit -framework CoreFoundation -framework Security" \
  ./configure --disable-silent-rules
make -j"$(sysctl -n hw.logicalcpu)"

install -m 0755 src/dfu-util "${OUTPUT}"
strip "${OUTPUT}"
# Tauri copies sidecars into the Cargo target directory. Remove any prior copy so a
# rebuilt helper can never be silently replaced by a stale cached executable.
rm -f "${ROOT_DIR}/src-tauri/target/${TARGET_TRIPLE}/release/dfu-util"
"${OUTPUT}" --version
file "${OUTPUT}" | grep -q "Mach-O 64-bit executable arm64"

if otool -L "${OUTPUT}" | grep -E '/(opt/homebrew|usr/local)/|libusb'; then
  echo "Bundled dfu-util unexpectedly depends on a non-system dynamic library." >&2
  exit 1
fi

install -m 0644 "${ARCHIVE}" "${SOURCE_ASSET_DIR}/dfu-util-${DFU_UTIL_VERSION}.tar.gz"
install -m 0644 "${LIBUSB_ARCHIVE}" "${SOURCE_ASSET_DIR}/libusb-${LIBUSB_VERSION}.tar.bz2"
shasum -a 256 \
  "${OUTPUT}" \
  "${SOURCE_ASSET_DIR}/dfu-util-${DFU_UTIL_VERSION}.tar.gz" \
  "${SOURCE_ASSET_DIR}/libusb-${LIBUSB_VERSION}.tar.bz2"
