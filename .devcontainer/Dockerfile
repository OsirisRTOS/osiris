# We build qemu in a separate stage to avoid installing a bunch of build dependencies in the final image
FROM fedora:latest AS qemu-builder

RUN dnf update -y && \
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

RUN git clone --depth 1 https://github.com/thomasw04/qemu.git /tmp/qemu && \
	cd /tmp/qemu && \
	python3 -m venv .venv && \
	. .venv/bin/activate && \
	pip install --no-cache-dir tomli sphinx && \
	mkdir build && cd build && \
	../configure --disable-werror --extra-cflags="-Wno-error -fdeclspec" --target-list=arm-softmmu,arm-linux-user --enable-kvm && \
	make -j$(nproc) && \
	make install && \
	cd / && \
	rm -rf /tmp/qemu

# This is the final image
FROM fedora:latest AS devcontainer

RUN dnf update -y && \
	dnf install -y \
	@development-tools \
	direnv \
	arm-none-eabi-gcc-cs \
	arm-none-eabi-gcc-c++ \
	arm-none-eabi-newlib \
	cmake \
	git \
	make \
	clang \
	libusb1-devel \
	'dnf-command(copr)' && \
	dnf copr enable -y rleh/arm-none-eabi-gdb && \
	dnf install -y arm-none-eabi-gdb && \
	dnf clean all && \
	ln -sf /usr/bin/clang /usr/bin/cc && \
	ln -sf /usr/bin/clang++ /usr/bin/c++

RUN git clone --single-branch --depth 1 -b "develop" https://github.com/stlink-org/stlink.git /tmp/stlink && \
	cd /tmp/stlink && \
	make -j release && \
	make install && \
	ldconfig && \
	rm -rf /tmp/stlink

# Fix for Fedora not finding libstlink.so.1
ENV LD_LIBRARY_PATH=/usr/local/lib:/usr/local/lib64

# Set Rust environment variables
ENV RUSTUP_HOME=/usr/local/rustup
ENV CARGO_HOME=/usr/local/cargo
ENV PATH="/usr/local/cargo/bin:${PATH}"

# Create directories with world-writable permissions before switching to non-root user
RUN mkdir -p ${RUSTUP_HOME} ${CARGO_HOME} && \
	chmod 1777 ${RUSTUP_HOME} ${CARGO_HOME}

# The container now runs with a non-root user to avoid file permission issues
ARG USERNAME=vscode
RUN useradd -m -s /bin/bash ${USERNAME} && \
	echo "${USERNAME} ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

WORKDIR /home/${USERNAME}
USER ${USERNAME}

RUN umask 0002 && \
	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
	| sh -s -- -y --no-modify-path && \
	rustup update stable && \
	rustup component add rust-src llvm-tools-preview && \
	cargo install cargo-binutils && \
	rustup target add thumbv7em-none-eabihf thumbv7em-none-eabi

COPY --from=qemu-builder /usr/local/ /usr/local/

ENTRYPOINT ["/bin/bash"]
