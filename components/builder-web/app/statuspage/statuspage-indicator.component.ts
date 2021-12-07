// Copyright (c) 2016-2018 Chef Software Inc. and/or applicable contributors
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

import { AfterViewInit, Component } from '@angular/core';

@Component({
  selector: 'hab-statuspage',
  template: `
    <a href="//status.chef.io/" target="_blank" class="statuspage-component">
      <span class="color-dot {{indicator}}"></span>
      <span class="color-description">{{description}}</span>
    </a>
  `
})
export class StatuspageIndicatorComponent implements AfterViewInit {
  indicator: string;
  description: string;

  ngAfterViewInit() {
    setInterval(this.queryStatuspage.bind(this), 300000);
    this.queryStatuspage();
  }

  queryStatuspage() {
    let sp = new window['StatusPage'].page({ page: 'c0nql3j2x52j'});

    sp.components({
      success: (data) => {
        let status = '';
        data.components.forEach(component => {
          if (component.id === '7jnp2p89z282') {
            this.indicator = component.status;
            this.description = component.status.replace('_', ' ');
          }
        });
      }
    });
  }
}