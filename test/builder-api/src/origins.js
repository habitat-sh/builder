const expect = require('chai').expect;
const supertest = require('supertest');
const request = supertest('http://localhost:9636/v1');

// These magic values correspond to the testpp repo in the habitat-sh org
const installationId = 56940;
const repoId = 114932712;
const projectCreatePayload = {
  origin: 'umbrella',
  plan_path: 'plan.sh',
  installation_id: installationId,
  repo_id: repoId,
  auto_build: true
};

describe('Origin API', function () {
  describe('Create neurosis origin', function () {
    it('requires authentication', function (done) {
      request.post('/depot/origins')
        .send({ 'name': 'neurosis' })
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('returns the created origin', function (done) {
      request.post('/depot/origins')
        .set('Authorization', global.boboBearer)
        .send({ 'name': 'neurosis', 'default_package_visibility': 'private' })
        .expect(201)
        .end(function (err, res) {
          expect(res.body.name).to.equal('neurosis');
          expect(res.body.default_package_visibility).to.equal('private');
          global.originNeurosis = res.body;
          done(err);
        });
    });
  });

  describe('Get origin neurosis', function () {
    it('returns the origin', function (done) {
      request.get('/depot/origins/neurosis')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.name).to.equal(global.originNeurosis.name);
          expect(res.body.id).to.equal(global.originNeurosis.id);
          expect(res.body.owner_id).to.equal(global.originNeurosis.owner_id);
          expect(res.body.default_package_visibility).to.equal(global.originNeurosis.default_package_visibility);
          done(err);
        });
    });
  });

  describe('Create Umbrella Corp. origin', function () {
    it('returns the created origin', function (done) {
      request.post('/depot/origins')
        .set('Authorization', global.weskerBearer)
        .send({ 'name': 'umbrella' })
        .expect(201)
        .end(function (err, res) {
          expect(res.body.name).to.equal('umbrella');
          global.originUmbrella = res.body;
          done(err);
        });
    });

    it('succeeds in creating a project', function (done) {
      this.timeout(5000);
      request.post('/projects')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.weskerBearer)
        .send(projectCreatePayload)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });
  });

  describe('Create xmen origin', function () {
    it('returns the created origin', function (done) {
      request.post('/depot/origins')
        .set('Authorization', global.mystiqueBearer)
        .send({ 'name': 'xmen' })
        .expect(201)
        .end(function (err, res) {
          expect(res.body.name).to.equal('xmen');
          global.originXmen = res.body;
          done(err);
        });
    });
  });

  describe('Updating origins', function () {
    it('requires authentication', function (done) {
      request.put('/depot/origins/neurosis')
        .send({ 'default_package_visibility': 'public' })
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires that you are a member of the origin being updated', function (done) {
      request.put('/depot/origins/neurosis')
        .set('Authorization', global.mystiqueBearer)
        .send({ 'default_package_visibility': 'public' })
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('succeeds', function (done) {
      request.put('/depot/origins/neurosis')
        .set('Authorization', global.boboBearer)
        .send({ 'default_package_visibility': 'public' })
        .expect(204)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('reflects the changes when viewing it again', function (done) {
      request.get('/depot/origins/neurosis')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.default_package_visibility).to.equal('public');
          global.originNeurosis = res.body;
          done(err);
        });
    });
  });

  describe('Origin secret creation', function () {
    it('requires authentication', function (done) {
      request.post('/depot/origins/neurosis/secret')
        .send({ 'name': 'foo', 'value': 'bar' })
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    // TODO - add a successful creation test
  });

  describe('Origin secret list', function () {
    it('requires authentication', function (done) {
      request.get('/depot/origins/neurosis/secret')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('succeeds', function (done) {
      request.get('/depot/origins/neurosis/secret')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(0);
          done(err);
        });
    });
  });

  describe('Origin secret deletion', function () {
    it('requires authentication', function (done) {
      request.delete('/depot/origins/neurosis/secret/foo')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    // TODO - add a successful deletion test
  });

  describe('Origin deletion', function () {
    it('requires authentication', function (done) {
      request.delete('/depot/origins/umbrella')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires membership in the origin', function (done) {
      request.delete('/depot/origins/umbrella')
        .set('Authorization', global.boboBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('fails with a conflict when not deletable', function (done) {
      request.delete('/depot/origins/umbrella')
        .set('Authorization', global.weskerBearer)
        .expect(409)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('succeeds in deleting associated project', function (done) {
      request.delete('/projects/umbrella/testapp')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.weskerBearer)
        .expect(204)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('succeeds', function (done) {
      request.delete('/depot/origins/umbrella')
        .set('Authorization', global.weskerBearer)
        .expect(204)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });
});
