const Route = require('./route');
const rootRoute = 'depot/origins'

const routes = [
  new Route('get', `user/origins`, (req, res) => res.send(data.userOrigins)),
  new Route('get', `${rootRoute}/:user`, (req, res) => res.send())
]

module.exports = routes;