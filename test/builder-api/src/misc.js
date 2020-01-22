const expect = require('chai').expect;
const supertest = require('supertest');
const request = supertest('http://localhost:9636/v1');
const fs = require('fs');

const hookPayload = fs.readFileSync(__dirname + '/../fixtures/ping-hook.json');

describe('Miscellanenous API', function () {
  describe('Retrieving the API status', function () {
    it('succeeds', function (done) {
      request.get('/status')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  // We're going to simulate receiving a GH ping hook
  describe('Receiving a GitHub webhook', function () {
    it('succeeds', function (done) {
      this.skip(); // don't run in master until passing

      request.post('/notify')
        .type('application/json')
        .accept('application/json')
        .set('X-Github-Event', 'ping')
        .set('X-Hub-Signature', 'sha1=6e30dd2c021bdb935f98a827a3d31a2fbdab69d6')
        .send(hookPayload)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  describe('Retrieving reverse dependencies', function () {
    it('returns all reverse dependencies for an origin and package name', function (done) {
      request.get('/rdeps/neurosis/testapp?target=x86_64-linux')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.origin).to.equal('neurosis');
          expect(res.body.name).to.equal('testapp');
          expect(res.body.rdeps).to.deep.equal(["neurosis/oddversion7", "neurosis/testapp3"]);
          done(err);
        });
    });

    it('returns group reverse dependencies for an origin and package name', function (done) {
      this.skip(); // fails in CI - TBD
      request.get('/rdeps/neurosis/testapp/group?target=x86_64-linux')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.origin).to.equal('neurosis');
          expect(res.body.name).to.equal('testapp');
          expect(res.body.rdeps[0].group).to.equal(0)
          expect(res.body.rdeps[0].idents).to.deep.equal(["neurosis/testapp3", "neurosis/oddversion7"]);
          done(err);
        });
    });

    it('sets origin default visibility to private (for test)', function (done) {
      request.put('/depot/origins/neurosis')
        .set('Authorization', global.boboBearer)
        .send({ 'default_package_visibility': 'private' })
        .expect(204)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('hides reverse dependencies for private origin and package name', function (done) {
      request.get('/rdeps/neurosis/testapp?target=x86_64-linux')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.origin).to.equal('neurosis');
          expect(res.body.name).to.equal('testapp');
          expect(res.body.rdeps).to.deep.equal([]);
          done(err);
        });
    });

    it('hides group reverse dependencies for private origin and package name', function (done) {
      this.skip(); // fails in CI - TBD
      request.get('/rdeps/neurosis/testapp/group?target=x86_64-linux')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.origin).to.equal('neurosis');
          expect(res.body.name).to.equal('testapp');
          expect(res.body.rdeps[0].group).to.equal(0)
          expect(res.body.rdeps[0].idents).to.deep.equal([]);
          done(err);
        });
    });


  });
});
