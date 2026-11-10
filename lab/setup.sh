#!/usr/bin/env bash
# OpenKDX lab setup: one-time, run as root on a Debian/Ubuntu host.
#
# Prepares a machine to run the original 32-bit Haxial KDX binaries in
# bubblewrap sandboxes, capture their traffic as an unprivileged user, and
# host a self-hosted GitHub Actions runner under the `openkdx` user.
#
# Everything here needs root. Nothing the lab does afterwards does.
#
# Usage:  sudo ./setup.sh [LAB_USER]      (default LAB_USER=openkdx)

set -euo pipefail

LAB_USER="${1:-openkdx}"

if [[ $EUID -ne 0 ]]; then
  echo "run as root (sudo $0)" >&2
  exit 1
fi

if ! command -v apt-get >/dev/null; then
  echo "this script targets Debian/Ubuntu (apt-get); adapt for your distro" >&2
  exit 1
fi

if ! id "$LAB_USER" >/dev/null 2>&1; then
  echo "user '$LAB_USER' does not exist; create it first (no sudo needed)" >&2
  exit 1
fi

export DEBIAN_FRONTEND=noninteractive

echo "==> enabling i386 multiarch"
dpkg --add-architecture i386
apt-get update -y

echo "==> 32-bit runtime for the Linux KDX binaries"
apt-get install -y \
  libc6:i386 \
  libstdc++5:i386 \
  libstdc++6:i386 \
  libx11-6:i386 \
  libxext6:i386 \
  libgcc-s1:i386

echo "==> Wine (32-bit) for the Windows client, and X tools to drive the GUIs"
apt-get install -y wine wine32:i386 winetricks xvfb \
  xdotool x11-utils imagemagick expect netcat-openbsd xxd

echo "==> sandbox, capture, and namespace tools"
# preseed so wireshark-common does not prompt; "yes" = non-root users may capture
echo "wireshark-common wireshark-common/install-setuid boolean true" | debconf-set-selections
apt-get install -y \
  bubblewrap \
  wireshark-common \
  tshark \
  iproute2 \
  uidmap \
  unzip \
  p7zip-full \
  curl \
  git \
  jq \
  build-essential \
  pkg-config \
  libssl-dev

echo "==> allowing $LAB_USER to capture packets without root"
usermod -aG wireshark "$LAB_USER"

# The runner user is deliberately NOT added to the docker group: membership
# is equivalent to root, and this machine runs code from pull requests.
if id -nG "$LAB_USER" | tr " " "\n" | grep -qx docker; then
  echo "==> removing $LAB_USER from the docker group (equivalent to root)"
  gpasswd -d "$LAB_USER" docker
fi

echo "==> unprivileged user namespaces (needed by bwrap/unshare without root)"
# Debian ≥11 and Ubuntu enable these by default; make it explicit and persistent.
SYSCTL_FILE=/etc/sysctl.d/90-openkdx-userns.conf
{
  echo "kernel.unprivileged_userns_clone = 1"
  # Ubuntu 24.04+ AppArmor restriction on unprivileged userns; harmless elsewhere.
  echo "kernel.apparmor_restrict_unprivileged_userns = 0"
} > "$SYSCTL_FILE"
sysctl --system >/dev/null 2>&1 || true

echo "==> lab directory layout under $LAB_USER's home"
LAB_HOME=$(getent passwd "$LAB_USER" | cut -d: -f6)
install -d -o "$LAB_USER" -g "$LAB_USER" -m 0750 \
  "$LAB_HOME/haxial"          \
  "$LAB_HOME/haxial/bin"      \
  "$LAB_HOME/haxial/tables"   \
  "$LAB_HOME/haxial/legacy"   \
  "$LAB_HOME/captures"        \
  "$LAB_HOME/actions-runner"
# haxial/bin     original binaries and archives (never committed)
# haxial/tables  extracted challenge tables, kdx.bin and hgc.bin
# haxial/legacy  the user's old server/client data directories, .hap, sound lists
# captures       pcapng + .md scenario notes, copied into the repo by hand

echo "==> Rust toolchain for $LAB_USER (rustup, stable)"
if ! sudo -u "$LAB_USER" -H bash -lc 'command -v cargo' >/dev/null 2>&1; then
  sudo -u "$LAB_USER" -H bash -lc \
    'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain stable'
fi

cat <<EOF

Setup complete. Remaining steps, done as $LAB_USER (no sudo):

  1. Log out and back in so the wireshark/docker group membership applies.
  2. Verify capture works without root:
       dumpcap -D
  3. Verify an unprivileged namespace works:
       unshare -rn ip link show lo
  4. Install the GitHub Actions runner into ~/actions-runner following
     https://github.com/spitfire915/OpenKDX/settings/actions/runners/new
     and give it the label:  lab
     Then, as root once:  sudo ~/actions-runner/svc.sh install $LAB_USER && sudo ~/actions-runner/svc.sh start
  5. Put the Haxial archives into ~/haxial/bin and the old data into ~/haxial/legacy.
     lab/fetch-binaries.sh (Phase 0) will download and verify them for you.

EOF
