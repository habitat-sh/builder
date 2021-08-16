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

import { Component, Input, ViewChild } from '@angular/core';
import { MatCalendar } from '@angular/material';

import { getDateRange, toDateString, toDate } from '../date-util';

@Component({
  selector: 'hab-events-date-filter',
  template: require('./date-filter.component.html')
})
export class DateFilterComponent {

  @ViewChild('fromDateCal') fromDateCal: MatCalendar<Date>;

  @Input() dateFilterChanged: Function;
  @Input() currentFilter: any;
  @Input() filters: any;

  maxDate: Date;
  fromSelected: Date | null;
  toSelected: Date | null;

  public showCalender = false;

  constructor() {
    this.maxDate = new Date();
  }

  getCurrentFilterLabel() {
    return this.currentFilter.label;
  }

  filterChanged(item: any) {
    if (this.currentFilter.label === item.label)
      return;

    this.dateFilterChanged(item);
  }

  fromCalender() {
    this.showCalender = true;
    const dateRange = getDateRange(this.currentFilter);
    this.fromSelected = toDate(dateRange.fromDate);
    this.toSelected = toDate(dateRange.toDate);

    setTimeout(() => {
      this.fromDateCal.activeDate = this.fromSelected;
    }, 500);
  }

  triggerDisabled() {
    return this.showCalender;
  }

  closeDateRange() {
    this.showCalender = false;
  }

  cancel() {
    this.closeDateRange();
  }

  apply() {
    const fromDateStr = toDateString(this.fromSelected);
    const toDateStr = toDateString(this.toSelected);
    const filter = {
      label: `${fromDateStr} - ${toDateStr}`,
      type: 'custom',
      startDate: this.fromSelected,
      endDate: this.toSelected
    };

    this.dateFilterChanged(filter);
    this.closeDateRange();
  }

  disabledApply() {
    if (this.fromSelected <= this.toSelected)
      return false;

    return true;
  }

  getStartDate() {
    if (this.fromSelected)
      return toDateString(this.fromSelected);

    return '';
  }

  getEndDate() {
    if (this.toSelected)
      return toDateString(this.toSelected);

    return '';
  }
}
