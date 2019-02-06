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

import { Component, OnInit, OnDestroy } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';
import { MatDialog } from '@angular/material';
import { AppStore } from '../../../app.store';
import { Title } from '@angular/platform-browser';
import { Subscription } from 'rxjs';
import { JobCancelDialog } from '../../../shared/dialog/job-cancel/job-cancel.dialog';
import { fetchJobGroup, cancelJobGroup } from '../../../actions/index';
import { parseDate, iconForJobState } from '../../../util';

@Component({
  selector: 'hab-origin-job-detail',
  template: require('./origin-job-detail.component.html')
})
export class OriginJobDetailComponent implements OnInit, OnDestroy {
  origin: string;
  name: string;
  id: string;
  selected: object;

  private sub: Subscription;
  private parentSub: Subscription;
  private poll: number;
  private completedStates = ['success', 'failure'];
  private cancelableStates = ['notstarted', 'inprogress'];

  constructor(
    private route: ActivatedRoute,
    private store: AppStore,
    private router: Router,
    private cancelDialog: MatDialog,
    private title: Title
  ) {
    this.sub = this.route.params.subscribe((params) => {
      this.id = params['id'];
      this.title.setTitle(`Packages › Build Job Groups > ${this.id} | Habitat`);
    });
    this.parentSub = this.route.parent.params.subscribe((params) => this.origin = params['origin']);
  }

  ngOnInit() {
    this.poll = window.setInterval(() => {
      this.fetchJobGroup();
    }, 5000);

    this.fetchJobGroup();
  }

  ngOnDestroy() {
    if (this.sub) {
      this.sub.unsubscribe();
      this.parentSub.unsubscribe();
    }

    window.clearInterval(this.poll);
  }

  get cancelableCount() {
    return this.cancelableStates.reduce((total, state) => {
      const stateList = this.group.projects_by_state[state];
      total += stateList ? stateList.length : 0;
      return total;
    }, 0);
  }

  get completedCount() {
    return this.completedStates.reduce((total, state) => {
      const stateList = this.group.projects_by_state[state];
      total += stateList ? stateList.length : 0;
      return total;
    }, 0);
  }

  get group() {
    return this.store.getState().jobGroups.selected;
  }

  get projects() {
    return this.group.projects;
  }

  get token() {
    return this.store.getState().session.token;
  }

  get totalCount() {
    return this.group.projects.length;
  }

  projectStateCount(param) {
    const stateList = this.group.projects_by_state[param];
    return stateList ? stateList.length : 0;
  }

  dateFor(timestamp) {
    return parseDate(timestamp, 'YYYY-MM-DD HH:mm:ss');
  }

  onSelectJob(name, job) {
    this.router.navigate(['pkgs', ...name.split('/'), 'jobs', job]);
  }

  backToGroups() {
    this.router.navigate(['origins', this.origin, 'jobs']);
  }

  hasJobId(project) {
    return project.job_id !== '0';
  }

  cancel(id) {
    this.cancelDialog
      .open(JobCancelDialog, {
        width: '480px',
        data: {
          cancelableCount: this.cancelableCount
        }
      })
      .afterClosed()
      .subscribe(confirmed => {
        if (confirmed) {
          this.store.dispatch(cancelJobGroup(this.id, this.token));
        }
      });
  }

  private fetchJobGroup() {
    if (this.token) {
      this.store.dispatch(fetchJobGroup(this.id, this.token));
    }
  }
}
