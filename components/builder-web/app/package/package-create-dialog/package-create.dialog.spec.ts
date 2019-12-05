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
import { DebugElement } from '@angular/core';
import { By } from '@angular/platform-browser';
import { of } from 'rxjs';
import { MockComponent } from 'ng2-mock-component';
import { Package } from '../../records/Package';
import { AppStore } from '../../app.store';
import { PackageCreateDialog } from '../package-create-dialog/package-create.dialog';
import * as actions from '../../actions';
import { MatDialog } from '@angular/material';

class MockAppStore {
  getState() {
    return {
      session: {
        token: 'token'
      },
      origins: {
        current: {
          name: 'mock-origin'
        }
      }
    };
  }

  dispatch() { }
}

class MockDialog { }

describe('PackageCreateDialog', () => {
  let fixture: ComponentFixture<PackageCreateDialog>;
  let component: PackageCreateDialog;
  let element: DebugElement;
  let store: MockAppStore;

  beforeEach(() => {

    store = new MockAppStore();
    spyOn(store, 'dispatch');
    spyOn(actions, 'createEmptyPackage');

    TestBed.configureTestingModule({
      imports: [],
      declarations: [
        PackageCreateDialog
      ],
      providers: [
        { provide: AppStore, useValue: store },
        { provide: MatDialog, useClass: MockDialog }
      ]
    });
  });

  describe('something', () => {

    it('tests something', () => {
      console.log('test goes here');
    });
  });
});