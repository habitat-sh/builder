// Copyright (c) 2021 Chef Software Inc. and/or applicable contributors
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
import { ReactiveFormsModule } from '@angular/forms';
import { MatInputModule } from '@angular/material';
import { By } from '@angular/platform-browser';
import { NoopAnimationsModule } from '@angular/platform-browser/animations';
import { ActivatedRoute } from '@angular/router';
import { RouterTestingModule } from '@angular/router/testing';
import { of } from 'rxjs';
import { List } from 'immutable';
import { MockComponent } from 'ng2-mock-component';

import { AppStore } from '../../app.store';
import { EventsComponent } from './events.component';

class MockAppStore {
  static state;

  getState() {
    return MockAppStore.state;
  }

  dispatch() { }
}

class MockRoute {
  get params() {
    return of({});
  }
}

describe('EventsComponent', () => {
  let fixture: ComponentFixture<EventsComponent>;
  let component: EventsComponent;
  let element: DebugElement;
  let store: AppStore;

  beforeEach(() => {
    MockAppStore.state = {
      events: {
        visible: List(),
        ui: {
          visible: {}
        }
      },
      app: {
        name: 'Habitat'
      }
    };
  });

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [
        ReactiveFormsModule,
        RouterTestingModule,
        MatInputModule,
        NoopAnimationsModule
      ],
      declarations: [
        MockComponent({
          selector: 'hab-event-results',
          inputs: ['errorMessage', 'noEvents', 'events']
        }),
        EventsComponent
      ],
      providers: [
        { provide: AppStore, useClass: MockAppStore },
        { provide: ActivatedRoute, useClass: MockRoute }
      ]
    });

    fixture = TestBed.createComponent(EventsComponent);
    component = fixture.componentInstance;
    element = fixture.debugElement;
    store = TestBed.get(AppStore);
  });

  describe('given the events', () => {

    beforeEach(() => {
      fixture.detectChanges();
    });

    it('shows the Builder Events heading', () => {
      let heading = element.query(By.css('.events-component h1'));
      expect(heading.nativeElement.textContent).toBe('Builder Events');
    });
  });
});
