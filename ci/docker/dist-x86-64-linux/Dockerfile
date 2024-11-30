From centos:7

ENV TARGET=x86_64-unknown-linux-gnu

COPY ci/scripts/install-rust.sh /scripts/
RUN . /scripts/install-rust.sh
ENV PATH=$PATH:/root/.cargo/bin

RUN curl -sL https://nodejs.org/dist/v16.9.0/node-v16.9-linux-x64.tar.xz | tar -xJ
ENV PATH=$PATH:/opt/node-v16.9.0-linux-x64/bin

CMD cargo build --release --target=$TARGET