# See here for image contents: https://github.com/microsoft/vscode-dev-containers/tree/v0.145.0/containers/rust/.devcontainer/base.Dockerfile

FROM mcr.microsoft.com/vscode/devcontainers/rust:0-1

# [Optional] Uncomment this section to install additional packages.
# RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
#     && apt-get -y install --no-install-recommends <your-package-list-here>

# Setup neovim
RUN curl -L https://github.com/neovim/neovim/releases/download/nightly/nvim.appimage -o ./nvim.appimage && \
    chmod +x ./nvim.appimage && \
    ./nvim.appimage --appimage-extract && \
    # Remove man symlink
    rm /usr/local/man && \
    cp -drpf ./squashfs-root/usr/* /usr/local/ && \
    rm -rf ./squashfs-root nvim.appimage