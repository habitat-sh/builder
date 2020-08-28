const expect = require("chai").expect;
const supertest = require("supertest");
const request = supertest("http://localhost:9636/v1");

let projectExpectations = function (res) {
  expect(res.body.origin).to.equal("neurosis");
  expect(res.body.name).to.equal("testapp");
  expect(res.body.visibility).to.equal("public");
  expect(res.body.owner_id).to.equal(global.sessionBobo.id);
};

describe("Settings API", function () {
  describe("Creating a package entry", function () {
    it("requires authentication", function (done) {
      request
        .post("/settings/neurosis/testapp")
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it("returns the created package settings", function (done) {
      request
        .post("/settings/neurosis/testapp")
        .set("Authorization", global.boboBearer)
        .expect(201)
        .end(function (err, res) {
          projectExpectations(res);
          done(err);
        });
    });
  });
  describe("Duplicate settings entry", function () {
    it("returns conflict on settings create", function (done) {
      request
        .post("/settings/neurosis/testapp")
        .set("Authorization", global.boboBearer)
        .expect(409)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });
  describe("Create pkg settings for wittr", function () {
    it("succeeds", function (done) {
      request
        .post("/settings/neurosis/wittr")
        .set("Authorization", global.boboBearer)
        .expect(201)
        .end(function (err, res) {
          expect(res.body.origin).to.equal("neurosis");
          expect(res.body.name).to.equal("wittr");
          expect(res.body.owner_id).to.equal(global.sessionBobo.id);
          done(err);
        });
    });
  });
  describe("Updating Package Settings", function () {
    it("requires authentication to update settings", function (done) {
      request
        .put("/settings/neurosis/wittr")
        .send({ visibility: "private" })
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it("requires origin membership to update settings", function (done) {
      request
        .put("/settings/neurosis/wittr")
        .send({ visibility: "private" })
        .set("Authorization", global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it("succeeds", function (done) {
      request
        .put("/settings/neurosis/wittr")
        .send({ visibility: "private" })
        .set("Authorization", global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.body.visibility).to.equal("private");
          done(err);
        });
    });
    it("reflects the changes when viewed again", function (done) {
      request
        .get("/settings/neurosis/wittr")
        .set("Authorization", global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.body.visibility).to.equal("private");
          done(err);
        });
    });
  });
  describe("Getting Package Settings", function () {
    it("requires authentication to view settings", function (done) {
      request
        .get("/settings/neurosis/wittr")
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it("requires origin membership to view settings", function (done) {
      request
        .get("/settings/neurosis/wittr")
        .set("Authorization", global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it("succeeds", function (done) {
      request
        .get("/settings/neurosis/testapp")
        .set("Authorization", global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.body.origin).to.equal("neurosis");
          expect(res.body.name).to.equal("testapp");
          expect(res.body.visibility).to.equal("public");
          expect(res.body.owner_id).to.equal(global.sessionBobo.id);
          done(err);
        });
    });
  });
  describe("Deleting Package Settings", function () {
    it("requires authentication to delete settings", function (done) {
      request
        .delete("/settings/neurosis/wittr")
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it("requires origin membership to delete settings", function (done) {
      request
        .delete("/settings/neurosis/wittr")
        .set("Authorization", global.mystiqueBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it("succeeds", function (done) {
      request
        .delete("/settings/neurosis/testapp")
        .set("Authorization", global.boboBearer)
        .expect(204)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
    it("reflects the changes when viewed again", function (done) {
      request
        .get("/settings/neurosis/testapp")
        .set("Authorization", global.boboBearer)
        .expect(404)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });
});
