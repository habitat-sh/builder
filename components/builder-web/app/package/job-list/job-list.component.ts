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

import { Component, EventEmitter, Input, Output } from '@angular/core';
import { List } from 'immutable';
import * as moment from 'moment';
import { iconForJobState } from '../../util';

@Component({
  selector: 'hab-job-list',
  template: require('./job-list.component.html')
})
export class JobListComponent {
  @Input() jobs = List();
  @Output() select = new EventEmitter();

  onClick(job) {
    this.select.emit(job);
  }

  dateFor(timestamp) {
    return moment(timestamp, 'YYYY-MM-DDTHH:mm:ss').format('YYYY-MM-DD');
  }

  iconFor(state) {
    return iconForJobState(state);
  }
}
