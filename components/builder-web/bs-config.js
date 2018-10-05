const proxyMiddleware = require('http-proxy-middleware');

module.exports = {
  open: false,
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
