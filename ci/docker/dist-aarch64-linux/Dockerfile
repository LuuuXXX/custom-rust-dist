# 使用阿里云的 CentOS 镜像源（ARM 版本）
FROM arm64v8/centos:7

# 更新并设置镜像源为阿里云（针对 ARM 架构）
RUN sed -i 's|^mirrorlist=http://mirrorlist.centos.org|#mirrorlist=http://mirrorlist.centos.org|' /etc/yum.repos.d/CentOS-Base.repo && \
    sed -i 's|#baseurl=http://mirror.centos.org|baseurl=http://mirrors.aliyun.com/centos/7/os/aarch64|' /etc/yum.repos.d/CentOS-Base.repo && \
    sed -i 's|^#baseurl=http://mirror.centos.org|baseurl=http://mirrors.aliyun.com/centos/7/updates/aarch64|' /etc/yum.repos.d/CentOS-Base.repo && \
    sed -i 's|^#baseurl=http://mirror.centos.org|baseurl=http://mirrors.aliyun.com/centos/7/extra/aarch64|' /etc/yum.repos.d/CentOS-Base.repo


# 更新系统并安装必要的软件包
RUN yum groupinstall -y "Development Tools" && \
    yum install -y epel-release && \
    yum install -y webkitgtk4-devel \
      curl \
      wget \
      file \
      openssl-devel \
      gtk3-devel \
      libappindicator-gtk3-devel \
      librsvg2-devel \
      perl \
      perl-core

# 安装 Rust（确保是适用于 ARM 的目标）
COPY ci/scripts/install-rust.sh /scripts/
RUN . /scripts/install-rust.sh
ENV PATH=$PATH:/root/.cargo/bin

ENV HOST_TRIPPLE aarch64-unknown-linux-gnu

# 安装 Node.js 18.x
COPY ci/scripts/install-nodejs.sh /scripts/
RUN . /scripts/install-nodejs.sh
ENV PATH=/opt/nodejs/bin:$PATH
RUN npm set strict-ssl false && npm install -g pnpm

# 安装 Tauri CLI
RUN cargo install tauri-cli@1.6.4

# 执行项目构建
CMD cargo dev vendor && \
    cargo dev dist
