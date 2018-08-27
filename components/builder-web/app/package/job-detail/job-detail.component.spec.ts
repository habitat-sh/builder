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
import { RouterTestingModule } from '@angular/router/testing';
import { BehaviorSubject } from 'rxjs';
import { Record } from 'immutable';
import { MockComponent } from 'ng2-mock-component';
import { AppStore } from '../../app.store';
import * as actions from '../../actions/index';
import { JobDetailComponent } from './job-detail.component';

class MockAppStore {
  getState() {
    return {
      jobs: {
        selected: Record({
          info: {
            id: '123'
          },
          log: {
            content: new BehaviorSubject([])
          }
        })()
      },
      session: {
        token: 'some-token',
      },
      oauth: {
        token: 'some-token'
      }
    };
  }
  subscribe() { }
  dispatch() { }
}

describe('JobDetailComponent', () => {
  let fixture: ComponentFixture<JobDetailComponent>;
  let component: JobDetailComponent;
  let element: DebugElement;
  let store: AppStore;

  beforeEach(() => {

    TestBed.configureTestingModule({
      imports: [
        RouterTestingModule
      ],
      declarations: [
        JobDetailComponent,
        MockComponent({ selector: 'hab-package-breadcrumbs', inputs: ['ident'] }),
        MockComponent({ selector: 'hab-icon', inputs: ['symbol'] })
      ],
      providers: [
        { provide: AppStore, useClass: MockAppStore }
      ]
    });

    fixture = TestBed.createComponent(JobDetailComponent);
    component = fixture.componentInstance;
    element = fixture.debugElement;
    store = TestBed.get(AppStore);
  });

  describe('on init', () => {

    beforeEach(() => {
      component.job = {
        origin: 'core',
        name: 'nginx',
        id: '123'
      };

      fixture.detectChanges();
    });
  });

  describe('on changes', () => {

    describe('when a job is provided', () => {
      let changes;

      beforeEach(() => {
        changes = {
          job: {
            currentValue: {
              id: '123'
            }
          }
        };
      });

      it('fetches the specified job log', () => {
        spyOn(actions, 'fetchJobLog');
        component.ngOnChanges(changes);

        expect(actions.fetchJobLog).toHaveBeenCalledWith(
          store.getState().jobs.selected.info.id,
          store.getState().session.token,
          0
        );
      });

      describe('log streaming', () => {

        describe('by default', () => {

          it('is set to false', () => {
            spyOn(actions, 'streamJobLog');
            component.ngOnChanges(changes);

            expect(actions.streamJobLog).toHaveBeenCalledWith(false);
          });
        });

        describe('when requested', () => {

          beforeEach(() => {
            component.stream = true;
          });

          it('is set to true', () => {
            spyOn(actions, 'streamJobLog');
            component.ngOnChanges(changes);

            expect(actions.streamJobLog).toHaveBeenCalledWith(true);
          });
        });
      });

      describe('log navigation', () => {

        describe('jump-to-top button', () => {

          it('scrolls to top', () => {
            spyOn(component, 'scrollTo');
            element.query(By.css('button.jump-to-top')).triggerEventHandler('click', {});
            expect(component.scrollTo).toHaveBeenCalledWith(0);
          });

          describe('when log following is enabled', () => {

            beforeEach(() => {
              component.followLog = true;
            });

            it('disables log following', () => {
              element.query(By.css('button.jump-to-top')).triggerEventHandler('click', {});
              expect(component.followLog).toBe(false);
            });
          });
        });

        describe('follow-log button', () => {

          it('enables log following', () => {
            expect(component.followLog).toBe(false);

            spyOn(component, 'scrollToEnd');
            element.query(By.css('button.jump-to-end')).triggerEventHandler('click', {});

            expect(component.scrollToEnd).toHaveBeenCalled();
            expect(component.followLog).toBe(true);
          });
        });
      });
    });
  });

  describe('on destroy', () => {

    it('terminates log streaming', () => {
      spyOn(actions, 'streamJobLog');
      component.ngOnDestroy();

      expect(actions.streamJobLog).toHaveBeenCalledWith(false);
    });
  });

  xit('shows the selected job status', () => {

  });

  xit('shows the selected job log', () => {

  });
});
