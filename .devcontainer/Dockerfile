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
	repo \
	git \
	make \
	ninja-build \
	clang \
	libusb1-devel \
	openssl-devel \
	lcov \
	dtc \
	awk \
	just \
	python-pyelftools \
	'dnf-command(copr)' && \
	dnf copr enable -y rleh/arm-none-eabi-gdb && \
	dnf install -y arm-none-eabi-gdb && \
	dnf clean all && \
	ln -sf /usr/bin/clang /usr/bin/cc && \
	ln -sf /usr/bin/clang++ /usr/bin/c++

RUN git clone --single-branch --depth 2 -b "develop" https://github.com/CreaxxOG/stlink.git /tmp/stlink && \
    cd /tmp/stlink && \
    git fetch origin 6622e2c4b9e28296ef14e7a0740b388eccb47987 && \
    git checkout 6622e2c4b9e28296ef14e7a0740b388eccb47987
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

WORKDIR /tmp

COPY rust-toolchain.toml /tmp/rust-toolchain.toml

RUN --mount=type=cache,target=/usr/local/cargo/registry \
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
	| sh -s -- -y --no-modify-path --default-toolchain none

# rustup show forces install of versions from the toolchain file
RUN rustup show

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

CMD ["/bin/bash"]
