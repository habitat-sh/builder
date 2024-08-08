const expect = require('chai').expect;
const supertest = require('supertest');
const request = supertest('http://localhost:9636/v1');

// These magic values correspond to the testpp repo in the habitat-sh org
const installationId = 56940;
const repoId = 114932712;
const projectCreatePayload = {
  origin: 'neurosis',
  plan_path: 'plan.sh',
  installation_id: installationId,
  repo_id: repoId,
  auto_build: true
};

const dependentProjectCreatePayload = {
  origin: 'neurosis',
  plan_path: 'testapp3/plan.sh',
  installation_id: installationId,
  repo_id: repoId,
  auto_build: true
};

const projectCreatePayloadWindows = {
  origin: 'neurosis',
  plan_path: 'windows/plan.ps1',
  target: 'x86_64-windows',
  installation_id: installationId,
  repo_id: repoId,
  auto_build: true
};

let projectExpectations = function (res) {
  expect(res.body.id).to.not.be.empty;
  expect(res.body.origin).to.equal(global.originNeurosis.name);
  expect(res.body.package_name).to.equal('testapp');
  expect(res.body.name).to.equal('neurosis/testapp');
  expect(res.body.plan_path).to.equal('plan.sh');
  expect(res.body.target).to.equal('x86_64-linux');
  expect(res.body.owner_id).to.equal(global.sessionBobo.id);
  expect(res.body.vcs_type).to.equal('git');
  expect(res.body.vcs_data).to.equal('https://github.com/habitat-sh/testapp.git');
  expect(res.body.vcs_installation_id).to.equal(installationId.toString());
  expect(res.body.auto_build).to.equal(true);
};

let dependentProjectExpectations = function (res) {
  expect(res.body.id).to.not.be.empty;
  expect(res.body.origin).to.equal(global.originNeurosis.name);
  expect(res.body.package_name).to.equal('testapp3');
  expect(res.body.name).to.equal('neurosis/testapp3');
  expect(res.body.plan_path).to.equal('testapp3/plan.sh');
  expect(res.body.target).to.equal('x86_64-linux');
  expect(res.body.owner_id).to.equal(global.sessionBobo.id);
  expect(res.body.vcs_type).to.equal('git');
  expect(res.body.vcs_data).to.equal('https://github.com/habitat-sh/testapp.git');
  expect(res.body.vcs_installation_id).to.equal(installationId.toString());
  expect(res.body.auto_build).to.equal(true);
};

let winProjectExpectations = function (res) {
  expect(res.body.id).to.not.be.empty;
  expect(res.body.origin).to.equal(global.originNeurosis.name);
  expect(res.body.package_name).to.equal('testapp');
  expect(res.body.name).to.equal('neurosis/testapp');
  expect(res.body.plan_path).to.equal('windows/plan.ps1');
  expect(res.body.target).to.equal('x86_64-windows');
  expect(res.body.owner_id).to.equal(global.sessionBobo.id);
  expect(res.body.vcs_type).to.equal('git');
  expect(res.body.vcs_data).to.equal('https://github.com/habitat-sh/testapp.git');
  expect(res.body.vcs_installation_id).to.equal(installationId.toString());
  expect(res.body.auto_build).to.equal(true);
};

describe('Projects API', function () {
  describe('Retrieving a project', function () {
    it('requires authentication', function (done) {
      request.get('/projects/neurosis/testapp')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires membership in the origin that the project refers to', function (done) {
      request.get('/projects/neurosis/testapp')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('succeeds', function (done) {
      request.get('/projects/neurosis/testapp')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .query({
            target: 'x86_64-linux'
        })
        .expect(200)
        .end(function (err, res) {
          projectExpectations(res);
          done(err);
        });
    });

    it('succeeds for non-linux targets', function(done) {
        request.get('/projects/neurosis/testapp')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .query({
            target: 'x86_64-windows'
        })
        .expect(200)
        .end(function (err, res) {
          winProjectExpectations(res);
          done(err);
        });
    });
  });

  describe('Listing all projects in an origin', function () {
    it('requires authentication', function (done) {
      request.get('/projects/neurosis')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires membership in the origin that the project refers to', function (done) {
      request.get('/projects/neurosis')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('succeeds', function (done) {
      request.get('/projects/neurosis')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(3);
          expect(res.body[0]).to.equal('testapp');
          done(err);
        });
    });
  });

  describe('Toggling the privacy of a project', function () {
    it('requires authentication', function (done) {
      request.patch('/projects/neurosis/testapp/private')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires membership in the origin that the project refers to', function (done) {
      request.patch('/projects/neurosis/testapp/private')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires that you set it to a known visibility setting', function (done) {
      request.patch('/projects/neurosis/testapp/lulz')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .expect(400)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('does not allow you to set hidden manually', function (done) {
      request.patch('/projects/neurosis/testapp/hidden')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .expect(400)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('succeeds', function (done) {
      request.patch('/projects/neurosis/testapp/private')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .expect(204)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });
});

describe('Project integrations API', function () {
  describe('Creating a project integration', function () {
    it('requires authentication', function (done) {
      request.put('/projects/neurosis/testapp/integrations/docker/default')
        .type('application/json')
        .accept('application/json')
        .send({
          fun: 'stuff',
          awesome: true,
          numbers: 123
        })
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires membership in the origin that the project refers to', function (done) {
      request.put('/projects/neurosis/testapp/integrations/docker/default')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.mystiqueBearer)
        .send({
          fun: 'stuff',
          awesome: true,
          numbers: 123
        })
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires a JSON body', function (done) {
      request.put('/projects/neurosis/testapp/integrations/docker/default')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .send('this is not JSON')
        .expect(400)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('succeeds', function (done) {
      request.put('/projects/neurosis/testapp/integrations/docker/default')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .send({
          fun: 'stuff',
          awesome: true,
          numbers: 123
        })
        // JB TODO: this is wrong - it should be a 201
        .expect(204)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  describe('Retrieving a project integration', function () {
    it('requires authentication', function (done) {
      request.get('/projects/neurosis/testapp/integrations/docker/default')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires membership in the origin that the project refers to', function (done) {
      request.get('/projects/neurosis/testapp/integrations/docker/default')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('succeeds', function (done) {
      request.get('/projects/neurosis/testapp/integrations/docker/default')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.body).to.deep.equal({
            fun: 'stuff',
            awesome: true,
            numbers: 123
          });
          done(err);
        });
    });
  });

  describe('Deleting a project integration', function () {
    it('requires authentication', function (done) {
      request.delete('/projects/neurosis/testapp/integrations/docker/default')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires membership in the origin that the project refers to', function (done) {
      request.delete('/projects/neurosis/testapp/integrations/docker/default')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('succeeds', function (done) {
      request.delete('/projects/neurosis/testapp/integrations/docker/default')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .expect(204)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });
});
