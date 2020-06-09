const expect = require('chai').expect;
const supertest = require('supertest');
const request = supertest('http://localhost:9636/v1');

describe('Channels API', function () {
  describe('Create foo channel', function () {
    it('requires authentication to create a channel', function (done) {
      request.post('/depot/channels/neurosis/foo')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('returns the created channel', function (done) {
      request.post('/depot/channels/neurosis/foo')
        .set('Authorization', global.boboBearer)
        .expect(201)
        .end(function (err, res) {
          expect(res.body.name).to.equal('foo');
          expect(res.body.owner_id).to.equal(global.sessionBobo.id);
          global.channelFoo = res.body;
          done(err);
        });
    });
  });

  describe('Duplicate channel', function () {
    it('returns conflict on channel create', function (done) {
      request.post('/depot/channels/neurosis/foo')
        .set('Authorization', global.boboBearer)
        .expect(409)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  describe('Create bar channel', function () {
    it('succeeds', function (done) {
      request.post('/depot/channels/neurosis/bar')
        .set('Authorization', global.boboBearer)
        .expect(201)
        .end(function (err, res) {
          expect(res.body.name).to.equal('bar');
          expect(res.body.owner_id).to.equal(global.sessionBobo.id);
          global.channelBar = res.body;
          done(err);
        });
    });
  });

  describe('Channel promotion', function () {
    it('requires authentication to promote a package', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/testapp/0.1.3/20171205003213/promote')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires origin membership to promote a package', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/testapp/0.1.3/20171205003213/promote')
        .set('Authorization', global.mystiqueBearer)
        .expect(401)
        .end(function (err, res) {
          done(err);
        });
    });

    it('puts the specified package into the specified channel', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/testapp/0.1.3/20171205003213/promote')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('should ignore packages promoted to a channel where the package already exists', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/testapp/0.1.3/20171205003213/promote')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('can promote private packages', function (done) {
      request.put('/depot/channels/neurosis/bar/pkgs/testapp/0.1.3/20171206004121/promote')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  describe('Listing packages in a channel', function () {
    it('returns all packages in a channel', function (done) {
      request.get('/depot/channels/neurosis/foo/pkgs')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(0);
          expect(res.body.total_count).to.equal(1);
          expect(res.body.data[0].name).to.equal('testapp');
          expect(res.body.data[0].version).to.equal('0.1.3');
          expect(res.body.data[0].release).to.equal('20171205003213');
          done(err);
        });
    });

    it('returns all packages with the given name in a channel', function (done) {
      request.get('/depot/channels/neurosis/foo/pkgs/testapp')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(0);
          expect(res.body.total_count).to.equal(1);
          expect(res.body.data[0].name).to.equal('testapp');
          expect(res.body.data[0].version).to.equal('0.1.3');
          expect(res.body.data[0].release).to.equal('20171205003213');
          done(err);
        });
    });

    it('returns all packages with the given name and version in a channel', function (done) {
      request.get('/depot/channels/neurosis/foo/pkgs/testapp/0.1.3')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(0);
          expect(res.body.total_count).to.equal(1);
          expect(res.body.data[0].name).to.equal('testapp');
          expect(res.body.data[0].version).to.equal('0.1.3');
          expect(res.body.data[0].release).to.equal('20171205003213');
          done(err);
        });
    });

    it('returns no packages with the given name and version swapped in a channel', function (done) {
      request.get('/depot/channels/neurosis/foo/pkgs/0.1.3/testapp')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(0);
          expect(res.body.total_count).to.equal(0);
          done(err);
        });
    });

    it('returns all packages with the specified name and version', function (done) {
      request.get('/depot/pkgs/neurosis/testapp/0.1.3')
        .set('Authorization', global.boboBearer)
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
          expect(res.body.data[0].release).to.equal('20181116180420');
          expect(res.body.data[1].platforms[0]).to.equal('x86_64-linux');
          expect(res.body.data[1].origin).to.equal('neurosis');
          expect(res.body.data[1].name).to.equal('testapp');
          expect(res.body.data[1].version).to.equal('0.1.3');
          expect(res.body.data[1].release).to.equal('20171206004121');
          expect(res.body.data[1].platforms[0]).to.equal('x86_64-linux');
          done(err);
        });
    });


    it('returns no packages with the specified name and version swapped', function (done) {
      request.get('/depot/pkgs/neurosis/0.1.3/testapp')
        .set('Authorization', global.boboBearer)
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(0);
          expect(res.body.total_count).to.equal(0);
          expect(res.body.data.length).to.equal(0);
          done(err);
        });
    });

    it('returns the package with the given name, version and release in a channel', function (done) {
      request.get('/depot/channels/neurosis/foo/pkgs/testapp/0.1.3/20171205003213')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('testapp');
          expect(res.body.ident.version).to.equal('0.1.3');
          expect(res.body.ident.release).to.equal('20171205003213');
          done(err);
        });
    });

    it('returns no package with the given release, name, version (swapped) channel', function (done) {
      request.get('/depot/channels/neurosis/foo/pkgs/20171205003213/testapp/0.1.3')
        .type('application/json')
        .accept('application/json')
        .expect(404)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('returns no package with the given name, release, version (swapped) channel', function (done) {
      this.skip(); // Fails until we do the right thing with contains ident array
      request.get('/depot/channels/neurosis/foo/pkgs/testapp/20171205003213/0.1.3')
        .type('application/json')
        .accept('application/json')
        .expect(404)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('returns the latest package with the given name in a channel', function (done) {
      request.get('/depot/channels/neurosis/foo/pkgs/testapp/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('testapp');
          expect(res.body.ident.version).to.equal('0.1.3');
          expect(res.body.ident.release).to.equal('20171205003213');
          done(err);
        });
    });

    it('returns the latest package with the given name and version in a channel', function (done) {
      request.get('/depot/channels/neurosis/foo/pkgs/testapp/0.1.3/latest')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('testapp');
          expect(res.body.ident.version).to.equal('0.1.3');
          expect(res.body.ident.release).to.equal('20171205003213');
          done(err);
        });
    });

    it('requires authentication to view private packages in a channel', function (done) {
      request.get('/depot/channels/neurosis/bar/pkgs/testapp/0.1.3/latest')
        .type('application/json')
        .accept('application/json')
        .expect(404)
        .end(function (err, res) {
          done(err);
        });
    });

    it('does not let members of other origins view private packages in a channel', function (done) {
      request.get('/depot/channels/neurosis/bar/pkgs/testapp/0.1.3/latest')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.mystiqueBearer)
        .expect(404)
        .end(function (err, res) {
          done(err);
        });
    });

    it('allows members of the origin to view private packages when they are authenticated', function (done) {
      request.get('/depot/channels/neurosis/bar/pkgs/testapp/0.1.3/latest')
        .type('application/json')
        .accept('application/json')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.body.ident.origin).to.equal('neurosis');
          expect(res.body.ident.name).to.equal('testapp');
          expect(res.body.ident.version).to.equal('0.1.3');
          expect(res.body.ident.release).to.equal('20171206004121');
          done(err);
        });
    });
  });

  describe('Latest packages in an origin', function () {

    it('returns latest packages in a channel fails without a target', function (done) {
      request.get('/depot/channels/neurosis/foo/pkgs/latest')
        .type('application/json')
        .accept('application/json')
        .expect(400)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('returns latest packages in a channel', function (done) {
      request.get('/depot/channels/neurosis/foo/pkgs/latest?target=x86_64-linux')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.channel).to.equal('foo');
          expect(res.body.target).to.equal('x86_64-linux');
          expect(res.body.data.length).to.equal(1);
          expect(res.body.data[0].name).to.equal('testapp');
          expect(res.body.data[0].version).to.equal('0.1.3');
          expect(res.body.data[0].release).to.equal('20171205003213');
          done(err);
        });
    });

    describe('with multiple versions of package in channel', function () {

      // What I really want to do is have a separate channel for the tests, with promotion/demotion and isolation from the other tests
      // But for the the life of me I can't get the before/after clauses to work.
      // They execute at random times, often the before clause runs *AFTER* the test, and I have no clue why.
      //
      // So for now, we test against existing channels, as fragile as that might be
      //

      // Below is one variant, but
      // before(function () {
      //   request.post('/depot/channels/neurosis/baz')
      //     .set('Authorization', global.boboBearer)
      //     .expect(201)
      //     .end();
      //   request.put('/depot/channels/neurosis/baz/pkgs/testapp/0.1.3/20171205003213/promote')
      //     .set('Authorization', global.boboBearer)
      //     .expect(200)
      //     .end();
      //   request.put('/depot/channels/neurosis/baz/pkgs/neurosis/testapp/0.1.4/20171206004139/promote')
      //     .set('Authorization', global.boboBearer)
      //     .expect(200)
      //     .end();
      //   request.put('/depot/channels/neurosis/baz/pkgs/neurosis/neurosis/testapp/0.1.4/20181115124506/promote')
      //     .set('Authorization', global.boboBearer)
      //     .expect(200)
      //     .end();
      //   request.put('/depot/channels/neurosis/baz/pkgs/testapp2/v1.2.3-aaster/20181018162220/promote')
      //     .set('Authorization', global.boboBearer)
      //     .expect(200)
      //     .end();
      // });

      // after(function () {
      //   request.delete('/depot/channels/neurosis/baz')
      //     .set('Authorization', global.boboBearer)
      //     .expect(200)
      //     .end();
      // })

      it('returns latest packages in a channel', function (done) {
        request.get('/depot/channels/neurosis/foo/pkgs/latest?target=x86_64-linux')
          .type('application/json')
          .accept('application/json')
          .expect(200)
          .end(function (err, res) {
            expect(res.body.channel).to.equal('foo');
            expect(res.body.target).to.equal('x86_64-linux');
            expect(res.body.data.length).to.equal(1);
            expect(res.body.data[0].name).to.equal('testapp');
            expect(res.body.data[0].version).to.equal('0.1.3');
            expect(res.body.data[0].release).to.equal('20171205003213');
            done(err);
          });
        });

      it('returns latest packages in a channel, but not the private ones', function (done) {
        request.get('/depot/channels/neurosis/baz/pkgs/latest?target=x86_64-linux')
           .type('application/json')
           .accept('application/json')
           .expect(200)
           .end(function (err, res) {
             expect(res.body.channel).to.equal('baz');
             expect(res.body.target).to.equal('x86_64-linux');
             expect(res.body.data.length).to.equal(0);
             done(err);
           });
      });

      // Wishful verify that it can see the private ones if the right auth is provided
      //

      it('returns latest packages in a channel but not the older ones', function (done) {
          request.get('/depot/channels/neurosis/unstable/pkgs/latest?target=x86_64-linux')
            .type('application/json')
            .accept('application/json')
            .expect(200)
            .end(function (err, res) {
              expect(res.body.channel).to.equal('unstable');
              expect(res.body.target).to.equal('x86_64-linux');
              expect(res.body.data.length).to.equal(12);
              expect(res.body.data[0].name).to.equal('abracadabra');
              expect(res.body.data[0].version).to.equal('3.0');
              expect(res.body.data[0].release).to.equal('20190618175235');
              expect(res.body.data[10].name).to.equal('testapp');
              expect(res.body.data[10].version).to.equal('0.1.13');
              expect(res.body.data[10].release).to.equal('20190511004436');
              expect(res.body.data[11].name).to.equal('testapp2');
              expect(res.body.data[11].version).to.equal('v1.2.3-master');
              expect(res.body.data[11].release).to.equal('20181018162212');
              done(err);
            });
        });
      });
  });

  describe('Listing channels in an origin', function () {
    it('returns a list of channels', function (done) {
      request.get('/depot/channels/neurosis')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.length).to.equal(4);
          expect(res.body[0].name).to.equal('bar');
          expect(res.body[1].name).to.equal('foo');
          expect(res.body[2].name).to.equal('stable');
          expect(res.body[3].name).to.equal('unstable');
          done(err);
        });
    });
  });

  describe('Channel demotion', function () {
    it('requires authentication to demote a package', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/testapp/0.1.3/20171205003213/demote')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires origin membership to demote a package', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/testapp/0.1.3/20171205003213/demote')
        .set('Authorization', global.mystiqueBearer)
        .expect(401)
        .end(function (err, res) {
          done(err);
        });
    });

    it('removes the specified package from the specified channel', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/testapp/0.1.3/20171205003213/demote')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('rejects attempts to demote a package from the unstable channel', function (done) {
      request.put('/depot/channels/neurosis/unstable/pkgs/testapp/0.1.3/20171205003213/demote')
        .set('Authorization', global.boboBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('will not find a package in a channel after it has been demoted', function (done) {
      request.get('/depot/channels/neurosis/foo/pkgs')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(0);
          expect(res.body.total_count).to.equal(0);
          expect(res.body.data.length).to.equal(0);
          done(err);
        });
    });
  });

  describe('Delete foo channel', function () {
    it('rejects attempts to delete the stable channel', function (done) {
      request.delete('/depot/channels/neurosis/stable')
        .set('Authorization', global.boboBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('rejects attempts to delete the unstable channel', function (done) {
      request.delete('/depot/channels/neurosis/unstable')
        .set('Authorization', global.boboBearer)
        .expect(403)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires authentication to delete a channel', function (done) {
      request.delete('/depot/channels/neurosis/foo')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires origin membership to delete a channel', function (done) {
      request.delete('/depot/channels/neurosis/foo')
        .set('Authorization', global.mystiqueBearer)
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('succeeds', function (done) {
      request.delete('/depot/channels/neurosis/foo')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  describe('Channel-to-Channel promotion', function () {
    it('requires authentication to promote all packages in channel', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/promote?channel=throneroom')
        .expect(401)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('requires origin membership to promote all packages', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/promote?channel=throneroom')
        .set('Authorization', global.mystiqueBearer)
        .expect(401)
        .end(function (err, res) {
          done(err);
        });
    });

    it('rejects attempts to promote packages when source_channel and target_channel match', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/promote?channel=foo')
        .set('Authorization', global.boboBearer)
        .expect(400)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('rejects attempts to promote packages to target_channel if set to unstable', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/promote?channel=unstable')
        .set('Authorization', global.boboBearer)
        .expect(400)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('puts all channel packages into a specified channel', function (done) {
      request.put('/depot/channels/neurosis/unstable/pkgs/promote?channel=foo')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('promotes between non-default channels channel', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/promote?channel=throneroom')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('should ignore packages promoted to a channel where the package already exists', function (done) {
      request.put('/depot/channels/neurosis/unstable/pkgs/promote?channel=foo')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('will find all packages in a channel after promotion', function (done) {
      request.get('/depot/channels/neurosis/foo/pkgs')
        .type('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(30);
          expect(res.body.total_count).to.equal(31);
          expect(res.body.data.length).to.equal(31);
          done(err);
        });
    });

    it('can promote private packages', function (done) {
      request.put('/depot/channels/neurosis/bar/pkgs/promote?channel=throneroom')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });
  });

  describe('Channel-to-channel Demotion', function () {
    it('requires authentication to demote all packages in channel', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/demote')
      .expect(401)
      .end(function (err, res) {
        expect(res.text).to.be.empty;
        done(err);
      });
    });

    it('requires origin membership to demote all packages in channel', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/demote')
        .set('Authorization', global.mystiqueBearer)
        .expect(401)
        .end(function (err, res) {
          done(err);
        });
    });

    it('removes all packages from the specified channel', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/demote?channel=throneroom')
        .set('Authorization', global.boboBearer)
        .expect(200)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('rejects attempts to demote all packages from the unstable channel', function (done) {
      request.put('/depot/channels/neurosis/unstable/pkgs/demote?channel=unstable')
        .set('Authorization', global.boboBearer)
        .expect(400)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('rejects attempts to demote packages when source_channel and target_channel match', function (done) {
      request.put('/depot/channels/neurosis/foo/pkgs/demote?channel=foo')
        .set('Authorization', global.boboBearer)
        .expect(400)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('rejects attempts to demote packages when source_channel is unstable', function (done) {
      request.put('/depot/channels/neurosis/unstable/pkgs/demote?channel=stable')
        .set('Authorization', global.boboBearer)
        .expect(400)
        .end(function (err, res) {
          expect(res.text).to.be.empty;
          done(err);
        });
    });

    it('will not find a package in a channel after it has been demoted', function (done) {
      request.get('/depot/channels/neurosis/throneroom/pkgs')
        .type('application/json')
        .accept('application/json')
        .expect(200)
        .end(function (err, res) {
          expect(res.body.range_start).to.equal(0);
          expect(res.body.range_end).to.equal(0);
          expect(res.body.total_count).to.equal(0);
          expect(res.body.data.length).to.equal(0);
          done(err);
        });
    });
  });
});
