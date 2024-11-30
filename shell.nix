with import <nixpkgs> { };

mkShell {
  nativeBuildInputs = [
    direnv
    gcc-arm-embedded
    cmake
    git
    python310
    python310Packages.pip
    ninja
    pkg-config
    glib
    flex
    bison
    clang
    zlib
    meson
    pixman
    vde2
    alsa-lib
    texinfo
    lzo
    snappy
    libaio
    libtasn1
    gnutls
    nettle
    curl
    dtc
    libcap
    libcap_ng
    socat
    libslirp
    glibc
    libffi
    ncurses
  ];
  NIX_ENFORCE_PURITY=0;

  shellHook = ''
  if ! git clone --depth 1 https://github.com/thomasw04/qemu.git; then
    echo "Qemu already installed"
  else
    python -m venv .venv
    source .venv/bin/activate
    pip install tomli
    pip install sphinx
    cd qemu
    mkdir build
    cd build
    ../configure --disable-werror --cc=clang --cxx=clang++  --extra-cflags="-Wno-error -fdeclspec" --target-list=arm-softmmu,arm-linux-user --enable-kvm
    make -j16
    cd ..
  fi
  export PATH=$(pwd)/qemu/build:$PATH
  '';
}
