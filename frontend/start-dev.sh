#!/bin/bash

cd "$(dirname "$0")"

echo "Stopping existing process..."
pm2 delete web3-wallet-frontend 2>/dev/null

echo "Building..."
npm run build

echo "Starting with PM2..."
pm2 start ecosystem.config.cjs

pm2 logs web3-wallet-frontend
