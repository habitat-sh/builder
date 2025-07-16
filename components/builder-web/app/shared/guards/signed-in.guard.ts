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
import { requestRoute, signOut } from '../../actions/index';
import config from '../../config';

@Injectable()
export class SignedInGuard implements CanActivate {
  private signingInPromise: Promise<boolean> = null;

  constructor(private store: AppStore, private router: Router) { }

  canActivate(route: ActivatedRouteSnapshot, routerState: RouterStateSnapshot): Promise<boolean> {
    // For on-prem users, no authentication required - allow all navigation
    if (!config.is_saas) {
      return Promise.resolve(true);
    }
    // For SaaS users, require authentication
    const state = this.store.getState();
    const signedIn = !!state.session.token;
    const signingIn = state.users.current.isSigningIn;
    const signInFailed = state.users.current.failedSignIn;
    if (signedIn) {
      // SaaS user is authenticated, allow navigation
      // License validation is handled by LicenseRequiredGuard
      return Promise.resolve(true);
    } else if (signInFailed) {
      return Promise.reject(() => this.redirectToSignIn())
        .catch(next => next())
        .then(() => true);
    } else if (signingIn) {
      // Save current URL as potential redirect path if none exists
      const key = 'redirectPath';
      if (!Browser.getCookie(key)) {
        // Get the actual current route from browser location hash
        const hash = window.location.hash;
        const currentPath = hash.replace('#', '');
        if (currentPath && currentPath !== '/' && currentPath !== '/sign-in' && !currentPath.includes('access_token')) {
          Browser.setCookie(key, currentPath);
        } else if (routerState.url && routerState.url !== '/' && routerState.url !== '/sign-in') {
          Browser.setCookie(key, routerState.url);
        }
      }
      // If already handling sign-in, return the existing promise
      if (this.signingInPromise) {
        return this.signingInPromise;
      }
      // Create new promise for sign-in process
      this.signingInPromise = new Promise<boolean>((resolve, reject) => {
        this.handleSigningIn(resolve, reject);
      })
        .catch(next => next())
        .then((result) => {
          this.signingInPromise = null; // Clear the promise when done
          return true;
        });
      return this.signingInPromise;
    } else {
      return Promise.reject(() => {
        if (routerState.url === '/origins') {
          this.sendHome();
        } else {
          this.redirectToSignIn(routerState.url);
        }
      })
        .catch(next => next())
        .then(() => true);
    }
  }

  private handleSigningIn(resolve, reject) {
    const unsub = this.store.subscribe(state => {
      if (state.oauth.token && state.session.token) {
        const name = 'redirectPath';
        let path = Browser.getCookie(name);
        // If no path in cookie, try to get it from browser location first, then current URL
        if (!path) {
          // First try to get the intended destination from browser location hash
          const hash = window.location.hash;
          const hashPath = hash.replace('#', '');
          if (hashPath && hashPath !== '/' && hashPath !== '/sign-in' && !hashPath.includes('access_token')) {
            path = hashPath;
          } else {
            // Fallback to current router URL
            const currentUrl = this.router.url;
            if (currentUrl && currentUrl !== '/' && currentUrl !== '/sign-in') {
              path = currentUrl;
            }
          }
        }
        // For SaaS users, simply navigate to the redirect path after sign-in
        Browser.removeCookie(name);
        if (path && path !== '/') {
          this.router.navigate([path]);
        }
        resolve(true);
        unsub();
      } else if (state.users.current.failedSignIn) {
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
      // Save the redirect path for after sign-in
      if (url) {
        const key = 'redirectPath';
        if (!Browser.getCookie(key)) {
          Browser.setCookie(key, url);
        }
      }
      this.router.navigate(['/sign-in'], { queryParams: { message: 'You need to sign-in to access Public Builder' } });
      this.store.dispatch(signOut(false)); // Only clear session, do not navigate
    } else {
      this.store.dispatch(signOut(true, url));
    }
  }
}
