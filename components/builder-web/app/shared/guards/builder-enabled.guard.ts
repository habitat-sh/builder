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

import { Injectable } from '@angular/core';
import { ActivatedRouteSnapshot, CanActivate, Router, RouterStateSnapshot } from '@angular/router';
import { AppStore } from '../../app.store';
import { requestRoute } from '../../actions/index';

@Injectable()
export class BuilderEnabledGuard implements CanActivate {

  constructor(private store: AppStore, private router: Router) { }

  canActivate(route: ActivatedRouteSnapshot, routerState: RouterStateSnapshot): Promise<boolean> {
    const state = this.store.getState();

    return new Promise((resolve, reject) => {
      if (state.features.builder) {
        resolve(true);
      }
      else {
        reject(() => this.redirect('/pkgs'));
      }
    })
    .catch(next => next())
    .then(() => true);
  }

  private redirect(path: string) {
    this.store.dispatch(requestRoute([path]));
  }
}
