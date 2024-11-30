FROM centos:7

# 使用阿里云的 CentOS 镜像源
RUN sed -i 's|^mirrorlist=http://mirrorlist.centos.org|#mirrorlist=http://mirrorlist.centos.org|' /etc/yum.repos.d/CentOS-Base.repo && \
    sed -i 's|#baseurl=http://mirror.centos.org|baseurl=http://mirrors.aliyun.com|' /etc/yum.repos.d/CentOS-Base.repo

RUN yum update -y 
RUN yum install -y epel-release 
RUN yum install -y \ 
      webkitgtk3-devel \
      libappindicator-gtk3-devel \
      librsvg2-devel \
      patchelf \
      gcc \
      gcc-c++ \
      make

ENV TARGET=x86_64-unknown-linux-gnu

COPY ci/scripts/install-rust.sh /scripts/
RUN . /scripts/install-rust.sh
ENV PATH=$PATH:/root/.cargo/bin

RUN curl -sL https://nodejs.org/dist/v16.9.0/node-v16.9.0-linux-x64.tar.xz | tar -xJf - -C /opt
ENV PATH=$PATH:/opt/node-v16.9.0-linux-x64/bin

CMD cargo build --release --target=$TARGET