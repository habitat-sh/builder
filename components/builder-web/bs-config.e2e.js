const proxyMiddleware = require('http-proxy-middleware');

module.exports = {
  open: false,
  port: 3000,
  server: {
    baseDir: './dist'
  }
};
