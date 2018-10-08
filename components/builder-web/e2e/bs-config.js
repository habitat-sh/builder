const proxyMiddleware = require('http-proxy-middleware');

module.exports = {
  open: false,
  port: 5000,
  server: {
    baseDir: './dist'
  }
};
