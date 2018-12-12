const Route = require('./route');
const rootRoute = 'depot';

const routes = [
  new Route('get', `${rootRoute}/origins/:user`, (req, res) => res.send(data.origins[req.params.user].root)),
  new Route('get', `${rootRoute}/origins/:user/integrations`, (req, res) => res.send(data.origins[req.params.user].integrations)),
  new Route('get', `${rootRoute}/origins/:user/secret`, (req, res) => res.send(data.origins[req.params.user].secret)),
  new Route('get', `${rootRoute}/:user/pkgs`, (req, res) => res.send(selectPkgs(req.params.user, req.query.range))),
  new Route('get', `${rootRoute}/invitations`, (req, res) => res.send(data.invitations))
]

module.exports = routes;