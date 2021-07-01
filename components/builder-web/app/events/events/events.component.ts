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

import { Component, OnInit, OnDestroy } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { ActivatedRoute, Router } from '@angular/router';
import { Subscription } from 'rxjs';

import { AppStore } from '../../app.store';
import { fetchEvents } from '../../actions/index';
import { dateFilters, getDateRange } from '../date-util';

@Component({
  template: require('./events.component.html')
})
export class EventsComponent implements OnInit, OnDestroy {
  dateFilterChanged: Function;

  public filters: any;
  public currentFilter: any;

  private sub: Subscription;
  private dateRange: any;

  constructor(
    private store: AppStore,
    private route: ActivatedRoute,
    private router: Router,
    private title: Title
  ) {
    this.filters = dateFilters;
    this.currentFilter = this.filters[0];
    this.dateFilterChanged = function (item: any) {
      this.currentFilter = item;
      this.isOpen = !this.isOpen;
      this.fetchEvents(0);
      return false;
    }.bind(this);
  }

  ngOnInit() {
    let state = this.store.getState();
    // Ensure that the builder events are enabled
    if (!state.features.events) {
      this.router.navigate(['/pkgs']);
      return;
    }

    this.sub = this.route.params.subscribe(_params => {
      this.title.setTitle(`Events | ${this.store.getState().app.name}`);

      this.fetchEvents(0);
    });
  }

  ngOnDestroy() {
    if (this.sub) {
      this.sub.unsubscribe();
    }
  }

  get events() {
    return this.store.getState().events.visible;
  }

  get perPage() {
    return this.store.getState().events.perPage;
  }

  get totalCount() {
    return this.store.getState().events.totalCount;
  }

  get ui() {
    return this.store.getState().events.ui.visible;
  }

  fetchEvents(limit) {
    this.dateRange = getDateRange(this.currentFilter);
    this.store.dispatch(fetchEvents(limit, this.dateRange.fromDate, this.dateRange.toDate));
  }

  fetchMoreEvents() {
    this.store.dispatch(
      fetchEvents(this.store.getState().events.nextRange, this.dateRange.fromDate, this.dateRange.toDate)
    );
  }
}
