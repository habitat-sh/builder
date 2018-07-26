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
import { Title } from '@angular/platform-browser';
import { ActivatedRoute, Router } from '@angular/router';
import { Subscription } from 'rxjs';
import { AppStore } from '../../../app.store';
import { fetchJobGroups } from '../../../actions/index';
import { List } from 'immutable';

@Component({
  template: require('./origin-jobs-tab.component.html')
})
export class OriginJobsTabComponent implements OnInit, OnDestroy {
  origin: string;
  name: string;

  private sub: Subscription;
  private poll: number;

  constructor(
    private route: ActivatedRoute,
    private store: AppStore,
    private router: Router,
    private title: Title
  ) {

    this.sub = this.route.parent.params.subscribe((params) => {
      this.origin = params['origin'];
      this.name = params['name'];

      this.title.setTitle(`Packages › ${this.origin} › Build Jobs | Habitat`);
    });
  }

  ngOnInit() {

    // When a build is active, check on it periodically so we can
    // indicate when it completes.
    this.poll = window.setInterval(() => {
      this.fetchJobGroups();
    }, 10000);

    this.fetchJobGroups();
  }

  ngOnDestroy() {
    if (this.sub) {
      this.sub.unsubscribe();
    }
    window.clearInterval(this.poll);
  }

  get token() {
    return this.store.getState().session.token;
  }

  get jobGroups() {
    return this.store.getState().jobGroups.visible;
  }

  get activeJob(): List<any> {
    const activeStates = ['Dispatched', 'Pending', 'Processing'];

    return this.store.getState().jobs.visible.filter((b) => {
      return activeStates.indexOf(b.state.toString()) !== -1;
    }).last();
  }

  onSelect(jobGroup) {
    this.router.navigate(['origins', this.origin, 'jobs', jobGroup.id]);
  }

  private fetchJobGroups() {
    if (this.token) {
      this.store.dispatch(fetchJobGroups(this.origin, this.token, 50));
    }
  }
}
