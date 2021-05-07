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

import { Component, Input } from '@angular/core';
import { Router } from '@angular/router';
import { List } from 'immutable';
import * as moment from 'moment';

import { packageString } from '../../util';
import { Browser } from '../../browser';

const BLDR_SAAS = 'https://bldr.habitat.sh';

@Component({
  selector: 'hab-event-results',
  template: require('./results.component.html')
})
export class EventResultsComponent {
  @Input() errorMessage: string;
  @Input() noEvents: boolean;
  @Input() events: List<Object>;
  @Input() fromSaas: boolean;

  constructor(
    private router: Router
  ) {
  }

  onClick(event: any) {
    if (this.fromSaas) {
      const url = `${BLDR_SAAS}/#pkgs/${event.origin}/${event.package_ident.name}/${event.package_ident.version}/${event.package_ident.release}`;
      Browser.openInTab(url);
    } else {
      this.router.navigate(['/pkgs', event.origin, event.package_ident.name, event.package_ident.version, event.package_ident.release]);
    }
  }

  packageString(event) {
    return packageString(event.package_ident);
  }

  dateFor(timestamp) {
    return moment(timestamp, 'YYYY-MM-DDTHH:mm:ss').fromNow();
  }

  stateFor(event) {
    return event.operation;
  }
}
