module.exports = {
  apps: [
    {
      name: 'web3-wallet-frontend',
      script: './node_modules/.bin/vite',
      args: '--host',
      cwd: __dirname,
      watch: false,
      env: {
        NODE_ENV: 'development',
      },
    },
  ],
};
