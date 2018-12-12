const debug = require('./debug');
const authenticate = require('./authenticate');

module.exports = [
  ...debug,
  ...authenticate
];