const expect = require('chai').expect;
const supertest = require('supertest');
const request = supertest('http://localhost:9636/v1');

describe('Profile API', function () {
  describe('Updating the profile', function () {
    it('requires authentication', function (done) {
      request.patch('/profile')
        .type('application/json')
        .accept('application/json')
        .send({ email: 'haha@example.com' })
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('allows someone to update their profile', function (done) {
      request.patch('/profile')
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .send({ email: 'haha@example.com' })
        .expect(200)
        .end(function (err, res) {
          // JB TODO: this is a sub-optimal UX here. we should return the
          // updated profile.
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  describe('Retrieving a profile', function () {
    it('requires authentication', function (done) {
      request.get('/profile')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('allows someone to retrieve their profile', function (done) {
      request.get('/profile')
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.email).to.equal('haha@example.com');
          done(err);
        });
    });
  });

  describe('Updating a license', function () {
    // Use the same license fixture as the package download tests.
    const validLicenseKey = 'free-6cc49568-d1e8-4165-b1a9-6b232b914ca6-7839';
    const otherValidLicenseKey = 'free-4f4eca5f-55cb-4072-99c6-83c0c060168b-4029';
    let accountId;

    before(async function () {
      const profile = await request.get('/profile')
        .set('Authorization', global.boboBearer)
        .expect(200);
      accountId = profile.body.id;
    });

    it('requires authentication even when an account ID is supplied', function () {
      return request.put('/profile/license')
        .send({ account_id: accountId, license_key: validLicenseKey })
        .expect(401);
    });

    it('saves a license for the authenticated account without an account ID', async function () {
      const saved = await request.put('/profile/license')
        .set('Authorization', global.boboBearer)
        .send({ license_key: validLicenseKey })
        .expect(200);
      const license = await request.get('/profile/license')
        .set('Authorization', global.boboBearer)
        .expect(200);
      expect(license.body.license_key).to.equal(validLicenseKey);
      expect(license.body.expiration_date).to.equal(saved.body.expiration_date);
    });

    it('accepts the authenticated account ID from older clients', function () {
      return request.put('/profile/license')
        .set('Authorization', global.boboBearer)
        .send({ account_id: accountId, license_key: validLicenseKey })
        .expect(200);
    });

    async function rejectOtherAccountUpdate(bearer, sharesOrigin) {
      // Verify the relationship established by the origin/invitation fixtures.
      const callerOrigins = await request.get('/user/origins')
        .set('Authorization', bearer)
        .expect(200);
      const victimOrigins = await request.get('/user/origins')
        .set('Authorization', global.boboBearer)
        .expect(200);
      expect(callerOrigins.body).to.not.be.empty;
      expect(victimOrigins.body).to.not.be.empty;
      const shared = callerOrigins.body.some(caller =>
        victimOrigins.body.some(victim => victim.name === caller.name));
      expect(shared).to.equal(sharesOrigin);

      // Seed the victim with key A before attempting an overwrite with key B.
      await request.put('/profile/license')
        .set('Authorization', global.boboBearer)
        .send({ license_key: validLicenseKey })
        .expect(200);
      const before = await request.get('/profile/license')
        .set('Authorization', global.boboBearer)
        .expect(200);
      expect(before.body.license_key).to.equal(validLicenseKey);

      // Both a different valid key and an invalid key must fail authorization.
      for (const key of [otherValidLicenseKey, 'nonexistent-license-for-authorization-test']) {
        await request.put('/profile/license')
          .set('Authorization', bearer)
          .send({ account_id: accountId, license_key: key })
          .expect(403);
        const after = await request.get('/profile/license')
          .set('Authorization', global.boboBearer)
          .expect(200);
        expect(after.body).to.deep.equal(before.body);
      }
    }

    it('rejects another account in the same origin and preserves its license', function () {
      return rejectOtherAccountUpdate(global.weskerBearer, true);
    });

    it('rejects another account in a different origin and preserves its license', function () {
      return rejectOtherAccountUpdate(global.mystiqueBearer, false);
    });

    it('rejects malformed account IDs before validating the license', async function () {
      for (const accountId of ['', 'not-an-account', '9223372036854775808']) {
        await request.put('/profile/license')
          .set('Authorization', global.boboBearer)
          .send({ account_id: accountId, license_key: validLicenseKey })
          .expect(400);
      }
    });

    it('rejects an invalid license for the caller and preserves the existing license', async function () {
      const before = await request.get('/profile/license')
        .set('Authorization', global.boboBearer)
        .expect(200);
      await request.put('/profile/license')
        .set('Authorization', global.boboBearer)
        .send({ license_key: 'nonexistent-license-for-authorization-test' })
        .expect(400);
      const after = await request.get('/profile/license')
        .set('Authorization', global.boboBearer)
        .expect(200);
      expect(after.body).to.deep.equal(before.body);
    });
  });

  describe('Generating a personal access token', function () {
    it('requires authentication', function (done) {
      request.post('/profile/access-tokens')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  it('succeeds', function (done) {
    request.post('/profile/access-tokens')
      .set('Authorization', global.boboBearer)
      .type('application/json')
      .accept('application/json')
      .expect(200)
      .end(function (err, res) {
        expect(res.body.token).to.not.be.empty;
        global.boboTokenId = res.body.id;
        done(err);
      });
  });

  describe('Getting a list of access tokens', function () {
    it('requires authentication', function (done) {
      request.get('/profile/access-tokens')
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  it('succeeds', function (done) {
    request.get('/profile/access-tokens')
      .set('Authorization', global.boboBearer)
      .type('application/json')
      .accept('application/json')
      .expect(200)
      .end(function (err, res) {
        expect(res.body.tokens).to.not.be.empty;
        expect(res.body.tokens[0].id).to.equal(global.boboTokenId);
        done(err);
      });
  });

  describe('Revoking an access token', function () {
    it('requires authentication', function (done) {
      request.delete('/profile/access-tokens/' + global.boboTokenId)
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  it('succeeds', function (done) {
    request.delete('/profile/access-tokens/' + global.boboTokenId)
      .set('Authorization', global.boboBearer)
      .type('application/json')
      .accept('application/json')
      .expect(200)
      .end(function (err, res) {
        expect(res.body).to.be.empty;
        done(err);
      });
  });

  describe('Profile API - Cross-User Token Deletion', function () {
    let userATokenId = null;
    let userBTokenId = null;
  
    before(function (done) {
      request.post('/profile/access-tokens')
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          if (err) return done(err);
          expect(res.body.token).to.not.be.empty;
          userATokenId = res.body.id;
          done();
        });
    });
  
    before(function (done) {
      request.post('/profile/access-tokens')
        .set('Authorization', global.mystiqueBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          if (err) return done(err);
          expect(res.body.token).to.not.be.empty;
          userBTokenId = res.body.id;
          done();
        });
    });
  
    it('should prevent USER B from deleting USER A\'s token', function (done) {
      request.delete('/profile/access-tokens/' + userATokenId)
        .set('Authorization', global.mystiqueBearer)
        .type('application/json')
        .accept('application/json')
        .expect(401)
        .end(function (err, res) {
          if (err) return done(err);
          expect(res.text).to.equal('Unauthorized access.');
          done();
        });
    });
  
    after(function (done) {
      request.delete('/profile/access-tokens/' + userATokenId)
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          if (err) return done(err);
          expect(res.body).to.be.empty;
          done();
        });
    });
  });
});
