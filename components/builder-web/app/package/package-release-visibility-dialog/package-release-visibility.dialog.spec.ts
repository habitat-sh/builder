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

import { DebugElement } from '@angular/core';
import { TestBed, ComponentFixture } from '@angular/core/testing';
import { MatDialogModule, MatDialogRef, MAT_DIALOG_DATA } from '@angular/material';
import { PackageReleaseVisibilityDialog } from './package-release-visibility.dialog';

describe('PackageReleaseVisibilityDialog', () => {
  let fixture: ComponentFixture<PackageReleaseVisibilityDialog>;
  let component: PackageReleaseVisibilityDialog;
  let element: DebugElement;
  let dialogRef = {
    open() {},
    close() {}
  };
  let dialogData = {
    visibility: 'private',
    package: {
      ident: {
        origin: 'testorigin',
        name: 'testname',
        version: '1.0',
        release: '100'
      }
    }
  };

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [
        MatDialogModule
      ],
      declarations: [
        PackageReleaseVisibilityDialog
      ],
      providers: [
        { provide: MatDialogRef, useFactory: () => dialogRef },
        { provide: MAT_DIALOG_DATA, useValue: dialogData }
      ]
    });

    fixture = TestBed.createComponent(PackageReleaseVisibilityDialog);
    component = fixture.componentInstance;
    element = fixture.debugElement;
    fixture.detectChanges();
  });

  it('creates', () => {
    expect(component).toBeTruthy();
  });

  describe('artifactName', () => {
    it('is correct', () => {
      expect(component.artifactName).toEqual('testorigin/testname/1.0/100');
    });
  });

  describe('confirm()', () => {
    it('closes dialog ref with true value', () => {
      spyOn(dialogRef, 'close');

      component.confirm();

      expect(dialogRef.close).toHaveBeenCalledWith(true);
    });
  });

  describe('cancel()', () => {
    it('closes dialog ref with false value', () => {
      spyOn(dialogRef, 'close');

      component.cancel();

      expect(dialogRef.close).toHaveBeenCalledWith(false);
    });
  });
});
