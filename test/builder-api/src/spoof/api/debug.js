const Route = require('../route');
const rootRoute = 'debug';

const routes = [
  new Route('get', `${rootRoute}`, (req, res) => {
    res.send(`<html><body><script> window.data = ${JSON.stringify(req.data)}; </script></body></html>`)
  })
]

module.exports = routes;

