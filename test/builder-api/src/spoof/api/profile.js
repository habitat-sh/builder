const Route = require('./route');
const rootRoute = 'profile';

const routes = [
  new Route('get', `${rootRoute}`, (req, res) => res.send(data))
]

module.exports = routes;