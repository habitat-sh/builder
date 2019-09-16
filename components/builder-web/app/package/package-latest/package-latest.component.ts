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
import config from '../../config';

@Component({
  template: require('./package-latest.component.html')
})
export class PackageLatestComponent implements OnDestroy {
  origin: string;
  name: string;

  private isDestroyed$: Subject<boolean> = new Subject();

  constructor(private store: AppStore, private title: Title) {
    this.store.observe('router.route.params')
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(params => {
        this.origin = params['origin'];
        this.name = params['name'];
        this.title.setTitle(`Packages › ${this.origin}/${this.name} › Latest | ${store.getState().app.name}`);
      });
  }

  ngOnDestroy() {
    this.isDestroyed$.next(true);
    this.isDestroyed$.complete();
  }

  get targets() {
    return this.store.getState().packages.currentPlatforms;
  }

  get config() {
    return config;
  }

  get hasLatest() {
    return !!this.store.getState().packages.latest.ident.name;
  }

  get ident() {
    return {
      origin: this.origin,
      name: this.name
    };
  }

  get latest() {
    return this.store.getState().packages.latest;
  }

  get ui() {
    return this.store.getState().packages.ui.latest;
  }
}
