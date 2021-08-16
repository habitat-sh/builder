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
import { FormControl } from '@angular/forms';
import { Subscription } from 'rxjs';
import { debounceTime, distinctUntilChanged } from 'rxjs/operators';

import { AppStore } from '../../app.store';
import { fetchSaasEvents } from '../../actions/index';
import { dateFilters, getDateRange } from '../date-util';
import { setSaasEventsSearchQuery, setSaasEventsDateFilter } from '../../actions/events-saas';

@Component({
  template: require('./events-saas.component.html')
})
export class EventsSaaSComponent implements OnInit, OnDestroy {
  dateFilterChanged: Function;
  query: string = '';
  searchBox: FormControl;

  public filters: any;

  private sub: Subscription;
  private dateRange: any;

  constructor(
    private store: AppStore,
    private route: ActivatedRoute,
    private router: Router,
    private title: Title
  ) {
    this.searchBox = new FormControl(this.searchQuery);
    this.filters = dateFilters;
    this.dateFilterChanged = function (item: any) {
      this.currentFilter = item;
      this.isOpen = !this.isOpen;
      this.store.dispatch(setSaasEventsDateFilter(item));
      this.fetchEvents(0);
      return false;
    }.bind(this);
  }

  ngOnInit() {
    let state = this.store.getState();
    // Ensure that the builder events are enabled
    if (!(state.features.events && state.features.saasEvents)) {
      this.router.navigate(['/pkgs']);
      return;
    }

    this.sub = this.route.params.subscribe(_params => {
      this.title.setTitle(`Events (SaaS) | ${this.store.getState().app.name}`);

      this.fetchEvents(0);
    });

    this.searchBox.valueChanges
      .pipe(
        debounceTime(400),
        distinctUntilChanged()
      )
      .subscribe(query => {
        this.query = query;
        this.store.dispatch(setSaasEventsSearchQuery(query));
        this.fetchEvents(0);
      });
  }

  ngOnDestroy() {
    if (this.sub) {
      this.sub.unsubscribe();
    }
  }

  get events() {
    return this.store.getState().eventsSaas.visible;
  }

  get perPage() {
    return this.store.getState().eventsSaas.perPage;
  }

  get totalCount() {
    return this.store.getState().eventsSaas.totalCount;
  }

  get ui() {
    return this.store.getState().eventsSaas.ui.visible;
  }

  get searchQuery() {
    return this.store.getState().eventsSaas.searchQuery;
  }

  get dateFilter() {
    return this.store.getState().events.dateFilter;
  }

  get currentFilter() {
    return this.dateFilter || this.filters[0];
  }

  fetchEvents(range) {
    this.dateRange = getDateRange(this.currentFilter);
    this.store.dispatch(fetchSaasEvents(range, this.dateRange.fromDate, this.dateRange.toDate, this.query));
  }

  fetchMoreEvents() {
    this.store.dispatch(
      fetchSaasEvents(this.store.getState().eventsSaas.nextRange, this.dateRange.fromDate, this.dateRange.toDate, this.query)
    );
  }
}
