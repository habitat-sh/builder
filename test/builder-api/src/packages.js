const expect = require('chai').expect;
const supertest = require('supertest');
const binaryParser = require('superagent-binary-parser');
const request = supertest('http://localhost:9636/v1');
const fs = require('fs');

const release1 = '20171205003213';
const release2 = '20171206004121';
const release3 = '20171206004139';
const release4 = '20171206005217';
const release5 = '20181018162212';
const release6 = '20181018162220';
const release7 = '20181115124506';
const release8 = '20181116180420';
const release9 = '20190327162559';

const file1 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp-0.1.3-${release1}-x86_64-linux.hart`);
const file2 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp-0.1.3-${release2}-x86_64-linux.hart`);
const file3 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp-0.1.4-${release3}-x86_64-linux.hart`);
const file4 = fs.readFileSync(__dirname + `/../fixtures/xmen-testapp-0.1.4-${release4}-x86_64-linux.hart`);
const file5 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp2-v1.2.3-master-${release5}-x86_64-linux.hart`);
const file6 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp2-v1.2.3-aaster-${release6}-x86_64-linux.hart`);
const file7 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp-0.1.4-${release7}-x86_64-windows.hart`);
const file8 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp-0.1.3-${release8}-x86_64-linux-kernel2.hart`);
const file9 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp3-0.1.0-${release9}-x86_64-linux.hart`);

var downloadedPath = '/tmp/';

describe('Working with packages', function () {
  describe('Uploading packages', function () {
    it('does not allow unauthenticated users to upload packages', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp/0.1.3/${release1}`)
        .query({ checksum: '3138777020e7bb621a510b19c2f2630deee9b34ac11f1c2a0524a44eb977e4a8' })
        .set('Content-Length', file1.length)
        .send(file1)
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires that you are a member of the origin to upload a package', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp/0.1.3/${release1}`)
        .set('Authorization', global.mystiqueBearer)
        .set('Content-Length', file1.length)
        .query({ checksum: '3138777020e7bb621a510b19c2f2630deee9b34ac11f1c2a0524a44eb977e4a8' })
        .send(file1)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('allows authenticated users to upload packages', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp/0.1.3/${release1}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file1.length)
        .query({ checksum: '3138777020e7bb621a510b19c2f2630deee9b34ac11f1c2a0524a44eb977e4a8' })
        .send(file1)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/testapp/0.1.3/${release1}/download`);
          done(err);
        });
    });

    it('uploads a second package', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp/0.1.3/${release2}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file2.length)
        .query({ checksum: 'd8943c86636eb0a24cb63a80b3d9375ce342f2fa192375f3a0b83eab44de21eb' })
        .send(file2)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/testapp/0.1.3/${release2}/download`);
          done(err);
        });
    });

    it('uploads a third package', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp/0.1.4/${release3}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file3.length)
        .query({ checksum: '1fa27a110fe01acba9d31a0f56801c5e38f4feb8105266231f308091e487c6d1' })
        .send(file3)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/testapp/0.1.4/${release3}/download`);
          done(err);
        });
    });

    it('uploads a fourth package', function (done) {
      request.post(`/depot/pkgs/xmen/testapp/0.1.4/${release4}`)
        .set('Authorization', global.mystiqueBearer)
        .set('Content-Length', file4.length)
        .query({ checksum: 'b1661779dd7dcef994ae0ab4c2c3c589dde56747d91511cb44a41813831336a1' })
        .send(file4)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/xmen/testapp/0.1.4/${release4}/download`);
          done(err);
        });
    });

    it('uploads a windows package', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp/0.1.4/${release7}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file7.length)
        .query({ checksum: '03d05a088fe6aeca482fe276adb4b08092fdc2c8df9e6d52ef5d78e731afbaa6' })
        .send(file7)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/testapp/0.1.4/${release7}/download`);
          done(err);
        });
    });

    it('uploads a kernel2 package', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp/0.1.3/${release8}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file8.length)
        .query({ checksum: 'bdae4812e37aa8d6d29eb5beae930c69334006e44edcbbbf75ec817c5e48ca2c' })
        .send(file8)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/testapp/0.1.3/${release8}/download`);
          done(err);
        });
    });

    // Test weird versions

    it('uploads a unusual versioned package five', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp2/v1.2.3-master/${release5}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file5.length)
        .query({ checksum: '434a21b794d2ef247f5000a1f9a961060c1301cb95fb1dade3595fe6d16a3caf' })
        .send(file5)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/testapp2/v1.2.3-master/${release5}/download`);
          done(err);
        });
    });

    it('uploads a unusual versioned package six', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp2/v1.2.3-aaster/${release6}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file6.length)
        .query({ checksum: '9e034abf815708f32d30663fc4d317a6451af489406474cd806743a24493fb53' })
        .send(file6)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/testapp2/v1.2.3-aaster/${release6}/download`);
          done(err);
        });
    });
  });

  describe('Downloading packages', function () {
    it('fails for invalid target specified', function (done) {
      request.get(`/depot/pkgs/neurosis/testapp/0.1.3/${release2}/download?target=foo`)
        .expect(422)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  describe('Deleting origin after package exists', function () {
    it('is not allowed', function (done) {
      request.delete('/depot/origins/neurosis')
        .set('Authorization', global.boboBearer)
        .expect(422)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err)
        });
    });
  });

  describe('Finding packages', function () {
    it('allows me to search for packages', function (done) {
      request.get('/depot/pkgs/search/testapp')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(5);
          expect(res.body.total_count).to.equal(6);
          expect(res.body.data.length).to.equal(6);
          expect(res.body.data[0].origin).to.equal('neurosis');
          expect(res.body.data[0].name).to.equal('testapp');
          expect(res.body.data[0].version).to.equal('0.1.3');
          expect(res.body.data[0].release).to.equal(release1);
          expect(res.body.data[1].origin).to.equal('neurosis');
          expect(res.body.data[1].name).to.equal('testapp');
          expect(res.body.data[1].version).to.equal('0.1.3');
          expect(res.body.data[1].release).to.equal(release2);
          expect(res.body.data[2].origin).to.equal('neurosis');
          expect(res.body.data[2].name).to.equal('testapp');
          expect(res.body.data[2].version).to.equal('0.1.3');
          expect(res.body.data[2].release).to.equal(release8);
          expect(res.body.data[5].origin).to.equal('xmen');
          expect(res.body.data[5].name).to.equal('testapp');
          expect(res.body.data[5].version).to.equal('0.1.4');
          expect(res.body.data[5].release).to.equal(release4);
          done(err);
        });
    });

    it('allows me to search for distinct packages', function (done) {
      request.get('/depot/pkgs/search/testapp?distinct=true')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(1);
          expect(res.body.total_count).to.equal(2);
          expect(res.body.data.length).to.equal(2);
          expect(res.body.data[0].origin).to.equal('neurosis');
          expect(res.body.data[0].name).to.equal('testapp');
          expect(res.body.data[1].origin).to.equal('xmen');
          expect(res.body.data[1].name).to.equal('testapp');
          done(err);
        });
    });

    it('lists all packages', function (done) {
      request.get('/depot/pkgs/neurosis')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(6);
          expect(res.body.total_count).to.equal(7);
          expect(res.body.data.length).to.equal(7);
          expect(res.body.data[2].origin).to.equal('neurosis');
          expect(res.body.data[2].name).to.equal('testapp');
          expect(res.body.data[2].version).to.equal('0.1.4');
          expect(res.body.data[2].release).to.equal(release7);
          expect(res.body.data[3].origin).to.equal('neurosis');
          expect(res.body.data[3].name).to.equal('testapp');
          expect(res.body.data[3].version).to.equal('0.1.4');
          expect(res.body.data[3].release).to.equal(release3);
          expect(res.body.data[4].origin).to.equal('neurosis');
          expect(res.body.data[4].name).to.equal('testapp');
          expect(res.body.data[4].version).to.equal('0.1.3');
          expect(res.body.data[4].release).to.equal(release8);
          done(err);
        });
    });

    it('lists all distinct packages', function (done) {
      request.get('/depot/pkgs/neurosis?distinct=true')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(1);
          expect(res.body.total_count).to.equal(2);
          expect(res.body.data.length).to.equal(2);
          expect(res.body.data[0].origin).to.equal('neurosis');
          expect(res.body.data[0].name).to.equal('testapp');
          expect(res.body.data[1].origin).to.equal('neurosis');
          expect(res.body.data[1].name).to.equal('testapp2');
          done(err);
        });
    });

    it('lists all unique package names', function (done) {
      request.get('/depot/neurosis/pkgs')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(1);
          expect(res.body.total_count).to.equal(2);
          expect(res.body.data.length).to.equal(2);
          expect(res.body.data[0].origin).to.equal('neurosis');
          expect(res.body.data[0].name).to.equal('testapp');
          done(err);
        });
    });

    it('lists all packages with the specified name', function (done) {
      request.get('/depot/pkgs/neurosis/testapp')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(4);
          expect(res.body.total_count).to.equal(5);
          expect(res.body.data.length).to.equal(5);
          expect(res.body.data[0].origin).to.equal('neurosis');
          expect(res.body.data[0].name).to.equal('testapp');
          expect(res.body.data[0].version).to.equal('0.1.4');
          expect(res.body.data[0].release).to.equal(release7);
          expect(res.body.data[1].platforms[0]).to.equal('x86_64-linux');
          expect(res.body.data[1].origin).to.equal('neurosis');
          expect(res.body.data[1].name).to.equal('testapp');
          expect(res.body.data[1].version).to.equal('0.1.4');
          expect(res.body.data[1].release).to.equal(release3);
          expect(res.body.data[1].platforms[0]).to.equal('x86_64-linux');
          expect(res.body.data[2].origin).to.equal('neurosis');
          expect(res.body.data[2].name).to.equal('testapp');
          expect(res.body.data[2].version).to.equal('0.1.3');
          expect(res.body.data[2].release).to.equal(release8);
          expect(res.body.data[1].platforms[0]).to.equal('x86_64-linux');
          done(err);
        });
    });

    it('lists all versions of the package with the specified name', function (done) {
      request.get('/depot/pkgs/neurosis/testapp/versions')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(2);
          expect(res.body[0].origin).to.equal('neurosis');
          expect(res.body[0].name).to.equal('testapp');
          expect(res.body[0].version).to.equal('0.1.4');
          expect(res.body[0].release_count).to.equal('2');
          expect(res.body[0].latest).to.equal(release7);
          expect(res.body[0].platforms.length).to.equal(2);
          expect(res.body[0].platforms[0]).to.equal('x86_64-linux');
          expect(res.body[1].origin).to.equal('neurosis');
          expect(res.body[1].name).to.equal('testapp');
          expect(res.body[1].version).to.equal('0.1.3');
          expect(res.body[1].release_count).to.equal('3');
          expect(res.body[1].latest).to.equal(release8);
          expect(res.body[1].platforms.length).to.equal(2);
          expect(res.body[1].platforms[0]).to.equal('x86_64-linux');
          done(err);
        });
    });

    it('returns the latest release of a package with the specified name', function (done) {
      request.get('/depot/pkgs/neurosis/testapp/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('testapp');
          expect(res.body.ident.version).to.equal('0.1.4');
          expect(res.body.ident.release).to.equal(release3);
          done(err);
        });
    });

    it('returns the latest release of a package with the specified odd name', function (done) {
      request.get('/depot/pkgs/neurosis/testapp2/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('testapp2');
          expect(res.body.ident.version).to.equal('v1.2.3-master');
          expect(res.body.ident.release).to.equal(release5);
          done(err);
        });
    });

    it('lists all packages with the specified name and version', function (done) {
      request.get('/depot/pkgs/neurosis/testapp/0.1.3')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(2);
          expect(res.body.total_count).to.equal(3);
          expect(res.body.data.length).to.equal(3);
          expect(res.body.data[0].origin).to.equal('neurosis');
          expect(res.body.data[0].name).to.equal('testapp');
          expect(res.body.data[0].version).to.equal('0.1.3');
          expect(res.body.data[0].release).to.equal(release8);
          expect(res.body.data[1].platforms[0]).to.equal('x86_64-linux');
          expect(res.body.data[1].origin).to.equal('neurosis');
          expect(res.body.data[1].name).to.equal('testapp');
          expect(res.body.data[1].version).to.equal('0.1.3');
          expect(res.body.data[1].release).to.equal(release2);
          expect(res.body.data[1].platforms[0]).to.equal('x86_64-linux');
          done(err);
        });
    });

    it('returns the latest release of a package with the spcified name and version', function (done) {
      request.get('/depot/pkgs/neurosis/testapp/0.1.3/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('testapp');
          expect(res.body.ident.version).to.equal('0.1.3');
          expect(res.body.ident.release).to.equal(release2);
          done(err);
        });
    });

    it('returns the specified release', function (done) {
      request.get(`/depot/pkgs/neurosis/testapp/0.1.3/${release2}`)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('testapp');
          expect(res.body.ident.version).to.equal('0.1.3');
          expect(res.body.ident.release).to.equal(release2);
          done(err);
        });
    });
  });

  describe('Deleting packages', function () {
    it('uploads a leaf node package', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp3/0.1.0/${release9}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file9.length)
        .query({ checksum: '02edaaf2d5fdb167e57026b17c86e8df5a7ca285e042f113bcb31ede765a67ce' })
        .send(file9)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/testapp3/0.1.0/${release9}/download`);
          done(err);
        });
    });

    it('puts the uploaded package into the stable channel', function (done) {
      request.put(`/depot/channels/neurosis/stable/pkgs/testapp3/0.1.0/${release9}/promote`)
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires authentication', function (done) {
      request.delete(`/depot/pkgs/neurosis/testapp3/0.1.0/${release9}`)
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('fails for package in stable channel', function (done) {
      request.delete(`/depot/pkgs/neurosis/testapp3/0.1.0/${release9}`)
        .set('Authorization', global.boboBearer)
        .expect(422)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err)
        });
    });

    it('demotes the uploaded package from the stable channel', function (done) {
      request.put(`/depot/channels/neurosis/stable/pkgs/testapp3/0.1.0/${release9}/demote`)
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('fails for non-leaf packages', function (done) {
      request.delete(`/depot/pkgs/neurosis/testapp/0.1.3/${release2}`)
        .set('Authorization', global.boboBearer)
        .expect(422)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err)
        });
    });

    it('succeeds for non-stable, leaf packages', function (done) {
      request.delete(`/depot/pkgs/neurosis/testapp3/0.1.0/${release9}`)
        .set('Authorization', global.boboBearer)
        .expect(204)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err)
        });
    });
  });

  describe('Other functions', function () {
    it('lists all the channels a package is in', function (done) {
      request.get(`/depot/pkgs/neurosis/testapp/0.1.3/${release2}/channels`)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(1);
          expect(res.body[0]).to.equal('unstable');
          done(err);
        });
    });

    it('downloads a package', function (done) {
      request.get(`/depot/pkgs/neurosis/testapp/0.1.3/${release2}/download`)
        .expect(200)
        .buffer()
        .parse(binaryParser)
        .end(function (err, res) {
          var name = res.header['x-filename'];
          var path = downloadedPath + name;
          fs.writeFileSync(path, res.body);
          var size = fs.statSync(path).size;
          expect(name).to.equal(`neurosis-testapp-0.1.3-${release2}-x86_64-linux.hart`)
          expect(size).to.equal(1569);
          done(err);
        });
    });

    it('toggles the privacy setting for a package', function (done) {
      request.patch(`/depot/pkgs/neurosis/testapp/0.1.3/${release2}/private`)
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires authentication to view private packages', function (done) {
      request.get(`/depot/pkgs/neurosis/testapp/0.1.3/${release2}`)
        .type('application/json')
        .accept('application/json')
        .expect(404)
        .end(function (err, res) {
          done(err);
        });
    });

    it('does not let members of other origins view private packages', function (done) {
      request.get(`/depot/pkgs/neurosis/testapp/0.1.3/${release2}`)
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.mystiqueBearer)
        .expect(404)
        .end(function (err, res) {
          done(err);
        });
    });

    it('allows members of the origin to view private packages when they are authenticated', function (done) {
      request.get(`/depot/pkgs/neurosis/testapp/0.1.3/${release2}`)
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('testapp');
          expect(res.body.ident.version).to.equal('0.1.3');
          expect(res.body.ident.release).to.equal(release2);
          done(err);
        });
    });
  });
});
