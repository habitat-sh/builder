const express = require('express');
const app = express();
const port = 9636;
const Router = require('./src/spoof/router');
const api = require('./src/spoof/api/index');
const Spoofer = require('./src/spoof/data/index');

const spoofer = new Spoofer();
spoofer.init();

app.use((req, res, next) => {
  req.data = spoofer.data;
  next();
})

const router = new Router(app);
router.register(api);

app.listen(port, () => {
  console.log(`Beginning Habitat dev api spoofer on port ${port}`)
})