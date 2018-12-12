const Route = require('./route');
const rootRoute = 'projects';

const routes = [
  new Route('get', `${rootRoute}/:user`, (req, res) => res.send(data[req.params.user])),
  new Route('get', `${rootRoute}/:user/:pkg`, (req, res) => res.send(data[req.params.user])),
]

module.exports = routes;