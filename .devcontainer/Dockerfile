FROM fedora:latest

# Install Development Tools and required packages
RUN dnf update -y && \
	dnf install -y @development-tools && \
	dnf install -y \
	direnv \
	arm-none-eabi-gcc-cs \
	arm-none-eabi-gcc-c++ \
	arm-none-eabi-newlib \
	cmake \
	git \
	python3 \
	python3-virtualenv \
	python3-pip \
	ninja-build \
	pkgconf \
	glib2-devel \
	flex \
	bison \
	clang \
	zlib-devel \
	meson \
	pixman-devel \
	alsa-lib-devel \
	texinfo \
	lzo-devel \
	snappy-devel \
	libaio-devel \
	libtasn1-devel \
	gnutls-devel \
	nettle-devel \
	curl \
	dtc \
	libcap-devel \
	libcap-ng-devel \
	socat \
	libslirp-devel \
	libffi-devel \
	ncurses-devel && \
	dnf clean all

ARG USERNAME=vscode
RUN useradd -m -s /bin/bash ${USERNAME} && \
	echo "${USERNAME} ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Set working directory and switch to non-root user
WORKDIR /home/${USERNAME}
USER ${USERNAME}

# Install Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y && \
	. $HOME/.cargo/env && \
	rustup component add rust-src && \
	rustup component add llvm-tools-preview && \
	cargo install cargo-binutils && \
	rustup target add thumbv7em-none-eabihf && \
	rustup target add thumbv7em-none-eabi

# Clone, build, and install QEMU
RUN git clone --depth 1 https://github.com/thomasw04/qemu.git /tmp/qemu && \
	cd /tmp/qemu && \
	python3 -m venv .venv && \
	. .venv/bin/activate && \
	pip install --no-cache-dir tomli sphinx && \
	mkdir build && cd build && \
	../configure --disable-werror --cc=clang --cxx=clang++ --extra-cflags="-Wno-error -fdeclspec" --target-list=arm-softmmu,arm-linux-user --enable-kvm && \
	make -j$(nproc) && \
	sudo make install && \
	cd / && \
	rm -rf /tmp/qemu

CMD ["bash"]
