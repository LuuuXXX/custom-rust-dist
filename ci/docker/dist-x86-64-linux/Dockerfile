FROM centos:7

# 使用阿里云的 CentOS 镜像源
RUN sed -i 's|^mirrorlist=http://mirrorlist.centos.org|#mirrorlist=http://mirrorlist.centos.org|' /etc/yum.repos.d/CentOS-Base.repo && \
    sed -i 's|#baseurl=http://mirror.centos.org|baseurl=http://mirrors.aliyun.com|' /etc/yum.repos.d/CentOS-Base.repo

RUN yum update -y && \
    yum install -y epel-release && \
    yum install -y \ 
      webkitgtk3-devel \
      libappindicator-gtk3-devel \
      librsvg2-devel \
      patchelf \
      gcc \
      gcc-c++ \
      glibc-devel \
      libstdc++-devel \
      make \
      openssl \
      openssl-devel \
      pkgconfig \
      perl \
      perl-core

# ENV OPENSSL_DIR=/usr/include/openssl
ENV TARGET=x86_64-unknown-linux-gnu

COPY ci/scripts/install-rust.sh /scripts/
RUN . /scripts/install-rust.sh
ENV PATH=$PATH:/root/.cargo/bin

# RUN curl -fsSL https://rpm.nodesource.com/setup_16.x | bash - \
#     && yum install -y nodejs


RUN cd /usr/local/src && \
    wget https://nodejs.org/dist/v18.x/node-v18.18.0.tar.gz && \
    tar -xzvf node-v18.18.0.tar.gz && \
    cd node-v18.18.0 && \
    ./configure && \
    make && \
    make install

ENV PATH=/usr/local/bin:$PATH

RUN curl -fsSL https://get.pnpm.io/install.sh | sh -
ENV PNPM_HOME="/root/.local/share/pnpm"
ENV PATH="$PNPM_HOME:$PATH"

RUN pnpm add -g pnpm

RUN cargo install tauri-cli@1.6.4

CMD cargo dev vendor && cd installer && pnpm update && pnpm i && cd - && cargo dev dist --target=$TARGET
