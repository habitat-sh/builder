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
import { Browser } from '../../browser';
import { fetchLicenseKey, requestRoute, signOut } from '../../actions/index';
import config from '../../config';

@Injectable()
export class SignedInGuard implements CanActivate {
  private fetchAttempted = false;

  constructor(private store: AppStore, private router: Router) { }

  canActivate(route: ActivatedRouteSnapshot, routerState: RouterStateSnapshot): Promise<boolean> {
    const state = this.store.getState();
    const signedIn = !!state.session.token;
    const signingIn = state.users.current.isSigningIn;
    const signInFailed = state.users.current.failedSignIn;

    return new Promise((resolve, reject) => {

      if (signedIn) {
        resolve(true);
      }
      else if (signInFailed) {
        reject(() => this.redirectToSignIn());
      }
      else if (signingIn) {
        this.handleSigningIn(resolve, reject);
      }
      else {
        reject(() => {
          if (routerState.url === '/origins') {
            this.sendHome();
          }
          else {
            this.redirectToSignIn(routerState.url);
          }
        });
      }
    })
      .catch(next => next())
      .then(() => true);
  }

  private handleSigningIn(resolve, reject) {
    const unsub = this.store.subscribe(state => {
      if (state.oauth.token && state.session.token) {
        const name = 'redirectPath';
        const path = Browser.getCookie(name);
        Browser.removeCookie(name);
        if (config.is_saas) {
          // Use isValid directly from store for license validation
          const license = state.users.current.license;
          const isValid = license && (license.get ? license.get('isValid') : license.isValid);
          const licenseFetchInProgress = license && (license.get ? license.get('licenseFetchInProgress') : license.licenseFetchInProgress);

          // If license fetch is in progress, wait for it to complete
          if (licenseFetchInProgress) {
            return;
          }

          // If license validity is unknown and fetch has not been attempted, fetch it
          if (isValid === null && !this.fetchAttempted) {
            this.fetchAttempted = true;
            this.store.dispatch(fetchLicenseKey(state.session.token));
            return;
          }

          // If license is still unknown after fetch attempt, redirect to profile
          if (isValid === null && this.fetchAttempted) {
            this.router.navigate(['/profile']);
            resolve(false);
            unsub();
            return;
          }

          // If license exists, check validity and route accordingly
          if (isValid) {
            this.router.navigate(['/origins']);
          } else {
            this.router.navigate(['/profile']);
          }
          resolve(true);
          unsub();
        } else {
          if (path) {
            this.router.navigate([path]);
          }
          resolve(true);
          unsub();
        }
      }
      else if (state.users.current.failedSignIn) {
        reject(() => this.redirectToSignIn());
        unsub();
      }
    });
  }

  private sendHome() {
    this.store.dispatch(requestRoute(['/pkgs']));
  }

  private redirectToSignIn(url?: string) {
    // Only show the sign-in message for SaaS mode
    if (config['is_saas']) {
      this.router.navigate(['/sign-in'], { queryParams: { message: 'You need to sign-in to access Public Builder' } });
      this.store.dispatch(signOut(false)); // Only clear session, do not navigate
    } else {
      this.store.dispatch(signOut(true, url));
    }
  }
}
