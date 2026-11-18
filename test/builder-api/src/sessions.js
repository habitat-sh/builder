const expect = require('chai').expect;
const supertest = require('supertest');
const request = supertest('http://localhost:9636/v1');

describe('Authenticate API', function() {
  let fixtures;

  before(function() {
    if (!process.env.BUILDER_OAUTH_IDENTITY_FIXTURES) {
      throw new Error('Run test.sh to seed the OAuth identity fixtures before running Mocha.');
    }
    fixtures = JSON.parse(process.env.BUILDER_OAUTH_IDENTITY_FIXTURES);
  });

  describe('Create sessions', function() {
    it('returns bobo', function(done) {
      request.get('/authenticate/bobo')
        .expect(200)
        .end(function(err, res) {
          expect(res.body.name).to.equal('bobo');
          expect(res.body.id).to.equal(fixtures.fixed_user);
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
    before(async function() {
      const inviter = await request.get('/authenticate/oauth-identity-inviter').expect(200);
      inviterBearer = `Bearer ${inviter.body.token}`;
    });

    let originCount = 0;
    let inviterBearer;

    // Fixture origins belong to a dedicated account so they do not change the
    // origin lists asserted by other suites.
    async function createOrigin(bearer = inviterBearer) {
      const response = await request.post('/depot/origins')
        .set('Authorization', bearer)
        .send({ name: `oauthidentity${Date.now()}${originCount++}` })
        .expect(201);
      return response.body;
    }

    it('binds a same-name legacy account on its first OAuth login', async function() {
      const origin = await createOrigin();
      const invitation = await request.post(`/depot/origins/${origin.name}/users/oauth-identity-legacy/invitations`)
        .set('Authorization', inviterBearer).expect(201);
      expect(invitation.body.account_id).to.equal(fixtures.legacy);

      const first = await request.get('/authenticate/oauth-identity-legacy-first-login').expect(200);
      const second = await request.get('/authenticate/oauth-identity-legacy-first-login').expect(200);
      expect(first.body.id).to.equal(fixtures.legacy);
      expect(second.body.id).to.equal(fixtures.legacy);
      expect(first.body.name).to.equal('oauth-identity-legacy');

      const bearer = `Bearer ${first.body.token}`;
      const profile = await request.get('/profile').set('Authorization', bearer).expect(200);
      expect(profile.body.email).to.equal('legacy@example.com');
      await request.put(`/depot/origins/${origin.name}/invitations/${invitation.body.id}`)
        .set('Authorization', bearer).expect(204);
      const origins = await request.get('/user/origins').set('Authorization', bearer).expect(200);
      expect(origins.body.map(value => value.name)).to.include(origin.name);
    });

    it('reuses an independently backfilled account on its first renamed login', async function() {
      const origin = await createOrigin();
      const invitation = await request.post(`/depot/origins/${origin.name}/users/oauth-identity-backfilled/invitations`)
        .set('Authorization', inviterBearer).expect(201);
      expect(invitation.body.account_id).to.equal(fixtures.backfilled);

      const login = await request.get('/authenticate/oauth-identity-backfilled').expect(200);
      expect(login.body.id).to.equal(fixtures.backfilled);
      expect(login.body.name).to.equal('oauth-identity-backfilled-renamed');
      const invitations = await request.get(`/depot/origins/${origin.name}/invitations`)
        .set('Authorization', inviterBearer).expect(200);
      expect(invitations.body.invitations[0].id).to.equal(invitation.body.id);
      expect(invitations.body.invitations[0].account_name).to.equal(login.body.name);
    });

    it('preserves origins, membership, and tokens on rename and isolates a reused name', async function() {
      const original = await request.get('/authenticate/oauth-identity-original').expect(200);
      const originalBearer = `Bearer ${original.body.token}`;
      const origin = await createOrigin(originalBearer);
      const membershipOrigin = await createOrigin();
      const invitation = await request.post(`/depot/origins/${membershipOrigin.name}/users/oauth-identity-original/invitations`)
        .set('Authorization', inviterBearer).expect(201);
      await request.put(`/depot/origins/${membershipOrigin.name}/invitations/${invitation.body.id}`)
        .set('Authorization', originalBearer).expect(204);
      const token = await request.post('/profile/access-tokens')
        .set('Authorization', originalBearer)
        .expect(200);
      const accessBearer = `Bearer ${token.body.token}`;
      await request.get('/profile').set('Authorization', accessBearer).expect(200);

      const reused = await request.get('/authenticate/oauth-identity-reused').expect(200);
      const reusedBearer = `Bearer ${reused.body.token}`;
      const reusedOrigins = await request.get('/user/origins')
        .set('Authorization', reusedBearer).expect(200);
      const reusedTokens = await request.get('/profile/access-tokens')
        .set('Authorization', reusedBearer).expect(200);
      await request.get(`/depot/origins/${origin.name}/users`)
        .set('Authorization', reusedBearer).expect(403);
      await request.get(`/depot/origins/${membershipOrigin.name}/users`)
        .set('Authorization', reusedBearer).expect(403);

      const displaced = await request.get('/profile').set('Authorization', accessBearer).expect(200);
      expect(displaced.body.id).to.equal(original.body.id);
      expect(displaced.body.name).to.match(/^__renamed_account_/);
      // The self-transfer guard reads session.name, unlike /profile's database lookup.
      for (const bearer of [originalBearer, accessBearer]) {
        await request.post(`/depot/origins/${origin.name}/transfer/${displaced.body.name}`)
          .set('Authorization', bearer).expect(422);
      }

      const renamed = await request.get('/authenticate/oauth-identity-renamed').expect(200);
      const renamedBearer = `Bearer ${renamed.body.token}`;
      const profile = await request.get('/profile')
        .set('Authorization', renamedBearer).expect(200);
      const tokenProfile = await request.get('/profile')
        .set('Authorization', accessBearer).expect(200);
      for (const bearer of [originalBearer, accessBearer]) {
        await request.post(`/depot/origins/${origin.name}/transfer/${renamed.body.name}`)
          .set('Authorization', bearer).expect(422);
      }
      const ownedOrigin = await request.get(`/depot/origins/${origin.name}`).expect(200);
      const origins = await request.get('/user/origins')
        .set('Authorization', renamedBearer).expect(200);
      const members = await request.get(`/depot/origins/${origin.name}/users`)
        .set('Authorization', renamedBearer).expect(200);
      await request.get(`/depot/origins/${membershipOrigin.name}/users/${renamed.body.name}/role`)
        .set('Authorization', inviterBearer).expect(200);
      const tokens = await request.get('/profile/access-tokens')
        .set('Authorization', renamedBearer).expect(200);

      expect(renamed.body.id).to.equal(original.body.id);
      expect(renamed.body.name).to.equal('oauth-identity-renamed');
      expect(profile.body.id).to.equal(original.body.id);
      expect(profile.body.name).to.equal('oauth-identity-renamed');
      expect(tokenProfile.body.id).to.equal(original.body.id);
      expect(tokenProfile.body.name).to.equal('oauth-identity-renamed');
      expect(origin.owner_id).to.equal(original.body.id);
      expect(ownedOrigin.body.owner_id).to.equal(original.body.id);
      expect(ownedOrigin.body.owner_account).to.equal('oauth-identity-renamed');
      expect(origins.body.map(value => value.name)).to.include(origin.name);
      expect(origins.body.map(value => value.name)).to.include(membershipOrigin.name);
      expect(members.body.members).to.include('oauth-identity-renamed');
      expect(members.body.members).to.not.include('oauth-identity-original');
      expect(tokens.body.tokens.map(value => value.id)).to.include(token.body.id);
      expect(reused.body.id).to.not.equal(original.body.id);
      expect(reused.body.name).to.equal('oauth-identity-original');
      expect(reusedOrigins.body.map(value => value.name)).to.not.include(origin.name);
      expect(reusedOrigins.body.map(value => value.name)).to.not.include(membershipOrigin.name);
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
      const origin = await createOrigin();
      const ownerInvite = await request.post(`/depot/origins/${origin.name}/users/${owner.body.name}/invitations`)
        .set('Authorization', inviterBearer).expect(201);
      const holderInvite = await request.post(`/depot/origins/${origin.name}/users/${holder.body.name}/invitations`)
        .set('Authorization', inviterBearer).expect(201);
      const renamed = await request.get('/authenticate/oauth-identity-collision-rename').expect(200);
      const ownerBearer = `Bearer ${owner.body.token}`;
      const holderBearer = `Bearer ${holder.body.token}`;
      const ownerAgain = await request.get('/profile').set('Authorization', ownerBearer).expect(200);
      const holderAgain = await request.get('/profile').set('Authorization', holderBearer).expect(200);
      const invitations = await request.get(`/depot/origins/${origin.name}/invitations`)
        .set('Authorization', inviterBearer).expect(200);

      expect(renamed.body.id).to.equal(owner.body.id);
      expect(renamed.body.name).to.equal('oauth-identity-collision-new');
      expect(ownerAgain.body.id).to.equal(owner.body.id);
      expect(ownerAgain.body.name).to.equal(renamed.body.name);
      expect(holderAgain.body.id).to.equal(holder.body.id);
      expect(holderAgain.body.name).to.match(/^__renamed_account_/);
      expect(invitations.body.invitations.find(value => value.id === ownerInvite.body.id).account_name)
        .to.equal(ownerAgain.body.name);
      expect(invitations.body.invitations.find(value => value.id === holderInvite.body.id).account_name)
        .to.equal(holderAgain.body.name);
      await request.put(`/depot/origins/${origin.name}/invitations/${holderInvite.body.id}`)
        .set('Authorization', ownerBearer).expect(403);
      await request.put(`/depot/origins/${origin.name}/invitations/${holderInvite.body.id}`)
        .set('Authorization', holderBearer).expect(204);
      await request.put(`/depot/origins/${origin.name}/invitations/${ownerInvite.body.id}`)
        .set('Authorization', ownerBearer).expect(204);
    });

    it('moves a conflicting legacy row without binding it to the renamed identity', async function() {
      const owner = await request.get('/authenticate/oauth-identity-collision-owner').expect(200);
      const origin = await createOrigin();
      const invitation = await request.post(`/depot/origins/${origin.name}/users/oauth-identity-collision-legacy/invitations`)
        .set('Authorization', inviterBearer).expect(201);
      expect(invitation.body.account_id).to.equal(fixtures.collision);

      const renamed = await request.get('/authenticate/oauth-identity-collision-legacy').expect(200);
      expect(renamed.body.id).to.equal(owner.body.id);
      expect(renamed.body.id).to.not.equal(fixtures.collision);
      expect(renamed.body.name).to.equal('oauth-identity-collision-legacy');
      const invitations = await request.get(`/depot/origins/${origin.name}/invitations`)
        .set('Authorization', inviterBearer).expect(200);
      expect(invitations.body.invitations[0].id).to.equal(invitation.body.id);
      expect(invitations.body.invitations[0].account_id).to.equal(fixtures.collision);
      expect(invitations.body.invitations[0].account_name).to.match(/^__renamed_account_/);
      await request.put(`/depot/origins/${origin.name}/invitations/${invitation.body.id}`)
        .set('Authorization', `Bearer ${renamed.body.token}`).expect(403);
    });
  });
});
