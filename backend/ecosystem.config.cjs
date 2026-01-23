module.exports = {
  apps: [
    {
      name: 'web3-wallet-backend',
      script: 'cargo',
      args: 'run',
      cwd: __dirname,
      interpreter: 'none',
      watch: false,
      autorestart: true,
      max_restarts: 10,
      restart_delay: 3000,
      env: {
        RUST_LOG: 'debug,sqlx=warn,hyper=info,reqwest=info',
        RUST_BACKTRACE: '1',
      },
      // 合并日志到单个文件
      merge_logs: true,
      // 日志文件配置
      out_file: './logs/app.log',
      error_file: './logs/error.log',
      log_date_format: 'YYYY-MM-DD HH:mm:ss',
      // 进程资源限制
      max_memory_restart: '500M',
    },
  ],
};
