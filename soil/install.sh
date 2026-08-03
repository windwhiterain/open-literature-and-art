#!/bin/sh
set -e

REPO="windwhiterain/open-literature-and-art"
BASE="https://github.com/${REPO}/releases/latest/download"

OS=$(uname -s)
ARCH=$(uname -m)

case "${OS}" in
    Darwin)
        case "${ARCH}" in
            arm64) BINARY="soil-macos-arm64" ;;
            x86_64) BINARY="soil-macos-x86_64" ;;
            *) echo "不支持的系统架构: macOS ${ARCH}"; exit 1 ;;
        esac
        ;;
    Linux)
        case "${ARCH}" in
            x86_64) BINARY="soil-linux-x86_64" ;;
            *) echo "不支持的系统架构: Linux ${ARCH}"; exit 1 ;;
        esac
        ;;
    *) echo "不支持的操作系统: ${OS}"; exit 1 ;;
esac

INSTALL_DIR="${HOME}/.local/bin"
mkdir -p "${INSTALL_DIR}"

echo "正在下载 soil (${BINARY})..."
curl -fsSL --progress-bar -o "${INSTALL_DIR}/soil" "${BASE}/${BINARY}"
chmod +x "${INSTALL_DIR}/soil"

case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
        SHELL_RC=""
        case "${SHELL}" in
            */zsh) SHELL_RC="${HOME}/.zshrc" ;;
            */bash) SHELL_RC="${HOME}/.bashrc" ;;
        esac
        if [ -n "${SHELL_RC}" ]; then
            echo "export PATH=\"${INSTALL_DIR}:\$PATH\"" >> "${SHELL_RC}"
            echo "已将 ${INSTALL_DIR} 追加到 ${SHELL_RC} 的 PATH"
        fi
        export PATH="${INSTALL_DIR}:${PATH}"
        ;;
esac

echo ""
echo "soil 安装完成！"
echo "重新打开终端后运行: soil"
