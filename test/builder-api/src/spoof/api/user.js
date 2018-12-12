const Route = require('./route');
const rootRoute = 'user';

const routes = [
  new Route('get', `${rootRoute}/origins`, (req, res) => res.send(data.origins)),
  new Route('get', `${rootRoute}/invitations`, (req, res) => res.send(data.invitations))
]

module.exports = routes;