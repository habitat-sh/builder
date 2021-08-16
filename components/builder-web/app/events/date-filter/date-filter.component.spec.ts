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

import { AppStore } from '../../app.store';
import { DateFilterComponent } from './date-filter.component';

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

describe('DateFilterComponent', () => {
  let fixture: ComponentFixture<DateFilterComponent>;
  let component: DateFilterComponent;
  let element: DebugElement;
  let store: AppStore;

  beforeEach(() => {
    MockAppStore.state = {
      filters: []
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
        DateFilterComponent
      ],
      providers: [
        { provide: AppStore, useClass: MockAppStore },
        { provide: ActivatedRoute, useClass: MockRoute }
      ]
    });

    fixture = TestBed.createComponent(DateFilterComponent);
    component = fixture.componentInstance;
    element = fixture.debugElement;
    store = TestBed.get(AppStore);
  });

  describe('given the events', () => {

    beforeEach(() => {
      fixture.detectChanges();
    });

    it('show label', () => {
      let heading = element.query(By.css('.show-label'));
      expect(heading.nativeElement.textContent).toBe('Show');
    });
  });
});
