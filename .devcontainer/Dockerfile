# We build qemu in a separate stage to avoid installing a bunch of build dependencies in the final image
FROM fedora:latest AS qemu-builder

RUN --mount=type=cache,target=/var/cache/dnf,id=qemu-dnf \
	dnf update -y && \
	dnf install -y \
	@development-tools \
	git \
	clang \
	python3 \
	python3-pip \
	python3-setuptools \
	python3-wheel \
	ninja-build \
	clang \
	glib2-devel \
	flex \
	bison \
	zlib-devel \
	meson \
	pixman-devel \
	dtc && \
	ln -sf /usr/bin/clang /usr/bin/cc && \
	ln -sf /usr/bin/clang++ /usr/bin/c++

RUN	git clone --depth 1 https://github.com/thomasw04/qemu.git /tmp/qemu

RUN --mount=type=cache,target=/tmp/qemu/build \
	--mount=type=cache,target=/tmp/.cache/pip \
	export PIP_CACHE_DIR=/tmp/.cache/pip && \
	cd /tmp/qemu && \
	python3 -m venv .venv && \
	. .venv/bin/activate && \
	pip install --no-cache-dir tomli sphinx && \
	mkdir -p build && cd build && \
	../configure --disable-werror --extra-cflags="-Wno-error -fdeclspec" --target-list=arm-softmmu,arm-linux-user --enable-kvm && \
	make -j$(nproc) && \
	make install

# This is the final image
FROM fedora:latest AS devcontainer

RUN --mount=type=cache,target=/var/cache/dnf,id=dev-dnf \
	dnf update -y && \
	dnf install -y \
	@development-tools \
	direnv \
	arm-none-eabi-gcc-cs \
	arm-none-eabi-gcc-c++ \
	arm-none-eabi-newlib \
	cmake \
	git \
	make \
	ninja-build \
	clang \
	libusb1-devel \
	openssl-devel \
	lcov \
	awk \
	just \
	python-pyelftools \
	'dnf-command(copr)' && \
	dnf copr enable -y rleh/arm-none-eabi-gdb && \
	dnf install -y arm-none-eabi-gdb && \
	dnf clean all && \
	ln -sf /usr/bin/clang /usr/bin/cc && \
	ln -sf /usr/bin/clang++ /usr/bin/c++

RUN git clone --single-branch --depth 1 -b "develop" https://github.com/stlink-org/stlink.git /tmp/stlink
RUN --mount=type=cache,target=/tmp/stlink/build \
	cd /tmp/stlink && \
	make -j release && \
	make install && \
	ldconfig

# Fix for Fedora not finding libstlink.so.1
ENV LD_LIBRARY_PATH=/usr/local/lib:/usr/local/lib64

# Set Rust environment variables
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV KANI_HOME=/usr/local/kani
ENV PATH="/usr/local/cargo/bin:${PATH}"

ARG RUST_VERSION=1.90.0
RUN --mount=type=cache,target=/usr/local/cargo/registry \
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
	| sh -s -- -y --no-modify-path --default-toolchain ${RUST_VERSION} && \
	rustup component add rust-src llvm-tools-preview && \
	rustup target add thumbv7em-none-eabihf thumbv7em-none-eabi

# Note: kani installs an additional toolchain...
RUN --mount=type=cache,target=/usr/local/cargo/registry \
	--mount=type=cache,target=/usr/local/cargo/target \
	CARGO_TARGET_DIR=/usr/local/cargo/target cargo install --locked kani-verifier && \
	cargo kani setup

RUN --mount=type=cache,target=/usr/local/cargo/registry \
	--mount=type=cache,target=/usr/local/cargo/target \
	CARGO_TARGET_DIR=/usr/local/cargo/target cargo install --locked cargo-binutils
RUN --mount=type=cache,target=/usr/local/cargo/registry \
	--mount=type=cache,target=/usr/local/cargo/target \
	CARGO_TARGET_DIR=/usr/local/cargo/target cargo install --locked cargo-tarpaulin
RUN --mount=type=cache,target=/usr/local/cargo/registry \
	--mount=type=cache,target=/usr/local/cargo/target \
	CARGO_TARGET_DIR=/usr/local/cargo/target cargo install --locked cargo-watch
RUN --mount=type=cache,target=/usr/local/cargo/registry \
	--mount=type=cache,target=/usr/local/cargo/target \
	CARGO_TARGET_DIR=/usr/local/cargo/target cargo install --locked cbindgen

COPY --from=qemu-builder /usr/local/ /usr/local/

ENTRYPOINT ["/bin/bash"]
