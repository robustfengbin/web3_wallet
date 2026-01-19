#!/bin/bash

# Web3 Wallet Service - 启动脚本
# 用于 debug 模式编译和运行

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查是否安装了 cargo
if ! command -v cargo &> /dev/null; then
    log_error "Cargo not found. Please install Rust first."
    exit 1
fi

# 创建日志目录
mkdir -p logs

# 加载环境变量
if [ -f .env ]; then
    log_info "Loading .env file..."
    set -a
    source .env
    set +a
fi

# 设置默认环境变量
export RUST_LOG=${RUST_LOG:-"info,sqlx=warn"}
export RUST_BACKTRACE=${RUST_BACKTRACE:-"1"}

case "$1" in
    build)
        log_info "Building in debug mode..."
        cargo build
        log_info "Build completed!"
        ;;
    run)
        log_info "Building and running in debug mode..."
        cargo run
        ;;
    release)
        log_info "Building in release mode..."
        cargo build --release
        log_info "Release build completed!"
        ;;
    run-release)
        log_info "Building and running in release mode..."
        cargo run --release
        ;;
    pm2)
        log_info "Starting with PM2..."
        if ! command -v pm2 &> /dev/null; then
            log_error "PM2 not found. Install with: npm install -g pm2"
            exit 1
        fi
        pm2 start ecosystem.config.cjs
        pm2 logs web3-wallet-backend
        ;;
    pm2-stop)
        log_info "Stopping PM2 process..."
        pm2 stop web3-wallet-backend
        ;;
    pm2-restart)
        log_info "Restarting PM2 process..."
        pm2 restart web3-wallet-backend
        ;;
    pm2-delete)
        log_info "Deleting PM2 process..."
        pm2 delete web3-wallet-backend
        ;;
    status)
        if command -v pm2 &> /dev/null; then
            pm2 status
        else
            log_warn "PM2 not installed"
        fi
        ;;
    *)
        echo "Usage: $0 {build|run|release|run-release|pm2|pm2-stop|pm2-restart|pm2-delete|status}"
        echo ""
        echo "Commands:"
        echo "  build        - Build in debug mode"
        echo "  run          - Build and run in debug mode"
        echo "  release      - Build in release mode"
        echo "  run-release  - Build and run in release mode"
        echo "  pm2          - Start with PM2 (debug mode)"
        echo "  pm2-stop     - Stop PM2 process"
        echo "  pm2-restart  - Restart PM2 process"
        echo "  pm2-delete   - Delete PM2 process"
        echo "  status       - Show PM2 status"
        exit 1
        ;;
esac
