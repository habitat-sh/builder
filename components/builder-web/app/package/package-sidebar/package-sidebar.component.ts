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

import { Component, Input } from '@angular/core';
import { AppStore } from '../../app.store';
import { fetchLatestInChannel, fetchPackageVersions, submitJob } from '../../actions/index';

@Component({
  selector: 'hab-package-sidebar',
  template: require('./package-sidebar.component.html')
})
export class PackageSidebarComponent {
  @Input() origin: string;
  @Input() name: string;
  @Input() target: string;
  @Input() building: boolean = false;
  @Input() isOriginMember: boolean = false;
  @Input() hasPlan: boolean = false;

  constructor(private store: AppStore) { }

  build() {
    let token = this.store.getState().session.token;
    this.store.dispatch(submitJob(this.origin, this.name, token));
  }

  get buildButtonLabel() {
    return this.building ? 'Build pending' : 'Build latest version';
  }

  get buildButtonAriaLabel() {
    return this.building ? 'Build pending' : `Build latest ${this.platform.name} version`;
  }

  get exportCommand() {
    return `hab pkg export docker ${this.origin}/${this.name}`;
  }

  get isAService() {
    return this.latestStable && this.latestStable.is_a_service;
  }

  get latestStable() {
    return this.store.getState().packages.latestInChannel.stable;
  }

  get loadingLatestStable() {
    return this.store.getState().packages.ui.latestInChannel.stable.loading;
  }

  get project() {
    return this.store.getState().projects.current;
  }

  get runCommand() {
    return `hab start ${this.origin}/${this.name}`;
  }

  get autoBuildSetting() {
    return this.project.auto_build ? 'enabled' : 'disabled';
  }

  get repoName() {
    return (this.project.vcs_data.match(/github.com\/(.+)\.git$/) || [''])[1] || '';
  }

  get repoUrl() {
    return this.project.vcs_data.replace('.git', '');
  }

  get platform() {
    return this.store.getState().packages.currentPlatform;
  }
}
