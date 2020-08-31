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
import { requestRoute } from '../../actions/index';

@Component({
  template: require('./package-release.component.html')
})
export class PackageReleaseComponent implements OnDestroy {

  private isDestroyed$: Subject<boolean> = new Subject();

  constructor(
    private store: AppStore,
    private title: Title
  ) {
    this.store.observe('router.route.params')
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(({ origin, name, version, release }) => {
        this.title.setTitle(`Packages › ${origin}/${name}/${version}/${release} | ${store.getState().app.name}`);
      });

    this.store.observe('packages.ui.current.errorMessage')
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(errorMessage => {
        if (errorMessage) {
          this.store.dispatch(requestRoute(['/pkgs']));
        }
      });
  }

  ngOnDestroy() {
    this.isDestroyed$.next(true);
    this.isDestroyed$.complete();
  }

  get package() {
    return this.store.getState().packages.current;
  }
}
