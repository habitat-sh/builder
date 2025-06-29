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

  canActivate(route, routerState): Promise<boolean> {
    const state = this.store.getState();
    const signedIn = !!state.session.token;
    const signingIn = state.users.current.isSigningIn;
    const signInFailed = state.users.current.failedSignIn;

    return new Promise((resolve) => {
      if (signedIn) {
        if (config.is_saas) {
          this.store.dispatch(fetchLicenseKey(state.session.token));
          if (state.license.isValid === null) {
            this.waitForLicense(resolve, routerState.url);
            return;
          }
          if (state.license.isValid) {
            this.router.navigate(['/origins']);
            resolve(false); // Block original navigation, force redirect
            return;
          } else {
            // If already on /profile, allow navigation, else redirect
            if (routerState.url.startsWith('/profile')) {
              resolve(true);
            } else {
              this.router.navigate(['/profile']);
              resolve(false);
            }
            return;
          }
        }
        this.router.navigate(['/origins']);
        resolve(false);
        return;
      }
      if (signInFailed) {
        this.redirectToSignIn();
        resolve(false);
        return;
      }
      if (signingIn) {
        this.handleSigningIn(resolve, routerState.url);
        return;
      }
      this.redirectToSignIn(routerState.url);
      resolve(false);
    });
  }

  private handleSigningIn(resolve, requestedUrl?: string) {
    const unsub = this.store.subscribe(state => {
      if (state.oauth.token && state.session.token) {
        // After OAuth, check license for SaaS
        if (config.is_saas) {
          const isValid = state.users.current.license.isValid;
          if (isValid === null) {
            // License still loading, wait
            return;
          }
          unsub();
          if (isValid) {
            this.router.navigate(['/origins']);
            resolve(false);
          } else {
            if (requestedUrl && requestedUrl.startsWith('/profile')) {
              resolve(true);
            } else {
              this.router.navigate(['/profile']);
              resolve(false);
            }
          }
        } else {
          // Non-SaaS: just resolve and navigate to redirectPath if present
          const name = 'redirectPath';
          const path = Browser.getCookie(name);
          Browser.removeCookie(name);
          if (path) {
            this.router.navigate([path]);
          }
          this.router.navigate(['/origins']);
          resolve(false);
          unsub();
        }
      } else if (state.users.current.failedSignIn) {
        this.redirectToSignIn();
        resolve(false);
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
      this.store.dispatch(signOut(true)); // Only clear session, do not navigate
    } else {
      this.store.dispatch(signOut(true, url));
    }
  }

  private waitForLicense(resolve, requestedUrl?: string) {
    const unsub = this.store.subscribe(state => {
      const isValid = state.users.current.license.isValid;
      if (isValid === null) {
        return; // Still loading
      }
      unsub();
      if (isValid) {
        this.router.navigate(['/origins']);
        resolve(false); // Block original navigation, force redirect
      } else {
        if (requestedUrl && requestedUrl.startsWith('/profile')) {
          resolve(true);
        } else {
          this.router.navigate(['/profile']);
          resolve(false);
        }
      }
    });
  }
}
