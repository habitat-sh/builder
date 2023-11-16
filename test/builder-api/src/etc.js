const expect = require("chai").expect;
const supertest = require("supertest");
const request = supertest("http://localhost:9636/v1");
const fs = require("fs");

const release = '20231113160958';
const release1 = '20231116041041';

const file = fs.readFileSync(__dirname + `/../fixtures/neurosis-winapp-0.1.0-${release}-x86_64-windows.hart`);
const file1 = fs.readFileSync(__dirname + `/../fixtures/neurosis-winapp-0.1.0-${release1}-x86_64-windows.hart`);

describe("Additional APIs", function () {
  describe("Package uploads", function () {
    it('uploads a windows only package', function (done) {
      request.post(`/depot/pkgs/neurosis/winapp/0.1.0/${release}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file.length)
        .query({ checksum: 'b4dad6c7ee397919b0cfcbb85bed9d047b0a86c5e4ece6cc4ef651528dbabb85' })
        .send(file)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/winapp/0.1.0/${release}/download`);
          done(err);
        });
    });

    it('toggles the public setting for a package', function (done) {
      request.patch(`/depot/pkgs/neurosis/winapp/0.1.0/${release}/public`)
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('puts the winapp package into the bar channel', function (done) {
      request.put(`/depot/channels/neurosis/bar/pkgs/winapp/0.1.0/${release}/promote?target=x86_64-windows`)
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  
    it('puts the winapp package into the stable channel', function (done) {
      request.put(`/depot/channels/neurosis/stable/pkgs/winapp/0.1.0/${release}/promote?target=x86_64-windows`)
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('uploads a latest package', function (done) {
      request.post(`/depot/pkgs/neurosis/winapp/0.1.0/${release1}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file1.length)
        .query({ checksum: '148ea3fc9b5818e76ab84150a3ba9e630ec608c591472b969ba32ca1b65dc136' })
        .send(file1)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/neurosis/winapp/0.1.0/${release1}/download`);
          done(err);
        });
    });

    it('toggles the public setting for new release', function (done) {
      request.patch(`/depot/pkgs/neurosis/winapp/0.1.0/${release1}/public`)
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('puts the new relase into the stable channel', function (done) {
      request.put(`/depot/channels/neurosis/stable/pkgs/winapp/0.1.0/${release1}/promote?target=x86_64-windows`)
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });
});
