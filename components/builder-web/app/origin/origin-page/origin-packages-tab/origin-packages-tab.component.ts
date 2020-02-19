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
import { MatDialog } from '@angular/material';
import { Subscription } from 'rxjs';
import { AppStore } from '../../../app.store';
import { getUniquePackages } from '../../../actions/index';
import config from '../../../config';
import { PackageCreateDialog } from '../../../package/package-create-dialog/package-create.dialog';

@Component({
  selector: 'hab-origin-packages-tab',
  template: require('./origin-packages-tab.component.html')
})
export class OriginPackagesTabComponent implements OnInit, OnDestroy {
  selectingPlan: boolean = false;

  private sub: Subscription;

  constructor(
    private store: AppStore,
    private router: Router,
    private route: ActivatedRoute,
    private title: Title,
    private createPackageDialog: MatDialog,
  ) {}

  ngOnInit() {
    this.sub = this.route.parent.params.subscribe((params) => {
      this.title.setTitle(`Origins › ${params.origin} › Packages | ${this.store.getState().app.name}`);
    });
  }

  ngOnDestroy() {
    if (this.sub) {
      this.sub.unsubscribe();
    }
  }

  get builderEnabled() {
    return config.enable_builder;
  }

  get integrations() {
    return this.store.getState().origins.currentIntegrations.integrations;
  }

  get memberOfOrigin() {
    return !!this.store.getState().origins.mine.find(origin => origin['name'] === this.origin);
  }

  get origin() {
    return this.store.getState().origins.current.name;
  }

  get hasPrivateKey() {
    const currentOrigin = this.store.getState().origins.current;
    return currentOrigin.name === this.origin && !!currentOrigin.private_key_name;
  }

  get packagesUi() {
    return this.store.getState().packages.ui.visible;
  }

  get perPage() {
    return this.store.getState().packages.perPage;
  }

  get projectsUi() {
    return this.store.getState().projects.ui.visible;
  }

  get projects() {
    return this.store.getState().projects.visible;
  }

  get projectsExist() {
    return this.projectsUi.exists && this.projects.size > 0;
  }

  get packages() {
    return this.store.getState().packages.visible;
  }

  get token() {
    return this.store.getState().session.token;
  }

  get defaultVisibility() {
    return this.store.getState().origins.current.default_package_visibility;
  }

  get totalCount() {
    return this.store.getState().packages.totalCount;
  }

  get noPackages() {
    return (!this.packagesUi.exists || this.packages.size === 0) && !this.packagesUi.loading;
  }

  get docsUrl() {
    return config['docs_url'];
  }

  fetchMorePackages() {
    this.store.dispatch(
      getUniquePackages(this.origin, this.store.getState().packages.nextRange, this.token)
    );
  }

  saved(project) {
    this.router.navigate(['/pkgs', project.origin, project.name, 'settings']);
  }

  toggled(active) {
    this.selectingPlan = active;
  }

  showCreatePackageDialog() {
    this.createPackageDialog.open(PackageCreateDialog, {
      width: '480px'
    });
  }
}
