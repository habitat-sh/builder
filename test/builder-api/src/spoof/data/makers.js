const faker = require('faker');
const util = require('./util');
const makers = {};

makers.user = () => {
  const name = faker.internet.userName();
  const user = {
    name,
    created_at: faker.date.past(3, faker.date.past(1)),    
    email: `${name}@${faker.internet.domainName()}`,
    id: util.numberStringByLength(18)
  };
  user.updated_at = faker.date.past(3, user.created_at);

  return user;
}

makers.auth = () => {
  return {
    token: `${faker.random.alphaNumeric(84)}=`,
    flags: 0,
    oauth_token: faker.random.alphaNumeric(40)
  };
}

makers.origin = (template) => {
  const origin = {
    ...{
      name: faker.internet.userName,
      created_at: faker.date.past(3),
      owner_id: util.numberStringByLength(18),
      default_package_visibility: false,
      package_count: faker.random.number({min: 2, max:10})
    },
    ...template};
    origin.updated_at = faker.date.past(3, origin.created_at);

    return origin;
}

makers.project = (package_name, origin, owner_id, visibility) => {
  return {
    package_name,
    origin,
    owner_id,          
    visibility,
    id: util.numberStringByLength(19),
    name: `${origin}/${package_name}`,
    plan_path: `${package_name}/plan.sh`,      
    vcs_type: 'git',
    vcs_data: `https://github.com/${origin}/${package_name}`,
    vcs_installation_id: util.numberStringByLength(5),
    auto_build: faker.random.boolean(),
    created_at: faker.date.past(),
    updated_at: faker.date.past()
  }
}

makers.job = (name, origin, owner_id) => {
  let job = {
    name,
    origin,
    owner_id,
    build_finished_at: faker.date.past(),
    id: util.numberStringByLength(19),
    release: util.numberStringByLength(19),
    state: faker.helpers.randomize(['Complete', 'Failed', 'Pending', 'Running']),
    version: faker.system.semver()
  }; 
  job.build_started_at = faker.date.past(1, job.build_finished_at);
  job.created_at = faker.date.past(1, job.build_started_at);
  
  return job;
}

makers.version = (name, origin, visibility) => {
  return {
    name, 
    origin,
    version: faker.system.semver(),
    release_count: faker.random.number({min:1, max:20}),
    latest: util.numberStringByLength(14),
    platforms: faker.helpers.randomize([['x86_64-linux'], ['x86_64-windows'], ['x86_64-linux', 'x86_64-windows']]),
    visibility
  }
}



module.exports = makers;