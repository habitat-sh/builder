// Copyright (c) 2016-2017 Chef Software Inc. and/or applicable contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

import * as util from './util';

describe('util', () => {
  describe('packageString', () => {
    describe('with a fully qualified identifier', () => {
      it('returns the string', () => {
        expect(util.packageString({
          origin: 'testorigin',
          name: 'testname',
          version: '1.0.0',
          release: '197001010000',
        })
        ).toEqual('testorigin/testname/1.0.0/197001010000');
      });
    });

    describe('with a missing parts', () => {
      it('returns the partial string', () => {
        expect(util.packageString({
          origin: 'testorigin',
          name: 'testname',
        })
        ).toEqual('testorigin/testname');
      });
    });
  });

  describe('parseKey', () => {
    describe('with an invalid key', () => {
      it('has a valid:false property', () => {
        expect(util.parseKey('').valid).toEqual(false);
      });
    });

    describe('with a valid key', () => {
      let keyString;

      beforeEach(() => {
        keyString = `SIG-PUB-1
core-20160423193745

Jpmj1gD9oTFCgz3wSLltt/QB6RTmNRWoUTe+xhDTIHc=`;
      });

      it('has a name property', () => {
        expect(util.parseKey(keyString).name).toEqual(
          'core-20160423193745'
        );
      });

      it('has a valid:true property', () => {
        expect(util.parseKey(keyString).valid).toEqual(true);
      });

      it('has an origin property', () => {
        expect(util.parseKey(keyString).origin).toEqual('core');
      });

      it('has a text property', () => {
        expect(util.parseKey(keyString).text).toEqual(keyString);
      });

      describe('with a private key', () => {
        beforeEach(() => {
          keyString = `SIG-SEC-1
core-20160423193745

NNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNNN==`;
        });

        it('has an uploadPath property', () => {
          expect(util.parseKey(keyString).uploadPath)
            .toEqual('core/secret_keys/20160423193745');
        });
      });

      describe('with a public key', () => {
        it('has a type property', () => {
          expect(util.parseKey(keyString).type).toEqual('SIG-PUB-1');
        });

        it('has an uploadPath property', () => {
          expect(util.parseKey(keyString).uploadPath)
            .toEqual('core/keys/20160423193745');
        });
      });
    });
  });

  describe('targetFrom', () => {
    it('returns target that has matching value for given key', () => {
      expect(util.targetFrom('id', 'x86_64-linux')).toEqual({
        id: 'x86_64-linux',
        name: 'Linux',
        title: 'Linux',
        param: 'linux'
      });
      expect(util.targetFrom('name', 'Windows')).toEqual({
        id: 'x86_64-windows',
        name: 'Windows',
        title: 'Windows',
        param: 'windows'
      });
      expect(util.targetFrom('title', 'Linux (Kernel Version 2)')).toEqual({
        id: 'x86_64-linux-kernel2',
        name: 'Linux 2',
        title: 'Linux (Kernel Version 2)',
        param: 'linux2'
      });
      expect(util.targetFrom('param', 'mac')).toEqual({
        id: 'x86_64-darwin',
        name: 'macOS',
        title: 'macOS',
        param: 'mac'
      });
    });

    describe('with an unknown key and/or value', () => {
      it('returns undefined', () => {
        expect(util.targetFrom('id', 'not_a_key')).toEqual(undefined);
        expect(util.targetFrom('id', 'not_a_value')).toEqual(undefined);
        expect(util.targetFrom('not_a_key', 'not_a_value')).toEqual(undefined);
      });
    });
  });

  describe('targetsFromIds', () => {
    it('returns array of targets that has matching id value within provided list of ids', () => {
      expect(util.targetsFromIds(['x86_64-windows'])).toEqual([{
        id: 'x86_64-windows',
        name: 'Windows',
        title: 'Windows',
        param: 'windows'
      }]);
      expect(util.targetsFromIds(['x86_64-windows', 'x86_64-linux-kernel2'])).toEqual([
        {
          id: 'x86_64-windows',
          name: 'Windows',
          title: 'Windows',
          param: 'windows'
        },
        {
          id: 'x86_64-linux-kernel2',
          name: 'Linux 2',
          title: 'Linux (Kernel Version 2)',
          param: 'linux2'
        }
      ]);
    });

    describe('when provided an empty list of ids', () => {
      it('returns an empty array', () => {
        expect(util.targetsFromIds([])).toEqual([]);
      });
    });

    describe('when provided a list that includes an unknown id', () => {
      it('returns array of targets that ignores unknown ids', () => {
        expect(util.targetsFromIds(['x86_64-windows', 'x99_99-not-real', 'x86_64-darwin'])).toEqual([
          {
            id: 'x86_64-windows',
            name: 'Windows',
            title: 'Windows',
            param: 'windows'
          },
          {
            id: 'x86_64-darwin',
            name: 'macOS',
            title: 'macOS',
            param: 'mac'
          }
        ]);
      });
    });
  });

  describe('targetsFromPkgVersions', () => {
    const allPlatforms = [
      { 'platforms': ['x86_64-linux', 'x86_64-linux-kernel2', 'x86_64-windows'] },
      { 'platforms': ['x86_64-linux', 'x86_64-windows', 'x86_64-darwin'] },
      { 'platforms': ['x86_64-linux', 'x86_64-windows'] }
    ];
    const somePlatforms = [
      { 'platforms': ['x86_64-linux', 'x86_64-linux-kernel2'] },
      { 'platforms': ['x86_64-linux'] },
      { 'platforms': ['x86_64-linux'] }
    ];
    const someUnknownPlatforms = [
      { 'platforms': ['x99_99-not-real', 'x86_64-linux-kernel2', 'x86_64-windows'] },
      { 'platforms': ['x99_99-not-real', 'x86_64-windows', 'x86_64-darwin'] },
      { 'platforms': ['x99_99-not-real', 'x86_64-windows'] }
    ];
    const emptyPlatforms = [
      { 'platforms': [] },
      { 'platforms': [] },
      { 'platforms': [] }
    ];

    it('returns an array of targets that match target ids found in array of platforms', () => {
      expect(util.targetsFromPkgVersions(allPlatforms)).toEqual([
        {
          id: 'x86_64-linux',
          name: 'Linux',
          title: 'Linux',
          param: 'linux'
        },
        {
          id: 'x86_64-linux-kernel2',
          name: 'Linux 2',
          title: 'Linux (Kernel Version 2)',
          param: 'linux2'
        },
        {
          id: 'x86_64-windows',
          name: 'Windows',
          title: 'Windows',
          param: 'windows'
        },
        {
          id: 'x86_64-darwin',
          name: 'macOS',
          title: 'macOS',
          param: 'mac'
        }
      ]);
      expect(util.targetsFromPkgVersions(somePlatforms)).toEqual([
        {
          id: 'x86_64-linux',
          name: 'Linux',
          title: 'Linux',
          param: 'linux'
        },
        {
          id: 'x86_64-linux-kernel2',
          name: 'Linux 2',
          title: 'Linux (Kernel Version 2)',
          param: 'linux2'
        }
      ]);
    });

    describe('when array of platforms includes unknown ids', () => {
      it('returns array of targets that ignores unknown ids', () => {
        expect(util.targetsFromPkgVersions(someUnknownPlatforms)).toEqual([
          {
            id: 'x86_64-linux-kernel2',
            name: 'Linux 2',
            title: 'Linux (Kernel Version 2)',
            param: 'linux2'
          },
          {
            id: 'x86_64-windows',
            name: 'Windows',
            title: 'Windows',
            param: 'windows'
          },
          {
            id: 'x86_64-darwin',
            name: 'macOS',
            title: 'macOS',
            param: 'mac'
          }
        ]);
      });
    });

    describe('when array of platforms is empty', () => {
      it('returns an empty array', () => {
        expect(util.targetsFromPkgVersions(emptyPlatforms)).toEqual([]);
      });
    });
  });
});
