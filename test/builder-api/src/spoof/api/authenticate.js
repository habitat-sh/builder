const Route = require('../route');
const rootRoute = 'authenticate';

const routes = [
  new Route('get', `${rootRoute}/:user`, (req, res) => {
    res.send(`<script> window.data = ${JSON.stringify(req.data)}; </script>`)
  })
]

module.exports = routes;

