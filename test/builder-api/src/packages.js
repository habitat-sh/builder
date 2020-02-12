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
const release10 = '20190511004436';
const release11 = '20190618173321';
const release12 = '20190618175235';

const file1 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp-0.1.3-${release1}-x86_64-linux.hart`);
const file2 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp-0.1.3-${release2}-x86_64-linux.hart`);
const file3 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp-0.1.4-${release3}-x86_64-linux.hart`);
const file4 = fs.readFileSync(__dirname + `/../fixtures/xmen-testapp-0.1.4-${release4}-x86_64-linux.hart`);
const file5 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp2-v1.2.3-master-${release5}-x86_64-linux.hart`);
const file6 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp2-v1.2.3-aaster-${release6}-x86_64-linux.hart`);
const file7 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp-0.1.4-${release7}-x86_64-windows.hart`);
const file8 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp-0.1.3-${release8}-x86_64-linux-kernel2.hart`);
const file9 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp3-0.1.0-${release9}-x86_64-linux.hart`);
const file10 = fs.readFileSync(__dirname + `/../fixtures/neurosis-testapp-0.1.13-${release10}-x86_64-linux.hart`);
const file11 = fs.readFileSync(__dirname + `/../fixtures/neurosis-neurosis-2.0-${release11}-x86_64-linux.hart`);
const file12 = fs.readFileSync(__dirname + `/../fixtures/neurosis-abracadabra-3.0-${release12}-x86_64-linux.hart`);

const fakefile1 = fs.readFileSync(__dirname + `/../fixtures/fake/neurosis-testapp-0.1.3-${release1}-x86_64-linux.hart`);

const ov11release = '20190510185610';
const ov12release = '20190510185527';
const ov13release = '20190510185500';
const ov14release = '20190510185946';
const ov15release = '20190510215446';

const ov21release = '20190510223726';
const ov22release = '20190510223656';
const ov23release = '20190510223644';

const ov31release = '20190510225029';
const ov32release = '20190510225040';

const ov41release = '20190510235915';
const ov42release = '20190510235906';

const ov51release = '20190513181055';
const ov52release = '20190513181032';

const ov61release = '20190513194138';
const ov62release = '20190513194219';
const ov63release = '20190513194206';
const ov64release = '20190513194234';

const ov71release = '20190531174743';
const ov72release = '20190531174711';
const ov73release = '20190531185313';

const ov81release = '20190617213055';
const ov82release = '20190617213102';

const ov11 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion1-R16B-${ov11release}-x86_64-linux.hart`);
const ov12 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion1-R16B02-5-${ov12release}-x86_64-linux.hart`);
const ov13 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion1-R16B03-1-${ov13release}-x86_64-linux.hart`);
const ov14 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion1-R9Z-9-${ov14release}-x86_64-linux.hart`);
const ov15 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion1-R9B03-1-${ov15release}-x86_64-linux.hart`);

const ov21 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion2-19.209-37-${ov21release}-x86_64-linux.hart`);
const ov22 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion2-19.227-15-${ov22release}-x86_64-linux.hart`);
const ov23 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion2-19.227-19-${ov23release}-x86_64-linux.hart`);

const ov31 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion3-7u79-${ov31release}-x86_64-linux.hart`);
const ov32 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion3-7u80-${ov32release}-x86_64-linux.hart`);

const ov41 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion4-1.0.2h-${ov41release}-x86_64-linux.hart`);
const ov42 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion4-1.0.2r-${ov42release}-x86_64-linux.hart`);

const ov51 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion5-0.0~r131-${ov51release}-x86_64-linux.hart`);
const ov52 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion5-1.7.3-${ov52release}-x86_64-linux.hart`);

const ov61 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion6-1.2.3-${ov61release}-x86_64-linux.hart`);
const ov62 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion6-1.2.3-beta12-${ov62release}-x86_64-linux.hart`);
const ov63 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion6-1.2.3-beta2-${ov63release}-x86_64-linux.hart`);
const ov64 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion6-2.0-${ov64release}-x86_64-linux.hart`);

const ov71 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion7-17.1.0-dev.cloud-${ov71release}-x86_64-linux.hart`);
const ov72 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion7-19.1.0.dev-cloud-${ov72release}-x86_64-linux.hart`);
const ov73 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion7-19.1.0.-${ov73release}-x86_64-linux.hart`);

const ov81 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion8-4-${ov81release}-x86_64-linux.hart`);
const ov82 = fs.readFileSync(__dirname + `/../fixtures/oddversions/neurosis-oddversion8-5-${ov82release}-x86_64-linux.hart`);

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

    it('uploads does not allow duplicate upload', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp/0.1.3/${release2}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file2.length)
        .query({ checksum: 'd8943c86636eb0a24cb63a80b3d9375ce342f2fa192375f3a0b83eab44de21eb' })
        .send(file2)
        .expect(409)
        .end(function (err, res) {
          expect(res.text).to.be.empty
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

    it('uploads a fifth package', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp/0.1.13/${release10}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file10.length)
        .query({ checksum: '583bf168a02b632af5fce26c06d5f29ae9af011750284595df977160be930db7' })
        .send(file10)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/testapp/0.1.13/${release10}/download`);
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

    it('Uploads package with same name as origin', function (done) {
      request.post(`/depot/pkgs/neurosis/neurosis/2.0/${release11}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file11.length)
        .query({ checksum: 'd41a07f6e54bed1df00f0b6a15d8ed10890a442af4538991fdecbfe5fcbff866' })
        .send(file11)
        .expect(201)
        .end(function (err, res) {
          ov15
          done(err);
        });
    });

    it('Uploads package with a alphabetically smaller name', function (done) {
      request.post(`/depot/pkgs/neurosis/abracadabra/3.0/${release12}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file12.length)
        .query({ checksum: 'c7f7a6b254b8d6637edff3336ba13444543d675a38ea947eb7fc88e985b8ea7f' })
        .send(file12)
        .expect(201)
        .end(function (err, res) {
          ov15
          done(err);
        });
    });
  });

  describe('Re-uploading package', function () {
    it('fails when a force flag is not specified', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp/0.1.3/${release1}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file1.length)
        .query({ checksum: '3138777020e7bb621a510b19c2f2630deee9b34ac11f1c2a0524a44eb977e4a8' })
        .send(file1)
        .expect(409)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('succeeds when forced on package with same checksum', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp/0.1.3/${release1}?forced=true`)
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

    it('fails when forced on package with different checksum', function (done) {
      request.post(`/depot/pkgs/neurosis/testapp/0.1.3/${release1}?forced=true`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', fakefile1.length)
        .query({ checksum: '918eecd70c8bb5d665af71fbd8156ac4aa6baee8bff28af70ee1ebf63d54a1cf' })
        .send(fakefile1)
        .expect(422)
        .end(function (err, res) {
          expect(res.text).to.equal('ds:up:4');
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
        .expect(409)
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
          expect(res.body.range_end).to.equal(6);
          expect(res.body.total_count).to.equal(7);
          expect(res.body.data.length).to.equal(7);
          expect(res.body.data[0].origin).to.equal('neurosis');
          expect(res.body.data[0].name).to.equal('testapp');
          expect(res.body.data[0].version).to.equal('0.1.13');
          expect(res.body.data[0].release).to.equal(release10);
          expect(res.body.data[1].version).to.equal('0.1.3');
          expect(res.body.data[1].release).to.equal(release1);
          expect(res.body.data[2].origin).to.equal('neurosis');
          expect(res.body.data[2].name).to.equal('testapp');
          expect(res.body.data[2].version).to.equal('0.1.3');
          expect(res.body.data[2].release).to.equal(release2);
          expect(res.body.data[3].origin).to.equal('neurosis');
          expect(res.body.data[3].name).to.equal('testapp');
          expect(res.body.data[3].version).to.equal('0.1.3');
          expect(res.body.data[3].release).to.equal(release8);
          expect(res.body.data[6].origin).to.equal('xmen');
          expect(res.body.data[6].name).to.equal('testapp');
          expect(res.body.data[6].version).to.equal('0.1.4');
          expect(res.body.data[6].release).to.equal(release4);
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
          expect(res.body.range_end).to.equal(9);
          expect(res.body.total_count).to.equal(10);
          expect(res.body.data.length).to.equal(10);
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
          expect(res.body.range_end).to.equal(3);
          expect(res.body.total_count).to.equal(4);
          expect(res.body.data.length).to.equal(4);
          expect(res.body.data[0].origin).to.equal('neurosis');
          expect(res.body.data[0].name).to.equal('abracadabra');
          expect(res.body.data[1].origin).to.equal('neurosis');
          expect(res.body.data[1].name).to.equal('neurosis');
          expect(res.body.data[2].origin).to.equal('neurosis');
          expect(res.body.data[2].name).to.equal('testapp');
          expect(res.body.data[3].origin).to.equal('neurosis');
          expect(res.body.data[3].name).to.equal('testapp2');
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
          expect(res.body.range_end).to.equal(3);
          expect(res.body.total_count).to.equal(4);
          expect(res.body.data.length).to.equal(4);
          expect(res.body.data[0].origin).to.equal('neurosis');
          expect(res.body.data[0].name).to.equal('abracadabra');
          expect(res.body.data[1].origin).to.equal('neurosis');
          expect(res.body.data[1].name).to.equal('neurosis');
          expect(res.body.data[2].origin).to.equal('neurosis');
          expect(res.body.data[2].name).to.equal('testapp');
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
          expect(res.body.range_end).to.equal(5);
          expect(res.body.total_count).to.equal(6);
          expect(res.body.data.length).to.equal(6);
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
          expect(res.body.length).to.equal(3);
          expect(res.body[0].origin).to.equal('neurosis');
          expect(res.body[0].name).to.equal('testapp');
          expect(res.body[0].version).to.equal('0.1.13');
          expect(res.body[0].release_count).to.equal('1');
          expect(res.body[0].latest).to.equal(release10);
          expect(res.body[0].platforms.length).to.equal(1);
          expect(res.body[0].platforms[0]).to.equal('x86_64-linux');

          expect(res.body[1].origin).to.equal('neurosis');
          expect(res.body[1].name).to.equal('testapp');
          expect(res.body[1].version).to.equal('0.1.4');
          expect(res.body[1].release_count).to.equal('2');
          expect(res.body[1].latest).to.equal(release7);
          expect(res.body[1].platforms.length).to.equal(2);
          expect(res.body[1].platforms[0]).to.equal('x86_64-linux');

          expect(res.body[2].origin).to.equal('neurosis');
          expect(res.body[2].name).to.equal('testapp');
          expect(res.body[2].version).to.equal('0.1.3');
          expect(res.body[2].release_count).to.equal('3');
          expect(res.body[2].latest).to.equal(release8);
          expect(res.body[2].platforms.length).to.equal(2);
          expect(res.body[2].platforms[0]).to.equal('x86_64-linux');
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
          expect(res.body.ident.version).to.equal('0.1.13');
          expect(res.body.ident.release).to.equal(release10);
          done(err);
        });
    });

    it('returns the latest release of a package with the same origin name', function (done) {
      request.get('/depot/pkgs/neurosis/neurosis/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('neurosis');
          expect(res.body.ident.version).to.equal('2.0');
          expect(res.body.ident.release).to.equal(release11);
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

    it('requires a member of the origin', function (done) {
      request.delete(`/depot/pkgs/neurosis/testapp3/0.1.0/${release9}`)
        .set('Authorization', global.mystiqueBearer)
        .expect(403)
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

    it('returns the package in the latest call', function (done) {
      request.get('/depot/pkgs/neurosis/testapp3/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('testapp3');
          expect(res.body.ident.version).to.equal('0.1.0');
          expect(res.body.ident.release).to.equal(release9);
          done(err);
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

    it('doesnt return the package in the latest call', function (done) {
      request.get('/depot/pkgs/neurosis/testapp3/latest')
        .type('application/json')
        .accept('application/json')
        .expect(404)
        .end(function (err, res) {
          expect(res.text).to.be.empty
          done(err);
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
        .set('Authorization', global.weskerBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('testapp');
          expect(res.body.ident.version).to.equal('0.1.3');
          expect(res.body.ident.release).to.equal(release2);
          done(err);
        });
    });

    it('allows owners of the origin to view private packages when they are authenticated', function (done) {
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

  describe('Behavior of non-standard version packages', function () {
    it('Uploads odd version package11', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion1/R16B/${ov11release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov11.length)
        .query({ checksum: '6b93c30c70496417339cca4f38390595ab5d68e1c8d6afa787913d985391627c' })
        .send(ov11)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('Uploads odd version package12', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion1/R16B02-5/${ov12release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov12.length)
        .query({ checksum: '08c38df9611b3341f1e44cc9d7c593385e013b91bd8268b429aa5daebcb574f8' })
        .send(ov12)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('Uploads odd version package13', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion1/R16B03-1/${ov13release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov13.length)
        .query({ checksum: '42ca8907514bef70aecc46c2d0608f0f65001019c2ccfeda5507a7bf687a49ef' })
        .send(ov13)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('Uploads odd version package14', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion1/R9Z-9/${ov14release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov14.length)
        .query({ checksum: 'ce5b5cb8904863cbdf02f72683f6c55e072b030a075dca2c027c422a0aad41e0' })
        .send(ov14)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('Uploads odd version package15', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion1/R9B03-1/${ov15release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov15.length)
        .query({ checksum: '7704e24f0b2e069cdc8c6963d0701d91f2397ed4e740576b34282e1f4ba2f3e8' })
        .send(ov15)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('returns the latest release of package oddversion1', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion1/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion1');
          expect(res.body.ident.version).to.equal('R16B03-1');
          expect(res.body.ident.release).to.equal(ov13release);
          done(err);
        });
    });

    it('returns the latest package oddversion1 in unstable channel', function (done) {
      request.get('/depot/channels/neurosis/unstable/pkgs/oddversion1/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion1');
          expect(res.body.ident.version).to.equal('R16B03-1');
          expect(res.body.ident.release).to.equal(ov13release);
          done(err);
        });
    });

    it('lists all versions of the package oddversion1', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion1/versions')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(5);
          expect(res.body[0].origin).to.equal('neurosis');
          expect(res.body[0].name).to.equal('oddversion1');
          expect(res.body[0].version).to.equal('R16B03-1');
          expect(res.body[0].latest).to.equal(ov13release);
          done(err);
        });
    });

    it('Uploads odd version package21', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion2/19.209-37/${ov21release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov21.length)
        .query({ checksum: '471f11e24a48babfc9958b29f01dcf44a3bfc2804389c1b58118c01db7c25d60' })
        .send(ov21)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('Uploads odd version package22', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion2/19.227-15/${ov22release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov22.length)
        .query({ checksum: '23e9fc237bf7609dfa9c8065d7ae38e254fd7e23ccdddde6aa64d867eb3d5108' })
        .send(ov22)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('Uploads odd version package23', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion2/19.227-19/${ov23release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov23.length)
        .query({ checksum: '8cfe7447eba22d2c927f9a44d254e1e84e0e7f35787db20768e9602a8495e8ec' })
        .send(ov23)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('returns the latest release of package oddversion2', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion2/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion2');
          expect(res.body.ident.version).to.equal('19.227-19');
          expect(res.body.ident.release).to.equal(ov23release);
          done(err);
        });
    });

    it('returns the latest package oddversion2 in unstable channel', function (done) {
      request.get('/depot/channels/neurosis/unstable/pkgs/oddversion2/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion2');
          expect(res.body.ident.version).to.equal('19.227-19');
          expect(res.body.ident.release).to.equal(ov23release);
          done(err);
        });
    });

    it('lists all versions of the package oddversion2', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion2/versions')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(3);
          expect(res.body[0].origin).to.equal('neurosis');
          expect(res.body[0].name).to.equal('oddversion2');
          expect(res.body[0].version).to.equal('19.227-19');
          expect(res.body[0].latest).to.equal(ov23release);
          done(err);
        });
    });

    it('Uploads odd version package31', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion3/7u79/${ov31release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov31.length)
        .query({ checksum: '6402539d622f72d51dd72ca225b49f38f29d7412c5863ab7f67bdf58957af6ab' })
        .send(ov31)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('Uploads odd version package32', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion3/7u80/${ov32release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov32.length)
        .query({ checksum: 'b4ebf6e8eabf41729899a8776cc2126d9daf113903281e2d7019467e3722f778' })
        .send(ov32)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('returns the latest release of package oddversion3', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion3/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion3');
          expect(res.body.ident.version).to.equal('7u80');
          expect(res.body.ident.release).to.equal(ov32release);
          done(err);
        });
    });

    it('returns the latest package oddversion3 in unstable channel', function (done) {
      request.get('/depot/channels/neurosis/unstable/pkgs/oddversion3/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion3');
          expect(res.body.ident.version).to.equal('7u80');
          expect(res.body.ident.release).to.equal(ov32release);
          done(err);
        });
    });

    it('lists all versions of the package oddversion3', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion3/versions')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(2);
          expect(res.body[0].origin).to.equal('neurosis');
          expect(res.body[0].name).to.equal('oddversion3');
          expect(res.body[0].version).to.equal('7u80');
          expect(res.body[0].latest).to.equal(ov32release);
          done(err);
        });
    });

    it('Uploads odd version package41', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion4/1.0.2h/${ov41release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov41.length)
        .query({ checksum: '2967ed873f8a1693b4aaf3cc6da39afd40cad5bd8543fa0b6d9f4d22b73c0a17' })
        .send(ov41)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('Uploads odd version package42', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion4/1.0.2r/${ov42release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov42.length)
        .query({ checksum: '01c042683f684ab06ce78c87ad25e959853e6f50064dcfd3f006698867f635ff' })
        .send(ov42)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('returns the latest release of package oddversion4', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion4/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion4');
          expect(res.body.ident.version).to.equal('1.0.2r');
          expect(res.body.ident.release).to.equal(ov42release);
          done(err);
        });
    });

    it('returns the latest package oddversion4 in unstable channel', function (done) {
      request.get('/depot/channels/neurosis/unstable/pkgs/oddversion4/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion4');
          expect(res.body.ident.version).to.equal('1.0.2r');
          expect(res.body.ident.release).to.equal(ov42release);
          done(err);
        });
    });

    it('lists all versions of the package oddversion4', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion4/versions')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(2);
          expect(res.body[0].origin).to.equal('neurosis');
          expect(res.body[0].name).to.equal('oddversion4');
          expect(res.body[0].version).to.equal('1.0.2r');
          expect(res.body[0].latest).to.equal(ov42release);
          done(err);
        });
    });

    it('Uploads odd version package51', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion5/0.0~r131/${ov51release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov51.length)
        .query({ checksum: '244adeb0e7d326eebe628c556ef407ed89b11e6559675fb86687e330ebb2a3dd' })
        .send(ov51)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('Uploads odd version package52', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion5/1.7.3/${ov52release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov52.length)
        .query({ checksum: 'deb86745faaf90ab46113837694a0713072aad88d40f6a50f5be8fdf1ab5dd53' })
        .send(ov52)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('returns the latest release of package oddversion5', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion5/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion5');
          expect(res.body.ident.version).to.equal('1.7.3');
          expect(res.body.ident.release).to.equal(ov52release);
          done(err);
        });
    });

    it('returns the latest package oddversion5 in unstable channel', function (done) {
      request.get('/depot/channels/neurosis/unstable/pkgs/oddversion5/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion5');
          expect(res.body.ident.version).to.equal('1.7.3');
          expect(res.body.ident.release).to.equal(ov52release);
          done(err);
        });
    });

    it('lists all versions of the package oddversion5', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion5/versions')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(2);
          expect(res.body[0].origin).to.equal('neurosis');
          expect(res.body[0].name).to.equal('oddversion5');
          expect(res.body[0].version).to.equal('1.7.3');
          expect(res.body[0].latest).to.equal(ov52release);
          done(err);
        });
    });

    it('Uploads odd version package61', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion6/1.2.3/${ov61release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov61.length)
        .query({ checksum: 'ea53673f81dbe9d728549f1abce07907bacffa960ae7f24e09bac7eccc56ee87' })
        .send(ov61)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('Uploads odd version package62', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion6/1.2.3-beta12/${ov62release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov62.length)
        .query({ checksum: 'eebac606b3f29ad3baef3ed0eedaacb5e327e9e9dc386e28689b369fe931cbf6' })
        .send(ov62)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('Uploads odd version package63', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion6/1.2.3-beta2/${ov63release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov63.length)
        .query({ checksum: 'da61308ca48fdb80a6cb69a6c10b01d458b3676264105ba703614b53b900da44' })
        .send(ov63)
        .expect(201)
        .end(function (err, res) {
          done(err);
        });
    });

    it('returns the latest release of package oddversion6', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion6/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion6');
          expect(res.body.ident.version).to.equal('1.2.3');
          expect(res.body.ident.release).to.equal(ov61release);
          done(err);
        });
    });

    it('lists all versions of the package oddversion6', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion6/versions')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(3);
          expect(res.body[0].origin).to.equal('neurosis');
          expect(res.body[0].name).to.equal('oddversion6');
          expect(res.body[0].version).to.equal('1.2.3');
          expect(res.body[0].latest).to.equal(ov61release);
          done(err);
        });
    });


    it('Uploads odd version package71', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion7/17.1.0-dev.cloud/${ov71release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov71.length)
        .query({ checksum: '87afacf023109a88da1d70f64054d740d4b92685d8f8cbd622e278a653a5d1f3' })
        .send(ov71)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/oddversion7/17.1.0-dev.cloud/${ov71release}/download`);
          done(err);
        });
    });

    it('Uploads odd version package72', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion7/19.1.0.dev-cloud/${ov72release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov72.length)
        .query({ checksum: '355ab7faf84904ec7ba5ab3504a046ffdbcffa6112c2ac112191486e1df2bde6' })
        .send(ov72)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/oddversion7/19.1.0.dev-cloud/${ov72release}/download`);
          done(err);
        });
    });

    it('Uploads odd version package73', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion7/19.1.0./${ov73release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov73.length)
        .query({ checksum: '1edbc350992b30ecd095924d9ddb9e45c92df5af52cba672f85c7dea2ed44764' })
        .send(ov73)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/oddversion7/19.1.0./${ov73release}/download`);
          done(err);
        });
    });

    it('returns all versions of package oddversion7', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion7/versions')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(3);
          expect(res.body[2].origin).to.equal('neurosis');
          expect(res.body[2].name).to.equal('oddversion7');
          expect(res.body[2].version).to.equal('17.1.0-dev.cloud');
          expect(res.body[2].latest).to.equal(ov71release);
          done(err);
        });
    });

    it('returns the latest release of package oddversion7', function (done) {
      request.get('/depot/pkgs/neurosis/oddversion7/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion7');
          expect(res.body.ident.version).to.equal('19.1.0.dev-cloud');
          expect(res.body.ident.release).to.equal(ov72release);
          done(err);
        });
    });

    it('Uploads odd version package81', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion8/4/${ov81release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov81.length)
        .query({ checksum: 'cc2a46fd759a13e8b79564dd684b5322cd3fd106c228dc366291bdcdea983470' })
        .send(ov81)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/oddversion8/4/${ov81release}/download`);
          done(err);
        });
    });

    it('Uploads odd version package82', function (done) {
      request.post(`/depot/pkgs/neurosis/oddversion8/5/${ov82release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', ov82.length)
        .query({ checksum: '5d665be0605fc67d0713a76e271dff3dc5828389326d82189f653e690bc42822' })
        .send(ov82)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/oddversion8/5/${ov82release}/download`);
          done(err);
        });
    });

    it('returns the latest package oddversion8 in unstable channel', function (done) {
      request.get('/depot/channels/neurosis/unstable/pkgs/oddversion8/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('oddversion8');
          expect(res.body.ident.version).to.equal('5');
          expect(res.body.ident.release).to.equal(ov82release);
          done(err);
        });
    });
  });
});
