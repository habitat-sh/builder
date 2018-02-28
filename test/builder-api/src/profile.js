const expect = require('chai').expect;
const supertest = require('supertest');
const request = supertest('http://localhost:9636/v1');

describe('Profile API', function() {
  describe('Updating the profile', function() {
    it('requires authentication', function(done) {
      request.patch('/profile')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('allows someone to update their profile', function(done) {
      request.patch('/profile')
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .send({email: 'haha@example.com'})
        .expect(200)
        .end(function(err, res) {
          // JB TODO: this is a sub-optimal UX here. we should return the
          // updated profile.
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  describe('Retrieving a profile', function() {
    it('requires authentication', function(done) {
      request.get('/profile')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('allows someone to retrieve their profile', function(done) {
      request.get('/profile')
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function(err, res) {
          expect(res.body.email).to.equal('haha@example.com');
          done(err);
        });
    });
  });

  describe('Operations on access token', function() {
    it('requires authentication on retrieval', function(done) {
      request.get('/profile/access-tokens')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires authentication on generation', function(done) {
      request.post('/profile/access-tokens')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires authentication on deletion', function(done) {
      request.delete('/profile/access-tokens/123')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function(err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('returns empty array when there are no access tokens', function(done) {
      request.get('/profile/access-tokens')
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function(err, res) {
          expect(res.body.tokens).to.be.empty;
          done(err);
        });
    });

    it('allows generation of a new access token', function(done) {
      request.post(`/profile/access-tokens`)
        .set('Authorization', global.boboBearer)
        .set('Content-Length', 0)
        .expect(200)
        .end(function(err, res) {
          console.log(res.text);
          expect(res.body.id).not.to.be.empty
          global.accessTokenId = res.body.id;
          expect(res.body.account_id).not.to.be.empty
          expect(res.body.token).not.to.be.empty
          expect(res.body.created_at).not.to.be.empty
          done(err);
        });
    });

    it('returns a valid access token when it exists', function(done) {
      request.get('/profile/access-tokens')
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function(err, res) {
          console.log(res.text);
          expect(res.body.tokens).to.not.be.empty;
          expect(res.body.tokens.length).to.equal(1);
          expect(res.body.tokens[0].id).not.to.be.empty;
          expect(res.body.tokens[0].id).to.equal(global.accessTokenId);
          expect(res.body.tokens[0].account_id).not.to.be.empty;
          expect(res.body.tokens[0].token).not.to.be.empty;
          expect(res.body.tokens[0].created_at).not.to.be.empty;
          done(err);
        });
    });

    it('allows deletion of an existing access token', function(done) {
      request.delete('/profile/access-tokens/' + global.accessTokenId)
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function(err, res) {
          expect(res.body).to.be.empty;
          done(err);
        });
    });
  });
});
