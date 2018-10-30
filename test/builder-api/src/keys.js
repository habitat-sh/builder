const expect = require('chai').expect;
const supertest = require('supertest');
const request = supertest('http://localhost:9636/v1');
const fs = require('fs');

const revision = '20171211220037';
const pubFile = fs.readFileSync(__dirname + `/../fixtures/neurosis-${revision}.pub`, 'utf8');
const secretFile = fs.readFileSync(__dirname + `/../fixtures/neurosis-${revision}.sig.key`, 'utf8');

describe('Keys API', function () {
  describe('Uploading public keys', function () {
    it('requires authentication', function (done) {
      request.post(`/depot/origins/neurosis/keys/${revision}`)
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires membership in the origin you are uploading to', function (done) {
      request.post(`/depot/origins/neurosis/keys/${revision}`)
        .set('Authorization', global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('uploads the key', function (done) {
      request.post(`/depot/origins/neurosis/keys/${revision}`)
        .set('Authorization', global.boboBearer)
        .send(pubFile)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/origins/neurosis/keys/${revision}`);
          // JB TODO: this is wrong - this URL doesn't go anywhere in our
          // system
          expect(res.header['location']).to.equal(`/v1/depot/origins/neurosis/keys/${revision}`);
          done(err);
        });
    });

    it('expects a Conflict result on second upload of same key', function (done) {
      request.post(`/depot/origins/neurosis/keys/${revision}`)
        .set('Authorization', global.boboBearer)
        .send(pubFile)
        .expect(409)
        .end(function (err, res) {
          expect(res.body).to.be.empty;
          done(err);
        });
    });
  });

  describe('Downloading public keys', function () {
    it('can download a specific revision', function (done) {
      request.get(`/depot/origins/neurosis/keys/${revision}`)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.equal(pubFile);
          done(err);
        });
    });

    it('can download the latest key', function (done) {
      request.get('/depot/origins/neurosis/keys/latest')
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.equal(pubFile);
          done(err);
        });
    });
  });

  describe('Uploading secret keys', function () {
    it('requires authentication', function (done) {
      request.post(`/depot/origins/neurosis/secret_keys/${revision}`)
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires membership in the origin you are uploading to', function (done) {
      request.post(`/depot/origins/neurosis/secret_keys/${revision}`)
        .set('Authorization', global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('uploads the key', function (done) {
      request.post(`/depot/origins/neurosis/secret_keys/${revision}`)
        .set('Authorization', global.boboBearer)
        .send(secretFile)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('expects a Conflict result on second upload of same key', function (done) {
      request.post(`/depot/origins/neurosis/secret_keys/${revision}`)
        .set('Authorization', global.boboBearer)
        .send(secretFile)
        .expect(409)
        .end(function (err, res) {
          expect(res.body).to.be.empty;
          done(err);
        });
    });

    it('retrieves the secret key with origin get request', function (done) {
      request.get('/depot/origins/neurosis')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.name).to.equal(global.originNeurosis.name);
          expect(res.body.id).to.equal(global.originNeurosis.id);
          expect(res.body.owner_id).to.equal(global.originNeurosis.owner_id);
          expect(res.body.default_package_visibility).to.equal(global.originNeurosis.default_package_visibility);
          expect(res.body.private_key_name).to.equal(`neurosis-${revision}`);
          done(err);
        });
    });

  });

  describe('Downloading secret keys', function () {
    it('requires authentication', function (done) {
      request.get('/depot/origins/neurosis/secret_keys/latest')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires membership in the origin you are uploading to', function (done) {
      request.get('/depot/origins/neurosis/secret_keys/latest')
        .set('Authorization', global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('can download the latest key', function (done) {
      request.get('/depot/origins/neurosis/secret_keys/latest')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.equal(secretFile);
          done(err);
        });
    });
  });

  describe('Downloading encryption keys', function () {
    it('requires authentication', function (done) {
      request.get('/depot/origins/neurosis/encryption_key')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires membership in the origin', function (done) {
      request.get('/depot/origins/neurosis/encryption_key')
        .set('Authorization', global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('can download the latest encryption public key', function (done) {
      request.get('/depot/origins/neurosis/encryption_key')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.not.be.empty;
          done(err);
        });
    });
  });

  describe('Generating keys', function () {
    it('requires authentication', function (done) {
      request.post('/depot/origins/neurosis/keys')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires membership in the origin you are uploading to', function (done) {
      request.post('/depot/origins/neurosis/keys')
        .set('Authorization', global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('generates the key', function (done) {
      request.post('/depot/origins/neurosis/keys')
        .set('Authorization', global.boboBearer)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  describe('Listing keys', function () {
    it('can list all public keys', function (done) {
      request.get('/depot/origins/neurosis/keys')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(2);
          expect(res.body[0].origin).to.equal('neurosis');
          expect(parseInt(res.body[0].revision) > parseInt(res.body[1].revision)).to.be.true;
          expect(res.body[0].location).to.equal(`/origins/neurosis/keys/${res.body[0].revision}`);
          expect(res.body[1].origin).to.equal('neurosis');
          expect(res.body[1].revision).to.equal(revision);
          expect(res.body[1].location).to.equal(`/origins/neurosis/keys/${revision}`);
          done(err);
        });
    });
  });
});
