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

import { Component, OnDestroy, OnInit } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { ActivatedRoute } from '@angular/router';
import { MatDialog } from '@angular/material';
import { Subscription } from 'rxjs';
import { AppStore } from '../../app.store';

@Component({
  selector: 'hab-package-settings',
  template: require('./package-settings.component.html')
})
export class PackageSettingsComponent implements OnDestroy, OnInit {
  name: string;
  origin: string;
  visibilitySelectorContent: any;

  private sub: Subscription;

  constructor(
    private route: ActivatedRoute,
    private store: AppStore,
    private disconnectDialog: MatDialog,
    private title: Title
  ) {
    this.sub = this.route.parent.params.subscribe((params) => {
      this.origin = params['origin'];
      this.name = params['name'];
      this.title.setTitle(`Packages › ${this.origin}/${this.name} › Settings | ${store.getState().app.name}`);

       // Move this to its own Data file?
      this.visibilitySelectorContent = {
        option1: {
          title: 'Public artifact',
          description: 'Artifacts will appear in public search results and can be utilized by any user.',
        },
        option2: {
          title: 'Private artifact',
          description: 'Artifacts will NOT appear in public search results and can ONLY be utilized by members of this origin.',
        }
      };
      // end move

    });
  }

  ngOnInit() {
    this.getPackageSettingsData();
  }

  ngOnDestroy() {
    if (this.sub) {
      this.sub.unsubscribe();
    }
  }

  get project() {
    const project = this.store.getState().projects.current;
    const exists = this.store.getState().projects.ui.current.exists;

    const isMember = !!this.store.getState().origins.mine.find((o) => {
      return o.name === this.origin;
    });

    if (isMember && exists) {
      return project;
    }
  }

  get token() {
    return this.store.getState().session.token;
  }

  get integrations() {
    return this.store.getState().origins.currentIntegrations.integrations || [];
  }

  get loading() {
    return this.store.getState().projects.ui.current.loading;
  }

  getPackageSettingsData() {
    // need to call api to get package data
    // will need to check if default visibility is being set somewhere else already
    return this.store.getState().packages.current;
  }

  get visibility() {
    return console.log('get default visibility from packageData.');
  }

  saved(project) {
    window.scroll(0, 0);
  }

  updatePackageVisibility(setting) {
    console.log('dispatch to store and update package visibility setting');
    // this.store.dispatch(updateOrigin({ name: this.origin.name, default_package_visibility: setting }, this.token));
  }
}
