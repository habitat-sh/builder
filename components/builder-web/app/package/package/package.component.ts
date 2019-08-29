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
import { combineLatest, Subject, Subscription } from 'rxjs';
import { filter, takeUntil } from 'rxjs/operators';
import { List } from 'immutable';
import { PackageJobComponent } from '../package-job/package-job.component';
import { PackageJobsComponent } from '../package-jobs/package-jobs.component';
import { PackageLatestComponent } from '../package-latest/package-latest.component';
import { PackageReleaseComponent } from '../package-release/package-release.component';
import { PackageVersionsComponent } from '../package-versions/package-versions.component';
import { AppStore } from '../../app.store';
import { fetchJobs, fetchIntegrations, fetchLatestPackage, fetchLatestInChannel, fetchOrigin, fetchProject, fetchPackageVersions, setCurrentPackageTarget, clearPackageVersions } from '../../actions/index';
import { targetFrom } from '../../util';

@Component({
  template: require('./package.component.html')
})
export class PackageComponent implements OnInit, OnDestroy {
  origin: string;
  name: string;
  target: string;
  showSidebar: boolean = false;
  showActiveJob: boolean = false;
  useFullWidth: boolean = true;

  private isDestroyed$: Subject<boolean> = new Subject();
  private poll: number;

  constructor(private store: AppStore) {
    const origin$ = this.store.observe('router.route.params.origin').pipe(filter(v => v));
    const name$ = this.store.observe('router.route.params.name').pipe(filter(v => v));
    const target$ = this.store.observe('router.route.params.target');
    const token$ = this.store.observe('session.token');
    const origins$ = this.store.observe('origins.mine');
    const platforms$ = this.store.observe('packages.currentPlatforms')
      .pipe(filter(platforms => platforms.length > 0));

    combineLatest(origin$, name$)
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(([origin, name]) => {
        this.origin = origin;
        this.name = name;
        this.fetchOrigin();
        this.fetchPackageVersions();
        this.fetchJobs();
      });

    combineLatest(target$, platforms$)
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(([target, platforms]) => {
        const defaultTarget = platforms[0];
        const currentTarget = target ?
          targetFrom('param', target || defaultTarget.param) :
          targetFrom('id', this.target || defaultTarget.id);
        this.target = currentTarget.id;
        this.store.dispatch(setCurrentPackageTarget(currentTarget));
        this.fetchLatest();
        this.fetchLatestStable();
      });

    combineLatest(origin$, name$, token$, origins$, platforms$)
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(() => {
        this.fetchProject();
      });
  }

  ngOnInit() {
    // When a build is active, check on it periodically so we can
    // indicate when it completes.
    this.poll = window.setInterval(() => {
      if (this.building) {
        this.fetchJobs();
      }
    }, 10000);
  }

  ngOnDestroy() {
    window.clearInterval(this.poll);

    this.store.dispatch(setCurrentPackageTarget(undefined));
    this.store.dispatch(clearPackageVersions());

    this.isDestroyed$.next(true);
    this.isDestroyed$.complete();
  }

  get ident() {
    return {
      origin: this.origin,
      name: this.name
    };
  }

  get isOriginMember() {
    return !!this.store.getState().origins.mine.find((o) => {
      return o.name === this.origin;
    });
  }

  get hasPlan() {
    return this.store.getState().projects.ui.current.exists;
  }

  get builderEnabled() {
    return this.store.getState().features.builder;
  }

  get activeJobs(): List<any> {
    const activeStates = ['Dispatched', 'Pending', 'Processing'];

    return this.store.getState().jobs.visible.filter((b) => {
      return activeStates.indexOf(b.state.toString()) !== -1;
    });
  }

  get activeJob() {
    let active = this.activeJobs.last();
    return active;
  }

  get building(): boolean {
    return this.activeJobs.size > 0;
  }

  get token() {
    return this.store.getState().session.token;
  }

  get visibility() {
    return this.store.getState().projects.current.visibility;
  }

  onRouteActivate(routedComponent) {
    this.showSidebar = false;
    this.showActiveJob = false;

    [
      PackageJobsComponent,
      PackageLatestComponent,
      PackageReleaseComponent,
      PackageVersionsComponent
    ].forEach((c) => {
      if (routedComponent instanceof c) {
        this.showSidebar = true;
        this.showActiveJob = true;
      }
    });

    if (routedComponent instanceof PackageJobComponent) {
      this.useFullWidth = true;
    }
  }

  private fetchOrigin() {
    this.store.dispatch(fetchOrigin(this.origin));
  }

  private fetchLatest() {
    this.store.dispatch(fetchLatestPackage(this.origin, this.name, this.target));
  }

  private fetchLatestStable() {
    this.store.dispatch(fetchLatestInChannel(this.origin, this.name, 'stable', this.target));
  }

  private fetchPackageVersions() {
    this.store.dispatch(fetchPackageVersions(this.origin, this.name));
  }

  private fetchProject() {
    if (this.token && this.origin && this.name && this.isOriginMember) {
      this.store.dispatch(fetchProject(this.origin, this.name, this.token, false));
      this.store.dispatch(fetchIntegrations(this.origin, this.token));
    }
  }

  private fetchJobs() {
    if (this.token) {
      this.store.dispatch(fetchJobs(this.origin, this.name, this.token));
    }
  }
}
