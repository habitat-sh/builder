const g: any = global;
let config;

g.habitatConfig = (opts) => config = opts;
require('../habitat.conf.sample.js');
delete g.habitatConfig;

export { config };
