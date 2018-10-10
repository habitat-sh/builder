const proxyMiddleware = require('http-proxy-middleware');

module.exports = {
  open: false,
  port: 3000,
  files: [
    './assets/**/*.{html,js,css}'
  ],
  server: {
    middleware: [
      proxyMiddleware('/v1', {
        target: 'http://localhost:9636',
        logLevel: 'debug'
      })
    ]
  },
  startPath: '/#/pkgs/core'
};
