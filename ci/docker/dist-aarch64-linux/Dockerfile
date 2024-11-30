FROM centos:7

RUN yum upgrade -y 
RUN yum install -y epel-release 
RUN yum install -y \ 
      webkitgtk3-devel \
      libappindicator-gtk3-devel \
      librsvg2-devel \
      patchelf \
      gcc \
      gcc-c++ \
      make

ENV TARGET=aarch64-unknown-linux-gnu

COPY ci/scripts/install-rust.sh /scripts/
RUN . /scripts/install-rust.sh
ENV PATH=$PATH:/root/.cargo/bin

RUN curl -sL https://nodejs.org/dist/v16.9.0/node-v16.9.0-linux-arm64.tar.xz | tar -xJf - -C /opt
ENV PATH=$PATH:/opt/node-v16.9.0-linux-arm64/bin

RUN rustup target add aarch64-unknown-linux-gnu

CMD cargo build --release --target=$TARGET
