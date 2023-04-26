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

import { TestBed, ComponentFixture } from '@angular/core/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { Component, DebugElement } from '@angular/core';
import { MatTabsModule } from '@angular/material';
import { By } from '@angular/platform-browser';
import { ActivatedRoute } from '@angular/router';
import { of } from 'rxjs';
import { MockComponent } from 'ng2-mock-component';
import * as actions from '../../actions/index';
import { AppStore } from '../../app.store';
import { PackageComponent } from './package.component';
import { get } from 'lodash';

class MockAppStore {
  static state;

  getState() {
    return MockAppStore.state;
  }

  dispatch() { }

  observe(path) {
    return of(get(this.getState(), path));
  }
}

class MockRoute {
  params = of({
    origin: 'core',
    name: 'nginx'
  });

  snapshot = of([]);
}

describe('PackageComponent', () => {
  let fixture: ComponentFixture<PackageComponent>;
  let component: PackageComponent;
  let element: DebugElement;

  beforeEach(() => {
    MockAppStore.state = {
      jobs: {
        visible: []
      },
      features: {
        builder: false
      },
      origins: {
        mine: [],
        current: {
          default_package_visibility: 'public'
        }
      },
      packages: {
        currentPlatforms: [
          {
            id: 'x86_64-linux',
            name: 'Linux',
            title: 'Linux',
            param: 'linux'
          }
        ]
      },
      projects: {
        ui: {
          current: {
            exists: true
          }
        },
        current: {
          visibility: 'private',
          vcs_data: 'https://github.com/cnunciato/testapp.git',
          auto_rebuild: false
        },
        currentProjects: []
      },
      session: {
        token: 'some-token'
      },
      router: {
        route: {
          params: {
            origin: 'core',
            name: 'nginx'
          }
        }
      }
    };
  });

  beforeEach(() => {

    TestBed.configureTestingModule({
      imports: [
        RouterTestingModule,
        MatTabsModule
      ],
      declarations: [
        PackageComponent,
        MockComponent({ selector: 'hab-package-breadcrumbs', inputs: ['ident'] }),
        MockComponent({ selector: 'hab-package-sidebar', inputs: ['origin', 'name', 'target', 'building', 'isOriginMember', 'isNewProject', 'hasPlan'] }),
        MockComponent({ selector: 'hab-job-notice', inputs: ['job'] }),
        MockComponent({ selector: 'hab-visibility-icon', inputs: ['visibility', 'prefix'] })
      ],
      providers: [
        { provide: ActivatedRoute, useClass: MockRoute },
        { provide: AppStore, useClass: MockAppStore }
      ]
    });

    fixture = TestBed.createComponent(PackageComponent);
    component = fixture.componentInstance;
    element = fixture.debugElement;
  });

  describe('given origin and name', () => {

    it('renders breadcrumbs and sidebar', () => {
      component.showSidebar = true;
      fixture.detectChanges();

      expect(element.query(By.css('hab-package-breadcrumbs'))).not.toBeNull();
      expect(element.query(By.css('hab-package-sidebar'))).not.toBeNull();
    });

    describe('when Builder is disabled', () => {

      beforeEach(() => {
        MockAppStore.state.features.builder = false;
      });

      it ('suppresses the Build Jobs and Settings tabs', () => {
        fixture.detectChanges();

        expect(element.query(By.css('[routerlink="jobs"]'))).toBeNull();
        expect(element.query(By.css('[routerlink="settings"]'))).toBeNull();
      });
    });

    describe('when Builder is enabled', () => {

      beforeEach(() => {
        MockAppStore.state.features.builder = true;
      });

      describe('and the user is an origin member', () => {

        beforeEach(() => {
          MockAppStore.state.origins.mine = [ { name: 'core' } ];
        });

        it('exposes the Build Jobs and Settings tabs', () => {
          fixture.detectChanges();

          expect(element.query(By.css('[routerlink="jobs"]'))).not.toBeNull();
          expect(element.query(By.css('[routerlink="settings"]'))).not.toBeNull();
        });
      });
    });
  });
});
