const expect = require('chai').expect;
const supertest = require('supertest');
const request = supertest('http://localhost:9636/v1');

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

    it('returns mystique', function(done) {
      request.get('/authenticate/mystique')
        .expect(200)
        .end(function(err, res) {
          expect(res.body.name).to.equal('mystique');
          global.sessionMystique = res.body;
          done(err);
        });
    });
    it('returns wesker', function(done) {
      request.get('/authenticate/wesker')
        .expect(200)
        .end(function(err, res) {
            expect(res.body.name).to.equal('wesker');
            global.sessionWesker = res.body;
            done(err);
        });
    });
    it('returns lkennedy', function(done) {
      request.get('/authenticate/lkennedy')
        .expect(200)
        .end(function(err, res) {
            expect(res.body.name).to.equal('lkennedy');
            global.sessionLkennedy = res.body;
            done(err);
        });
    });
  });

  describe('Provider account identity', function() {
    it('binds a same-name legacy account on its first OAuth login', async function() {
      const first = await request.get('/authenticate/oauth-identity-legacy-first-login').expect(200);
      const second = await request.get('/authenticate/oauth-identity-legacy-first-login').expect(200);
      expect(first.body.id).to.equal(second.body.id);
      expect(first.body.name).to.equal('oauth-identity-legacy');
    });

    it('preserves origins, membership, and tokens on rename and isolates a reused name', async function() {
      const originName = `oauthidentity${Date.now()}`;
      const originalBearer = 'Bearer oauth-identity-original';
      const renamedBearer = 'Bearer oauth-identity-renamed';
      const reusedBearer = 'Bearer oauth-identity-reused';
      const original = await request.get('/authenticate/oauth-identity-original').expect(200);
      const origin = await request.post('/depot/origins')
        .set('Authorization', originalBearer)
        .send({ name: originName })
        .expect(201);
      const token = await request.post('/profile/access-tokens')
        .set('Authorization', originalBearer)
        .expect(200);

      const reused = await request.get('/authenticate/oauth-identity-reused').expect(200);
      const reusedOrigins = await request.get('/user/origins')
        .set('Authorization', reusedBearer).expect(200);
      const reusedTokens = await request.get('/profile/access-tokens')
        .set('Authorization', reusedBearer).expect(200);
      await request.get(`/depot/origins/${originName}/users`)
        .set('Authorization', reusedBearer).expect(403);

      const renamed = await request.get('/authenticate/oauth-identity-renamed').expect(200);
      const profile = await request.get('/profile')
        .set('Authorization', renamedBearer).expect(200);
      const ownedOrigin = await request.get(`/depot/origins/${originName}`).expect(200);
      const origins = await request.get('/user/origins')
        .set('Authorization', renamedBearer).expect(200);
      const members = await request.get(`/depot/origins/${originName}/users`)
        .set('Authorization', renamedBearer).expect(200);
      const tokens = await request.get('/profile/access-tokens')
        .set('Authorization', renamedBearer).expect(200);

      expect(renamed.body.id).to.equal(original.body.id);
      expect(renamed.body.name).to.equal('oauth-identity-renamed');
      expect(profile.body.id).to.equal(original.body.id);
      expect(profile.body.name).to.equal('oauth-identity-renamed');
      expect(origin.body.owner_id).to.equal(original.body.id);
      expect(ownedOrigin.body.owner_id).to.equal(original.body.id);
      expect(ownedOrigin.body.owner_account).to.equal('oauth-identity-renamed');
      expect(origins.body.map(origin => origin.name)).to.include(originName);
      expect(members.body.members).to.include('oauth-identity-renamed');
      expect(members.body.members).to.not.include('oauth-identity-original');
      expect(tokens.body.tokens.map(value => value.id)).to.include(token.body.id);
      expect(reused.body.id).to.not.equal(original.body.id);
      expect(reused.body.name).to.equal('oauth-identity-original');
      expect(reusedOrigins.body.map(origin => origin.name)).to.not.include(originName);
      expect(reusedTokens.body.tokens.map(value => value.id)).to.not.include(token.body.id);
    });

    it('keeps equal subject IDs from different providers separate', async function() {
      const github = await request.get('/authenticate/oauth-identity-renamed').expect(200);
      const gitlab = await request.get('/authenticate/oauth-identity-gitlab').expect(200);
      const okta = await request.get('/authenticate/oauth-identity-okta').expect(200);

      expect(gitlab.body.id).to.not.equal(github.body.id);
      expect(okta.body.id).to.not.equal(github.body.id);
      expect(okta.body.id).to.not.equal(gitlab.body.id);
    });

    it('moves a conflicting bound account name without swapping identities', async function() {
      const owner = await request.get('/authenticate/oauth-identity-collision-owner').expect(200);
      const holder = await request.get('/authenticate/oauth-identity-collision-holder').expect(200);
      const renamed = await request.get('/authenticate/oauth-identity-collision-rename').expect(200);
      const ownerAgain = await request.get('/authenticate/oauth-identity-collision-owner').expect(200);
      const holderAgain = await request.get('/authenticate/oauth-identity-collision-holder').expect(200);

      expect(renamed.body.id).to.equal(owner.body.id);
      expect(renamed.body.name).to.equal('oauth-identity-collision-new');
      expect(ownerAgain.body.id).to.equal(owner.body.id);
      expect(holderAgain.body.id).to.equal(holder.body.id);
    });
  });
});
