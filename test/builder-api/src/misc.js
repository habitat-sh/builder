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

  describe('Retrieving reverse dependencies', function () {
    it('returns all reverse dependencies for an origin and package name', function (done) {
      request.get('/rdeps/neurosis/testapp?target=x86_64-linux')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.origin).to.equal('neurosis');
          expect(res.body.name).to.equal('testapp');
          expect(res.body.rdeps).to.deep.equal(["neurosis/oddversion7"]);
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
  });
});
