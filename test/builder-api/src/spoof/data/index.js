const faker = require('faker');
const util = require('./util');
const make = require('./makers');

class Spoof {
  constructor() {
    this.data = {};
  }

  addUser() {
    this.data.user = make.user();
    return this;
  }

  addAuth() {
    this.data.authentication = make.auth();
    return this;
  }

  addOrigins() {
    const core = make.origin({
      name: 'core',
      default_package_visibility: true,
      package_count: faker.random.number({min: 100, max:999})
    })
    const user = make.origin({
      name: this.data.user.name
    })

    this.data.origins = [core, user];
    return this;
  }

  addProjects() {
    const projects = {};
    this.data.origins.forEach((origin) => {
      projects[origin.name] = {};
      
      for (var i=0; i < origin.package_count; i++) {
        let package_name = `${(Math.random() > .5 ? faker.company.bsBuzz() : faker.company.catchPhraseNoun())}-${faker.company.bsNoun()}`.replace(/ /gi, '').toLowerCase();

        projects[origin.name][package_name] = {
          root: make.project(package_name, origin.name, origin.owner_id, origin.default_package_visibility),
          jobs: util.randomArrayLengthOf(make.job.bind(this, package_name, origin.name, origin.owner_id), 1, 100),
          versions: util.randomArrayLengthOf(make.version.bind(this, package_name, origin.name, origin.default_package_visibility))
        }
      }
    });

    this.data.projects = projects;
    return this;
  }

  init() {
    this.addUser()
      .addAuth()
      .addOrigins()
      .addProjects();

    return this.data;
  }
}

module.exports = Spoof;
