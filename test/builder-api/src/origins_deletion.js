const expect = require('chai').expect;
const supertest = require('supertest');
const request = supertest('http://localhost:9636/v1');
const fs = require('fs');

// These magic values correspond to the testpp repo in the habitat-sh org
const installationId = 56940;
const repoId = 114932712;
const projectCreatePayload = {
  origin: 'deletemeifyoucan',
  plan_path: 'plan.sh',
  installation_id: installationId,
  repo_id: repoId,
  auto_build: true
};

const release1 = '20200205165325';
const file1 = fs.readFileSync(__dirname + `/../fixtures/deletemeifyoucan-testapp-0.1.3-${release1}-x86_64-linux.hart`);
const revision = '20200205153202';
const pubFile = fs.readFileSync(__dirname + `/../fixtures/deletemeifyoucan-${revision}.pub`, 'utf8');
const secretFile = fs.readFileSync(__dirname + `/../fixtures/deletemeifyoucan-${revision}.sig.key`, 'utf8');

describe('Origin Deletion - Extended Tests', function() {
  describe('[Prereq 1]: create an origin and project', function() {
    it('creates an origin', function(done) {
      request.post('/depot/origins')
        .set('Authorization', global.boboBearer)
        .send({
          'name': 'deletemeifyoucan',
          'default_package_visibility': 'private'
        })
        .expect(201)
        .end(function(err, res) {
          expect(res.body.name).to.equal('deletemeifyoucan');
          done(err);
        });
    });
    it('creates a project', function(done) {
      this.timeout(5000);
      request.post('/projects')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .send(projectCreatePayload)
        .expect(201)
        .end(function(err, res) {
          done(err);
        });
    });
  });

  describe('1. Attempt to delete an origin when a project exists', function() {
    it('gets 409 before the project is deleted', function(done) {
      request.delete('/depot/origins/deletemeifyoucan')
        .set('Authorization', global.boboBearer)
        .expect(409)
        .end(function(err, res) {
          expect(res.text).to.match(/^There are 1 projects remaining in origin deletemeifyoucan./);
          done(err);
        });
    });
    it('deletes the origin project', function(done) {
      request.delete('/projects/deletemeifyoucan/testapp')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .expect(204)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('succeeds after the project was deleted', function(done) {
      request.delete('/depot/origins/deletemeifyoucan')
        .set('Authorization', global.boboBearer)
        .expect(204)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('gets a 404 on the origin after deletion', function(done) {
      request.get('/depot/origins/deletemeifyoucan')
        .expect(404)
        .end(function(err, res) {
          done(err);
        });
    });
  });

  describe('[Prereq 2]: create an origin and add a member', function() {
    it('creates an origin', function(done) {
      request.post('/depot/origins')
        .set('Authorization', global.boboBearer)
        .send({
          'name': 'deletemeifyoucan',
          'default_package_visibility': 'private'
        })
        .expect(201)
        .end(function(err, res) {
          expect(res.body.name).to.equal('deletemeifyoucan');
          expect(res.body.default_package_visibility).to.equal('private');
          done(err);
        });
    });
    it('invites a member', function(done) {
      request.post('/depot/origins/deletemeifyoucan/users/wesker/invitations')
        .set('Authorization', global.boboBearer)
        .expect(201)
        .end(function(err, res) {
          expect(res.body.origin).to.equal('deletemeifyoucan');
          global.inviteWeskerToDeleteMeIfYouCan = res.body;
          done(err);
        });
    });
    it('accepts the invitation', function(done) {
      request.put('/depot/origins/deletemeifyoucan/invitations/' + global.inviteWeskerToDeleteMeIfYouCan.id)
        .set('Authorization', global.weskerBearer)
        .expect(204)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  describe('2. Attempt to delete an origin when an additional member exists', function() {
    it('gets 409 before the non-owner member is removed', function(done) {
      request.delete('/depot/origins/deletemeifyoucan')
        .set('Authorization', global.boboBearer)
        .expect(409)
        .end(function(err, res) {
          expect(res.text).to.match(/^There are 2 members remaining in origin deletemeifyoucan./);
          done(err);
        });
    });
    it('departs the non-owner member', function(done) {
      request.post('/depot/origins/deletemeifyoucan/depart')
        .set('Authorization', global.weskerBearer)
        .expect(204)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err)
        });
    });
    it('succeeds when no additional members exist', function(done) {
      request.delete('/depot/origins/deletemeifyoucan')
        .set('Authorization', global.boboBearer)
        .expect(204)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('gets a 404 on the origin after deletion', function(done) {
      request.get('/depot/origins/deletemeifyoucan')
        .expect(404)
        .end(function(err, res) {
          done(err);
        });
    });
  });

  describe('[Prereq 3]: create an origin and add an integration', function() {
    it('creates an origin', function(done) {
      request.post('/depot/origins')
        .set('Authorization', global.boboBearer)
        .send({
          'name': 'deletemeifyoucan',
          'default_package_visibility': 'private'
        })
        .expect(201)
        .end(function(err, res) {
          expect(res.body.name).to.equal('deletemeifyoucan');
          expect(res.body.default_package_visibility).to.equal('private');
          done(err);
        });
    });
    it('adds an integration', function(done) {
      request.put('/depot/origins/deletemeifyoucan/integrations/docker/testintegration')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .send({
          some: 'data',
          random: true,
          does_not_matter: 'haha'
        })
        .expect(201)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  describe('3. Attempt to delete an origin when an integration exists', function() {
    it('gets 409 before the integration is removed', function(done) {
      request.delete('/depot/origins/deletemeifyoucan')
        .set('Authorization', global.boboBearer)
        .expect(409)
        .end(function(err, res) {
          expect(res.text).to.match(/^There are 1 integrations remaining in origin deletemeifyoucan./);
          done(err);
        });
    });
    it('removes the integration', function(done) {
      request.delete('/depot/origins/deletemeifyoucan/integrations/docker/testintegration')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .expect(204)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('succeeds after the integration was removed', function(done) {
      request.delete('/depot/origins/deletemeifyoucan')
        .set('Authorization', global.boboBearer)
        .expect(204)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('gets a 404 on the origin after deletion', function(done) {
      request.get('/depot/origins/deletemeifyoucan')
        .expect(404)
        .end(function(err, res) {
          done(err);
        });
    });
  });

  describe('[Prereq 4]: create an origin and add a channel', function() {
    it('creates an origin', function(done) {
      request.post('/depot/origins')
        .set('Authorization', global.boboBearer)
        .send({
          'name': 'deletemeifyoucan',
          'default_package_visibility': 'private'
        })
        .expect(201)
        .end(function(err, res) {
          expect(res.body.name).to.equal('deletemeifyoucan');
          expect(res.body.default_package_visibility).to.equal('private');
          done(err);
        });
    });
    it('creates a channel', function(done) {
      request.post('/depot/channels/deletemeifyoucan/testchan')
        .set('Authorization', global.boboBearer)
        .expect(201)
        .end(function(err, res) {
          expect(res.body.name).to.equal('testchan');
          global.channelFoo = res.body;
          done(err);
        });
    });
  });

  describe('4. Attempt to delete an origin when channel exists', function() {
    it('gets 409 before the channel is removed', function(done) {
      request.delete('/depot/origins/deletemeifyoucan')
        .set('Authorization', global.boboBearer)
        .expect(409)
        .end(function(err, res) {
          expect(res.text).to.match(/^There are 3 channels remaining in origin deletemeifyoucan. Only two are allowed \[unstable, stable\]/);
          done(err);
        });
    });
    it('deletes the channel', function(done) {
      request.delete('/depot/channels/deletemeifyoucan/testchan')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('succeeds after the channel was removed', function(done) {
      request.delete('/depot/origins/deletemeifyoucan')
        .set('Authorization', global.boboBearer)
        .expect(204)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('gets a 404 on the origin after deletion', function(done) {
      request.get('/depot/origins/deletemeifyoucan')
        .expect(404)
        .end(function(err, res) {
          done(err);
        });
    });
  });

  describe('[Prereq 5]: create an origin and upload a package', function() {
    it('creates an origin', function(done) {
      request.post('/depot/origins')
        .set('Authorization', global.boboBearer)
        .send({
          'name': 'deletemeifyoucan',
          'default_package_visibility': 'private'
        })
        .expect(201)
        .end(function(err, res) {
          expect(res.body.name).to.equal('deletemeifyoucan');
          expect(res.body.default_package_visibility).to.equal('private');
          done(err);
        });
    });
    it('uploads the pub signing key', function(done) {
      request.post(`/depot/origins/deletemeifyoucan/keys/${revision}`)
        .set('Authorization', global.boboBearer)
        .send(pubFile)
        .expect(201)
        .end(function(err, res) {
          expect(res.text).to.equal(`/origins/deletemeifyoucan/keys/${revision}`);
          done(err);
        });
    });
    it('uploads the package', function(done) {
      request.post(`/depot/pkgs/deletemeifyoucan/testapp/0.1.3/${release1}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file1.length)
        .query({
          checksum: '11e7e19b9349f0e92fe454a5368e3e50422604a509e821bf538284aad0c984c1'
        })
        .send(file1)
        .expect(201)
        .end(function(err, res) {
          expect(res.text).to.equal(`/pkgs/deletemeifyoucan/testapp/0.1.3/${release1}/download`);
          done(err);
        });
    });
  });

  describe('5. Attempt to delete an origin when a package exists', function() {
    it('gets 409 before the package is removed', function(done) {
      request.delete('/depot/origins/deletemeifyoucan')
        .set('Authorization', global.boboBearer)
        .expect(409)
        .end(function(err, res) {
          expect(res.text).to.match(/^There are 1 packages remaining in origin deletemeifyoucan./);
          done(err);
        });
    });
    it('deletes the package', function(done) {
      request.delete(`/depot/pkgs/deletemeifyoucan/testapp/0.1.3/${release1}`)
        .set('Authorization', global.boboBearer)
        .expect(204)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err)
        });
    });
    it('succeeds after the package was deleted', function(done) {
      request.delete('/depot/origins/deletemeifyoucan')
        .set('Authorization', global.boboBearer)
        .expect(204)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('gets a 404 on the origin after deletion', function(done) {
      request.get('/depot/origins/deletemeifyoucan')
        .expect(404)
        .end(function(err, res) {
          done(err);
        });
    });
  });

  // TODO: Add an additional test for origin deletion when origin secret(s)
  // exist. It turns out that testing creation of an origin secret here presents quite
  // a challenge.

});
