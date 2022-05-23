const expect = require("chai").expect;
const supertest = require("supertest");
const request = supertest("http://localhost:9636/v1");
const fs = require("fs");
require("dotenv").config()
const origin_name = "neurosis";
const release1 = '20171205003213';
const file1 = fs.readFileSync(__dirname + `/../fixtures/${origin_name}-testapp-0.1.3-${release1}-x86_64-linux.hart`);

const revision = '20171211220037';
const pubFile = fs.readFileSync(__dirname + `/../fixtures/${origin_name}-${revision}.pub`, 'utf8');
const secretFile = fs.readFileSync(__dirname + `/../fixtures/${origin_name}-${revision}.sig.key`, 'utf8');

// These magic values correspond to the testapp repo in the habitat-sh org
const installationId = 79953;
const repoId = 114932712;
const projectCreatePayload = {
  origin: 'neurosis',
  plan_path: 'plan.sh',
  installation_id: installationId,
  repo_id: repoId,
  auto_build: true
};

//works!
//console.log("Checking environment - token " + process.env.HAB_AUTH_TOKEN);
const authBearer = `Bearer ${process.env.HAB_AUTH_TOKEN}`;

describe("Startup/Origin Creation", function () {
  it("returns the created origin", function (done) {
    request
      .post("/depot/origins")
      .set("Authorization", authBearer)
      .send({ name: `${origin_name}`, default_package_visibility: "public" })
      .expect(201)
      .end(function (err, res) {
	console.log(res);
        expect(res.body.name).to.equal(`${origin_name}`);
        expect(res.body.default_package_visibility).to.equal("public");
        global.originWorker = res.body;
        done(err);
      });
    });
});

/*
describe('Authenticate API', function() {
  describe('Create sessions', function() {
    it('returns bobo', function(done) {
      request.get('/authenticate/bobo')
        .expect(200)
        .end(function(err, res) {
          expect(res.body.name).to.equal('bobo');
          global.sessionBobo = res.body;
          done(err);
        });
    });
  });
});

describe('Settings API', function() {
  describe('Create package settings', function() {
    it("returns the created package settings", function (done) {
      request
        .post(`/settings/${origin_name}/testapp`)
        .set("Authorization", global.boboBearer)
        .expect(201)
        .end(function (err, res) {
          expect(res.body.origin).to.equal(`${origin_name}`);
          expect(res.body.name).to.equal("testapp");
          expect(res.body.visibility).to.equal("public");
          expect(res.body.owner_id).to.equal(global.sessionBobo.id);
          done(err);
        });
    });
  });
});

describe('Create and promote a package', function () {
    it('allows authenticated users to upload packages', function (done) {
      request.post(`/depot/pkgs/${origin_name}/testapp/0.1.3/${release1}`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', file1.length)
        .query({ checksum: '3138777020e7bb621a510b19c2f2630deee9b34ac11f1c2a0524a44eb977e4a8' })
        .send(file1)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/pkgs/${origin_name}/testapp/0.1.3/${release1}/download`);
          done(err);
        });
    });

    it('promotes package to stable channel', function (done) {
      request.put(`/depot/channels/${origin_name}/stable/pkgs/testapp/0.1.3/20171205003213/promote`)
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
});

describe('Keys API', function () {
    it('uploads the public key', function (done) {
      request.post(`/depot/origins/${origin_name}/keys/${revision}`)
        .set('Authorization', global.boboBearer)
        .send(pubFile)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.equal(`/origins/${origin_name}/keys/${revision}`);
          expect(res.header['location']).to.equal(`/v1/depot/origins/${origin_name}/keys/${revision}`);
          done(err);
        });
    });

    it('uploads the private key', function (done) {
      request.post(`/depot/origins/${origin_name}/secret_keys/${revision}`)
        .set('Authorization', global.boboBearer)
        .send(secretFile)
        .expect(201)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('can download the latest key', function (done) {
      request.get(`/depot/origins/${origin_name}/keys/latest`)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.equal(pubFile);
          done(err);
        });
    });

    it('retrieves the secret key for origin', function (done) {
      request.get(`/depot/origins/${origin_name}`)
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.body.private_key_name).to.equal(`${origin_name}-${revision}`);
          done(err);
        });
    });

});

describe('Projects API', function () {
  describe('Creating a project', function () {
    beforeEach('SLEEP 5s', async function() {
      console.log("Pause 5s");
      await new Promise(resolve => setTimeout(resolve, 5000));
    });
    it('succeeds', function (done) {
      request.post('/projects')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .set("Referer", "http://localhost:9636")
        .send(projectCreatePayload)
        .expect(201)
        .end(function (err, res) {
          expect(res.body.name).to.equal('neurosis/testapp');
          done(err);
        });
    });
  });
});

describe("Jobs API", function () {
  describe("Scheduling jobs", function () {
    var found = false;
    var jobState = "";
    it("Schedules job and returns group", function (done) {
      request
        .post(`/depot/pkgs/schedule/${origin_name}/testapp`)
        .type("application/json")
        .accept("application/json")
        .set("Authorization", global.boboBearer)
        .set("Referer", "http://localhost:9636")
        .expect(201)
        .end(function (err, res) {
          expect(res.body).to.not.be.empty;
          expect(res.body.state).to.equal("Queued");
          expect(res.body.project_name).to.equal(`${origin_name}/testapp`);
          global.neurosisJobGroup = res.body;
	  console.log("Echo group id: " + `${global.neurosisJobGroup.id}`);
	  console.log("Echo next request: " + `/depot/pkgs/schedule/${global.neurosisJobGroup.id}`);
          done(err);
        });
    });
  });
});

describe("Jobsrv and Worker - Dispatching", function() {
    beforeEach('SLEEP 5s', async function() {
      console.log("Pause 5s");
      await new Promise(resolve => setTimeout(resolve, 5000));
    });
    it("Waiting for the job group to get dispatched", function (done) {
      console.log("Echo next request: " + `/depot/pkgs/schedule/${global.neurosisJobGroup.id}`);
      request
        .get(`/depot/pkgs/schedule/${global.neurosisJobGroup.id}`)
        .type("application/json")
        .accept("application/json")
        .expect(200)
        .end(function (err, res) {
           expect(res.body.state).to.equal("Dispatching");
           global.neurosisJobGroup = res.body;
	   done(err);
    });
  });
});

describe("Jobsrv and Worker - Complete", function() {
    beforeEach('SLEEP 90s', async function() {
      console.log("Pause 90s");
      await new Promise(resolve => setTimeout(resolve, 90000));
    });
    it("Waiting for the job group to get marked Complete", function (done) {
      console.log("Next request: " + `/depot/pkgs/schedule/${global.neurosisJobGroup.id}`);
      request
        .get(`/depot/pkgs/schedule/${global.neurosisJobGroup.id}`)
        .type("application/json")
        .accept("application/json")
        .expect(200)
        .end(function (err, res) {
           expect(res.body.state).to.equal("Complete");
           global.neurosisJobGroup = res.body;
	   done(err);
    });
  });
});

*/
