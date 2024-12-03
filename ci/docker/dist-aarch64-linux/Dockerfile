# 使用阿里云的 CentOS 镜像源
FROM centos:7

# 更新并设置镜像源为阿里云
RUN sed -i 's|^mirrorlist=http://mirrorlist.centos.org|#mirrorlist=http://mirrorlist.centos.org|' /etc/yum.repos.d/CentOS-Base.repo && \
    sed -i 's|#baseurl=http://mirror.centos.org|baseurl=http://mirrors.aliyun.com|' /etc/yum.repos.d/CentOS-Base.repo

# 更新系统并安装必要的软件包
RUN yum groupinstall "Development Tools" && \
    yum install epel-release && \
    yum install webkitgtk4-devel \
      curl \
      wget \
      file \
      openssl-devel \
      gtk3-devel \
      libappindicator-gtk3-devel \
      librsvg2-devel


# 安装 Rust
COPY ci/scripts/install-rust.sh /scripts/
RUN . /scripts/install-rust.sh
ENV PATH=$PATH:/root/.cargo/bin

RUN rustup target add aarch64-unknown-linux-gnu

# 安装 Node.js 18.x
RUN curl -fsSL https://unofficial-builds.nodejs.org/download/release/v18.19.0/node-v18.19.0-linux-x64-glibc-217.tar.gz -o /tmp/node-v18.19.0-linux-x64-glibc-217.tar.gz \
    && tar -xzf /tmp/node-v18.19.0-linux-x64-glibc-217.tar.gz -C /opt/nodejs --strip-components=1 \
    && rm /tmp/node-v18.19.0-linux-x64-glibc-217.tar.gz
ENV PATH=/opt/nodejs/bin:$PATH
RUN npm set strict-ssl false && npm install -g pnpm

# 安装 Tauri CLI
RUN cargo install tauri-cli@1.6.4

# 执行项目构建
CMD cargo dev vendor && \
    cargo dev dist
