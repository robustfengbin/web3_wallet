module.exports = {
  apps: [
    {
      name: 'web3-wallet-frontend',
      script: 'npm',
      args: 'run dev -- --host',
      cwd: __dirname,
      interpreter: 'none',
      watch: false,
      autorestart: true,
      max_restarts: 10,
      restart_delay: 3000,
      env: {
        NODE_ENV: 'development',
      },
      // 日志文件配置
      out_file: './logs/app.log',
      error_file: './logs/error.log',
      log_date_format: 'YYYY-MM-DD HH:mm:ss',
      merge_logs: true,
    },
  ],
};
