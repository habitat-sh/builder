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

import { TestBed, ComponentFixture } from '@angular/core/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { DebugElement } from '@angular/core';
import { By } from '@angular/platform-browser';
import { List } from 'immutable';
import { MockComponent } from 'ng2-mock-component';

import { EventResultsComponent } from './results.component';

describe('EventResultsComponent', () => {
  let fixture: ComponentFixture<EventResultsComponent>;
  let component: EventResultsComponent;
  let element: DebugElement;

  beforeEach(() => {

    TestBed.configureTestingModule({
      imports: [
        RouterTestingModule
      ],
      declarations: [
        EventResultsComponent,
        MockComponent({ selector: 'hab-job-status-icon', inputs: ['status'] }),
      ]
    });

    fixture = TestBed.createComponent(EventResultsComponent);
    component = fixture.componentInstance;
    element = fixture.debugElement;
  });

  beforeEach(() => {
    component.events = List([
      {
        "operation": "Demote",
        "created_at": "2021-05-18T15:02:33.095231",
        "origin": "rcpd",
        "channel": "stable",
        "package_ident": {
          "origin": "rcpd",
          "name": "testapp",
          "version": "0.1.0",
          "release": "20200401202136"
        }
      }, {
        "operation": "Promote",
        "created_at": "2021-05-18T15:02:33.059659",
        "origin": "rcpd",
        "channel": "stable",
        "package_ident": {
          "origin": "rcpd",
          "name": "testapp",
          "version": "0.1.0",
          "release": "20200401202136"
        }
      }, {
        "operation": "Demote",
        "created_at": "2021-05-18T15:02:29.212797",
        "origin": "neurosis",
        "channel": "foo",
        "package_ident": {
          "origin": "neurosis",
          "name": "testapp",
          "version": "0.1.3",
          "release": "20171205003213"
        }
      }, {
        "operation": "Promote",
        "created_at": "2021-05-18T15:02:28.521932",
        "origin": "neurosis",
        "channel": "foo",
        "package_ident": {
          "origin": "neurosis",
          "name": "testapp",
          "version": "0.1.3",
          "release": "20171205003213"
        }
      }
    ]);
    fixture.detectChanges();
  });

  it('renders a list of events', () => {
    let items = element.queryAll(By.css('.results-component ol li.item'));
    expect(items.length).toBe(4);

    function text(item, selector) {
      return item.query(By.css(selector)).nativeElement.textContent;
    }

    expect(text(items[0], '.origin')).toContain('rcpd');
    expect(text(items[0], '.channel')).toContain('stable');
    expect(text(items[0], '.pident')).toContain('rcpd/testapp/0.1.0/20200401202136');

    expect(text(items[1], '.origin')).toContain('rcpd');
    expect(text(items[1], '.channel')).toContain('stable');
    expect(text(items[1], '.pident')).toContain('rcpd/testapp/0.1.0/20200401202136');

    expect(text(items[2], '.origin')).toContain('neurosis');
    expect(text(items[2], '.channel')).toContain('foo');
    expect(text(items[2], '.pident')).toContain('neurosis/testapp/0.1.3/20171205003213');

    expect(text(items[3], '.origin')).toContain('neurosis');
    expect(text(items[3], '.channel')).toContain('foo');
    expect(text(items[3], '.pident')).toContain('neurosis/testapp/0.1.3/20171205003213');
  });

  describe('given an empty list of events', () => {

    beforeEach(() => {
      component.events = List();
      fixture.detectChanges();
    });

    it('hides the list', () => {
      let el = element.query(By.css('.results-component ol li.item'));
      expect(el).toBeNull();
    });

    it('renders an appropriate message', () => {
      let el = element.query(By.css('.results-component ol li.none'));
      expect(el.nativeElement.textContent).toContain('No events found.');
    });
  });
});
