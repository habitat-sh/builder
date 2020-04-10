const expect = require('chai').expect;
const supertest = require('supertest');
const request = supertest('http://localhost:9636/v1');
const fs = require('fs');

const release1 = '20200401202136';
const file1 = fs.readFileSync(__dirname + `/../fixtures/rcpd-testapp-0.1.0-${release1}-x86_64-linux.hart`);
const revision = '20200401201905';
const pubFile = fs.readFileSync(__dirname + `/../fixtures/rcpd-${revision}.pub`, 'utf8');
const secretFile = fs.readFileSync(__dirname + `/../fixtures/rcpd-${revision}.sig.key`, 'utf8');

describe('Origin Roles API', function () {
  describe('Create rcpd origin', function () {
    it('returns the created origin', function (done) {
      request.post('/depot/origins')
        .set('Authorization', global.boboBearer)
        .send({ 'name': 'rcpd', 'default_package_visibility': 'private' })
        .expect(201)
        .end(function (err, res) {
          expect(res.body.name).to.equal('rcpd');
          expect(res.body.default_package_visibility).to.equal('private');
          global.originRcpd = res.body;
          done(err);
        });
    });
  });
  describe('Uploads an origin signing key', function () {
    it('uploads the key', function (done) {
      request.post(`/depot/origins/rcpd/secret_keys/${revision}`)
        .set('Authorization', global.boboBearer)
        .send(secretFile)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });
  describe('Upload a fixture package', function () {
    it('allows authenticated users to upload packages', function (done) {
      request.post(`/depot/pkgs/rcpd/testapp/0.1.0/${release1}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file1.length)
        .query({ checksum: '569bf7fa781bcf5fb167d42884728ffdb264cf9ad6ac05b2a217406070a4c7ba' })
        .send(file1)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/rcpd/testapp/0.1.0/${release1}/download`);
          done(err);
        });
    });
  });
  describe('Invite lkennedy', function () {
    it('lkennedy gets invited', function (done) {
      request.post('/depot/origins/rcpd/users/lkennedy/invitations')
        .set('Authorization', global.boboBearer)
        .expect(201)
        .end(function (err, res) {
          expect(res.body.account_id).to.equal(global.sessionLkennedy.id);
          expect(res.body.origin).to.equal(global.originRcpd.name);
          expect(res.body.owner_id).to.equal(global.sessionBobo.id);
          global.inviteLkennedyToRcpd = res.body;
          done(err);
        });
    });
  });
  describe('lkennedy accepts the invite', function () {
    it('lkennedy joins the origin', function (done) {
      request.put('/depot/origins/rcpd/invitations/' + global.inviteLkennedyToRcpd.id)
        .set('Authorization', global.lkennedyBearer)
        .expect(204)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });
  describe('Query a member role', function () {
    it('returns the default maintainer role for lkennedy', function (done) {
      request.get('/depot/origins/rcpd/users/lkennedy/role')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.body.role).to.equal('maintainer');
          done(err);
        });
    });
  });
  describe('Demote lkennedy to member role', function () {
    it('changes the member role', function (done) {
      request.put('/depot/origins/rcpd/users/lkennedy/role')
        .query({role: 'member'})
        .set('Authorization', global.boboBearer)
        .expect(204)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('shows updated member role', function (done) {
      request.get('/depot/origins/rcpd/users/lkennedy/role')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.body.role).to.equal('member');
          done(err);
        });
    });
  });
  describe('Read-only Behaviors', function () {
    it('user with member role not authorized to send invitation', function (done) {
      request.post('/depot/origins/rcpd/users/wesker/invitations')
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.txt).to.be.emtpy;
          done(err);
        });
    });
    it('user with member role not authorized to elevate own role', function (done) {
      request.put('/depot/origins/rcpd/users/lkennedy/role')
        .query({role: 'maintainer'})
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.txt).to.be.emtpy;
          done(err);
        });
    });
    it('user with member role not authorized to alter other user role', function (done) {
      request.put('/depot/origins/rcpd/users/bobo/role')
        .query({role: 'member'})
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.txt).to.be.emtpy;
          done(err);
        });
    });
    it('user not authorized to query role of members in other origins', function (done) {
      request.get('/depot/origins/rcpd/users/lkennedy/role')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with role member not authorized to upload packages', function (done) {
      request.post(`/depot/pkgs/rcpd/testapp/0.1.0/${release1}`)
        .set('Authorization', global.lkennedyBearer)
        .set('Content-Length', file1.length)
        .query({ checksum: '569bf7fa781bcf5fb167d42884728ffdb264cf9ad6ac05b2a217406070a4c7ba' })
        .send(file1)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with member role not authorized to delete a package', function (done) {
      request.delete('/depot/pkgs/rcpd/testapp/0.1.0/${release1}')
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with member role not authorized to create origin keys', function (done) {
      request.post('/depot/origins/rcpd/keys')
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with member role not authorized to get origin signing key', function (done) {
      request.post('/depot/origins/rcpd/secret_keys/latest')
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with member role not authorized to upload origin public key', function (done) {
      request.post('/depot/origins/rcpd/keys/${revision}')
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with member role not authorized to upload origin secret key', function (done) {
      request.post('/depot/origins/rcpd/secret_keys/${revision}')
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with member role not authorized to list origin secrets', function (done) {
      request.get('/depot/origins/rcpd/secret')
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.body.length).to.be.empty;
          done(err);
        });
    });
    it('user with member role not authorized to delete origin secrets', function (done) {
      request.delete('/depot/origins/rcpd/secret/foo')
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with member role not authorized to demote a package', function (done) {
      request.put(`/depot/channels/rcpd/unstable/pkgs/testapp/0.1.0/${release1}/demote`)
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with member role not authorized to promote a package', function (done) {
      request.put(`/depot/channels/rcpd/stable/pkgs/testapp/0.1.0/${release1}/promote`)
        .set('Authorization', global.lkennedyBearer)
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with member role not authorized to add integration', function (done) {
      request.put('/depot/origins/rcpd/integrations/docker/foo')
        .set('Authorization', global.lkennedyBearer)
        .type('application/json')
        .accept('application/json')
        .send({
          some: 'data',
          random: true,
          does_not_matter: 'haha'
        })
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with member role not authorized to update package settings', function(done) {
      request.put('/settings/rcpd/testapp')
          .send({ 'visibility': 'public'})
          .set('Authorization', global.lkennedyBearer)
          .expect(403)
          .end(function (err, res) {
              expect(res.text).to.be.empty;
              done(err);
          });
    });
    it('user with role member not authorized to update the origin settings', function (done) {
      request.put('/depot/origins/rcpd')
        .set('Authorization', global.lkennedyBearer)
        .send({ 'default_package_visibility': 'public' })
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with role member not authorized to delete associated project', function (done) {
      request.delete('/projects/rcpd/testapp')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('user with role member not authorized to trigger builds', function (done) {
      request.post('/depot/pkgs/schedule/rcpd/testapp')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.lkennedyBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });
  describe('Package visibility and non origin member searches', function () {
    it('non origin member cannot see latest release of a package when its private', function (done) {
      request.get('/depot/pkgs/rcpd/testapp/latest')
        .set('Authorization', global.hankBearer)
        .type('application/json')
        .accept('application/json')
        .expect(404)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('non origin members cannot search and find a private package', function (done) {
      request.get('/depot/pkgs/search/testapp')
        .set('Authorization', global.hankBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          index = res.body.data.findIndex(x => x.origin ==="rcpd");
          expect(index).to.equal(-1);
          done(err);
        });
    });
    it('set the origin default visibility to public', function (done) {
      request.put('/depot/origins/rcpd')
        .set('Authorization', global.boboBearer)
        .send({ 'default_package_visibility': 'public' })
        .expect(204)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it('set the package project visibility to public', function(done) {
      request.put('/settings/rcpd/testapp')
          .send({ 'visibility': 'public'})
          .set('Authorization', global.boboBearer)
          .expect(200)
          .end(function (err, res) {
              done(err);
          });
    });
    it('force re-upload the package to make its visibility public', function (done) {
      request.post(`/depot/pkgs/rcpd/testapp/0.1.0/${release1}?forced=true`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file1.length)
        .query({ checksum: '569bf7fa781bcf5fb167d42884728ffdb264cf9ad6ac05b2a217406070a4c7ba' })
        .send(file1)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/rcpd/testapp/0.1.0/${release1}/download`);
          done(err);
        });
    });
    it('non origin member can now query latest release of a package with the specified name', function (done) {
      request.get('/depot/pkgs/rcpd/testapp/latest')
        .set('Authorization', global.hankBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('rcpd');
          expect(res.body.ident.name).to.equal('testapp');
          expect(res.body.ident.version).to.equal('0.1.0');
          expect(res.body.ident.release).to.equal(release1);
          done(err);
        });
    });
    it('non origin members can also search for the package and find it', function (done) {
      request.get('/depot/pkgs/search/testapp')
        .set('Authorization', global.hankBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          index = res.body.data.findIndex(x => x.origin ==="rcpd");
          expect(res.body.data[index].origin).to.equal('rcpd');
          expect(res.body.data[index].name).to.equal('testapp');
          expect(res.body.data[index].version).to.equal('0.1.0');
          expect(res.body.data[index].release).to.equal(release1);
          done(err);
        });
    });
  });
});
