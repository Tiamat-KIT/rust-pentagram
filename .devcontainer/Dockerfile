# ベースイメージを指定
FROM mcr.microsoft.com/vscode/devcontainers/rust:1-1

# 必要なツールをインストール
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    libssl-dev \
    pkg-config \
    && apt-get clean -y && rm -rf /var/lib/apt/lists/*

# Wasmtime CLI をインストール
RUN cargo install wasmtime-cli
WORKDIR /workspace