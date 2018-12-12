const availableTypes = ['get', 'post', 'put']

module.exports = class Route {
  constructor(type, path, callback) {
    if (!availableTypes.includes(type)) {
      throw new Error(`Bad route type: ${type} - must be one of ${JSON.stringify(availableTypes)}`);
    }
    this.type = type;
    this.path = path.startsWith('/') ? path : '/' + path;
    this.callback = callback;
  }
}