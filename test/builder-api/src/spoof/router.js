class Router {
  constructor(app) {
    this.app = app;
    this.data = { pants: true };
  }

  register(routes) {
    [...routes].forEach((route) => {
      this.app[route.type].call(this.app, route.path, route.callback.bind(this));
    })
  }
}

module.exports = Router;