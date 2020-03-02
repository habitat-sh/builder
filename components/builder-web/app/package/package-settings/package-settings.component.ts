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

import { Component, OnDestroy } from '@angular/core';
import { Title } from '@angular/platform-browser';
import { Subject } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { AppStore } from '../../app.store';
import { targetFrom } from '../../util';

@Component({
  selector: 'hab-package-settings',
  template: require('./package-settings.component.html')
})
export class PackageSettingsComponent implements OnDestroy {
  name: string;
  origin: string;
  target: string;

  private isDestroyed$: Subject<boolean> = new Subject();

  constructor(
    private store: AppStore,
    private title: Title
  ) {
    this.store.observe('router.route.params')
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(params => {
        this.origin = params.origin;
        this.name = params.name;
        const target = targetFrom('param', params.target);
        this.target = target ? target.id : null;
        this.title.setTitle(`Packages › ${this.origin}/${this.name} › Settings | ${store.getState().app.name}`);
      });
  }

  ngOnDestroy() {
    this.isDestroyed$.next(true);
    this.isDestroyed$.complete();
  }

  get projects() {
    return this.store.getState().projects.currentProjects;
  }

  get project() {
    return this.projects.filter(p => p.target === this.target)[0];
  }

  get integrations() {
    return this.store.getState().origins.currentIntegrations.integrations || [];
  }

  get loading() {
    return this.store.getState().projects.ui.current.loading;
  }

  saved(project) {
    window.scroll(0, 0);
  }
}
