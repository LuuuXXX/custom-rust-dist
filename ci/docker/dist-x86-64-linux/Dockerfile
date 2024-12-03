# 使用阿里云的 CentOS 镜像源
FROM centos:7

# 更新并设置镜像源为阿里云
RUN sed -i 's|^mirrorlist=http://mirrorlist.centos.org|#mirrorlist=http://mirrorlist.centos.org|' /etc/yum.repos.d/CentOS-Base.repo && \
    sed -i 's|#baseurl=http://mirror.centos.org|baseurl=http://mirrors.aliyun.com|' /etc/yum.repos.d/CentOS-Base.repo

# 更新系统并安装必要的软件包
RUN yum update -y && \
    yum install -y epel-release && \
    yum groupinstall -y "Development Tools" && \
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
        perl-core \
        wget \
        python3

# 安装 Rust
COPY ci/scripts/install-rust.sh /scripts/
RUN . /scripts/install-rust.sh
ENV PATH=$PATH:/root/.cargo/bin

# 安装 Node.js 18.x
RUN curl -o /usr/local/src/node-v18.18.0.tar.gz https://nodejs.org/dist/v18.18.0/node-v18.18.0.tar.gz
RUN tar -xzvf /usr/local/src/node-v18.18.0.tar.gz -C /usr/local/src

# 安装 devtoolset-8，支持更高版本的 g++
RUN yum install -y centos-release-scl
RUN yum install -y devtoolset-8-gcc devtoolset-8-gcc-c++
RUN scl enable devtoolset-8 bash

# 编译 Node.js
RUN cd /usr/local/src/node-v18.18.0 && ./configure && make && make install

# 设置 Node.js 路径
ENV PATH=/usr/local/bin:$PATH

# 安装 pnpm
RUN curl -fsSL https://get.pnpm.io/install.sh | sh -
ENV PNPM_HOME="/root/.local/share/pnpm"
ENV PATH="$PNPM_HOME:$PATH"
RUN pnpm add -g pnpm

# 安装 Tauri CLI
RUN cargo install tauri-cli@1.6.4

# 执行项目构建
CMD cargo dev vendor && \
    cd installer && \
    pnpm update && \
    pnpm i && \
    cd - && \
    cargo dev dist --target=$TARGET
