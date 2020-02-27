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
import { Router } from '@angular/router';
import { Subject } from 'rxjs';
import { takeUntil } from 'rxjs/operators';
import { AppStore } from '../../app.store';
import { packageString, parseDate, targetsFromPkgVersions } from '../../util';
import { demotePackage, filterPackagesBy } from '../../actions/index';

@Component({
  template: require('./package-versions.component.html')
})
export class PackageVersionsComponent implements OnDestroy {
  origin: string;
  name: string;
  selected: string;

  private isDestroyed$: Subject<boolean> = new Subject();

  constructor(
    private store: AppStore,
    private router: Router,
    private title: Title
  ) {
    this.store.observe('router.route.params')
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(params => {
        this.origin = params.origin;
        this.name = params.name;
        this.title.setTitle(`Packages › ${this.origin}/${this.name} › Versions | ${store.getState().app.name}`);
      });

    this.store.observe('router.route.params.version')
      .pipe(takeUntil(this.isDestroyed$))
      .subscribe(version => {
        this.toggle(version);
      });
  }

  ngOnDestroy() {
    this.isDestroyed$.next(true);
    this.isDestroyed$.complete();
  }

  get ident() {
    return {
      origin: this.origin,
      name: this.name
    };
  }

  toggle(version: string) {
    if (this.selected === version) {
      this.selected = null;
    } else {
      this.selected = version;

      this.fetchPackages({
        origin: this.origin,
        name: this.name,
        version: version
      });
    }
  }

  itemLinkFor(version: string): string[] {
    const linkPath = ['/pkgs', this.origin, this.name];
    if (this.selected !== version) {
      linkPath.push(version);
    }
    return linkPath;
  }

  platformsFor(version) {
    return targetsFromPkgVersions([version]).map(target => target.id);
  }

  fetchPackages(params) {
    this.store.dispatch(filterPackagesBy(params, null, false));
  }

  handleDemote(pkg, channel) {
    let token = this.store.getState().session.token;
    this.store.dispatch(demotePackage(pkg.origin, pkg.name, pkg.version, pkg.release, pkg.platforms[0], channel, token));
  }

  promotable(pkg) {
    return this.memberOfOrigin && pkg.channels.indexOf('stable') === -1;
  }

  get memberOfOrigin() {
    return !!this.store.getState().origins.mine.find(origin => origin['name'] === this.origin);
  }

  packageString(pkg) {
    return packageString(pkg);
  }

  releaseToDate(release) {
    return parseDate(release);
  }

  osIconFor(pkg) {
    return pkg.target || 'linux';
  }

  toggleFor(version: string): string {
    return this.selected === version ? 'chevron-up' : 'chevron-down';
  }

  navigateTo(pkg) {
    let params = ['pkgs', pkg.origin, pkg.name, pkg.version, pkg.release];
    this.router.navigate(params);
  }

  get versions() {
    return this.store.getState().packages.versions || [];
  }

  packagesFor(version) {
    let packages = this.store.getState().packages.visible;

    if (packages && packages.size > 0 && packages.get(0).version === version.version) {
      return packages;
    }

    return [];
  }
}
