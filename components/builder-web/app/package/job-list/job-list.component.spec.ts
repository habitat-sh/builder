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

import { Component, DebugElement } from '@angular/core';
import { TestBed, ComponentFixture } from '@angular/core/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { By } from '@angular/platform-browser';
import { MockComponent } from 'ng2-mock-component';
import { List } from 'immutable';
import { JobListComponent } from './job-list.component';

describe('JobListComponent', () => {
  let component: JobListComponent,
    fixture: ComponentFixture<JobListComponent>,
    element: DebugElement;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [
        RouterTestingModule
      ],
      declarations: [
        MockComponent({ selector: 'hab-icon', inputs: ['symbol'] }),
        MockComponent({ selector: 'hab-job-status-icon', inputs: ['job'] }),
        JobListComponent
      ]
    });

    fixture = TestBed.createComponent(JobListComponent);
    component = fixture.componentInstance;
    element = fixture.debugElement;
  });

  describe('given a list of jobs', () => {

    let jobs;

    beforeEach(() => {
      jobs = [
        {
          'build_finished_at': '2018-10-04T21:56:49.475924+00:00',
          'build_started_at': '2018-10-04T21:56:16.353801+00:00',
          'channels': [
            'bldr-1085686806252797952',
            'unstable'
          ],
          'created_at': '2018-10-04T21:56:15.106690+00:00',
          'id': '1085687068480929792',
          'name': 'testapp',
          'origin': 'cnunciato',
          'owner_id': '1085686806252797952',
          'platforms': [
            'x86_64-linux'
          ],
          'release': '20181004215649',
          'state': 'Complete',
          'version': '0.1.0'
        },
        {
          'created_at': '2018-10-03T21:55:43.605320+00:00',
          'id': '1085686804222992384',
          'name': 'testapp',
          'origin': 'cnunciato',
          'owner_id': '1085686803887202304',
          'state': 'CancelComplete'
        }
      ];

      component.jobs = List(jobs);

      fixture.detectChanges();
    });

    it('renders them', () => {
      let items = element.queryAll(By.css('.job-list-component ol li.item'));
      expect(items.length).toBe(2);

      function text(item, selector) {
        return item.query(By.css(selector)).nativeElement.textContent;
      }

      expect(text(items[0], '.name')).toContain('0.1.0');
      expect(text(items[0], '.package')).toContain('cnunciato / testapp');
      expect(text(items[0], '.date')).toContain('2018-10-04');

      expect(text(items[1], '.name')).toContain('—');
      expect(text(items[1], '.package')).toContain('cnunciato / testapp');
      expect(text(items[1], '.date')).toContain('2018-10-03');
    });

    describe('when a job item is clicked', () => {

      it('emits an event containing the job', () => {
        let items = element.queryAll(By.css('.job-list-component ol li.item'));

        spyOn(component.select, 'emit');
        items[1].nativeElement.click();

        expect(component.select.emit).toHaveBeenCalledWith(jobs[1]);
      });
    });
  });

  describe('given an empty list of jobs', () => {

    beforeEach(() => {
      component.jobs = List();
      fixture.detectChanges();
    });

    it('hides the list', () => {
      let el = element.query(By.css('.job-list-component ol li.item'));
      expect(el).toBeNull();
    });

    it('renders an appropriate message', () => {
      let el = element.query(By.css('.job-list-component ol li.none'));
      expect(el.nativeElement.textContent).toContain('There are no available build jobs for this package.');
    });
  });
});
