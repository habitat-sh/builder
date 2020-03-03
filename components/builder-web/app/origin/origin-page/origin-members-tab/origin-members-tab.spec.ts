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
import { FormsModule, ReactiveFormsModule } from '@angular/forms';
import { By } from '@angular/platform-browser';
import { MatDialog } from '@angular/material';
import { List } from 'immutable';
import { ActivatedRoute, Router } from '@angular/router';
import { Observable } from 'rxjs';
import { MockComponent } from 'ng2-mock-component';
import { AppStore } from '../../../app.store';
import { Origin } from '../../../records/Origin';
import { OriginMembersTabComponent } from './origin-members-tab.component';
import * as actions from '../../../actions';


class MockAppStore {
  getState() {
    return {
      session: {
        token: 'token'
      },
      oauth: {
        token: 'token'
      },
      origins: {
        mine: List([Origin({ name: 'test' })]),
        current: {
          owner_id: 111111,
          default_package_visibility: 'public'
        }
      },
      users: {
        current: {
          profile: {
            id: 123456
          }
        }
      },
      app: {
        name: 'Habitat'
      }
    };
  }

  dispatch() { }
}

class MockDialog { }

fdescribe('OriginMembersTabComponent', () => {
  let fixture: ComponentFixture<OriginMembersTabComponent>;
  let component: OriginMembersTabComponent;
  let element: DebugElement;
  let store: MockAppStore;

  beforeEach(() => {

    store = new MockAppStore();
    spyOn(store, 'dispatch');
    spyOn(actions, 'departOrigin');

    TestBed.configureTestingModule({
      imports: [
        RouterTestingModule,
        FormsModule,
        ReactiveFormsModule
      ],
      declarations: [
        OriginMembersTabComponent,
        MockComponent({ selector: 'hab-icon', inputs: ['symbol', 'cancel'] })
      ],
      providers: [
        { provide: AppStore, useValue: store },
        { provide: MatDialog, useClass: MockDialog }
      ]
    }).compileComponents();

    fixture = TestBed.createComponent(OriginMembersTabComponent);
    component = fixture.componentInstance;
    element = fixture.debugElement;
  });

  describe('component', () => {

    it('should exist', () => {
      expect(component).toBeTruthy();
    });
  });

  describe('departing an origin', () => {

    it('shows a depart origin button when user is not the owner', () => {
      // console.log('*************');
      // console.log(component.isOriginOwner);
      fixture.detectChanges();
      let departButton = fixture.debugElement.query(By.css('#dfo-section button'));
      expect(departButton).toBeTruthy();
    });

    it('opens up a modal when depart button is clicked', () => {
      element.query(By.css('#dfo-section button')).nativeElement.click();
      fixture.detectChanges();
      const modalWindow = element.query(By.css('.dialog.depart-origin'));

      expect(modalWindow).toBeTruthy();
    });

    it('removes the user from the origin when the button is clicked', () => {

      // expect() // user to be gone
    });

    it('reloads the page after departure', () => {
      // expect page reload
    });
  });


});
