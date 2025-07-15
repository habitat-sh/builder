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
  private signingInPromise: Promise<boolean> = null;
  private licenseFetchPromise: Promise<void> = null;

  constructor(private store: AppStore, private router: Router) { }

  canActivate(route: ActivatedRouteSnapshot, routerState: RouterStateSnapshot): Promise<boolean> {
    const state = this.store.getState();
    const signedIn = !!state.session.token;
    const signingIn = state.users.current.isSigningIn;
    const signInFailed = state.users.current.failedSignIn;
    // Don't reset fetchAttempted if license fetch is in progress to prevent multiple API calls
    const license = state.users.current.license;
    const licenseFetchInProgress = license && (license.get ? license.get('licenseFetchInProgress') : license.licenseFetchInProgress);
    if (!licenseFetchInProgress && !this.licenseFetchPromise) {
      this.fetchAttempted = false;
    }
    if (signedIn) {
      // In SaaS mode, also check license validity for already signed-in users
      if (config.is_saas) {
        return new Promise((resolve, reject) => {
          this.handleLicenseValidation(resolve, reject, routerState.url);
        })
          .catch(next => next())
          .then(() => true);
      } else {
        return Promise.resolve(true);
      }
    }
    else if (signInFailed) {
      return Promise.reject(() => this.redirectToSignIn())
        .catch(next => next())
        .then(() => true);
    }
    else if (signingIn) {
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
    }
    else {
      return Promise.reject(() => {
        if (routerState.url === '/origins') {
          this.sendHome();
        }
        else {
          this.redirectToSignIn(routerState.url);
        }
      })
        .catch(next => next())
        .then(() => true);
    }
  }

  private handleLicenseValidation(resolve, reject, requestedUrl?: string) {
    const state = this.store.getState();
    const license = state.users.current.license;
    const isValid = license && (license.get ? license.get('isValid') : license.isValid);
    const licenseFetchInProgress = license && (license.get ? license.get('licenseFetchInProgress') : license.licenseFetchInProgress);
    // If license fetch is in progress, wait for it to complete
    if (licenseFetchInProgress) {
      const unsub = this.store.subscribe(newState => {
        const newLicense = newState.users.current.license;
        const newLicenseFetchInProgress = newLicense && (newLicense.get ? newLicense.get('licenseFetchInProgress') : newLicense.licenseFetchInProgress);
        if (!newLicenseFetchInProgress) {
          unsub();
          // Small delay to ensure state is properly updated
          setTimeout(() => {
            this.handleLicenseValidation(resolve, reject, requestedUrl);
          }, 0);
        }
      });
      return;
    }
    // If license validity is unknown, fetch it
    if (isValid === null) {
      // If license fetch promise already exists, wait for it
      if (this.licenseFetchPromise) {
        this.licenseFetchPromise.then(() => {
          setTimeout(() => {
            this.handleLicenseValidation(resolve, reject, requestedUrl);
          }, 0);
        }).catch((error) => {
          // If license fetch failed, redirect to profile to show license dialog
          if (requestedUrl !== '/profile') {
            this.router.navigate(['/profile']);
          }
          resolve(true);
        });
        return;
      }
      if (!this.fetchAttempted) {
        this.fetchAttempted = true;
        // Create promise to track license fetch and prevent multiple calls
        this.licenseFetchPromise = new Promise<void>((resolveFetch, rejectFetch) => {
          this.store.dispatch(fetchLicenseKey(state.session.token));
          // Set timeout to prevent infinite waiting
          const timeout = setTimeout(() => {
            this.licenseFetchPromise = null;
            rejectFetch(new Error('License fetch timeout'));
          }, 10000);
          // Wait for license fetch to complete
          const unsub = this.store.subscribe(newState => {
            const newLicense = newState.users.current.license;
            const newLicenseFetchInProgress = newLicense && (newLicense.get ? newLicense.get('licenseFetchInProgress') : newLicense.licenseFetchInProgress);
            if (!newLicenseFetchInProgress) {
              clearTimeout(timeout);
              unsub();
              this.licenseFetchPromise = null;
              // Check if fetch failed (404 case)
              const fetchedMessage = newLicense && (newLicense.get ? newLicense.get('fetchedLicenseMessage') : newLicense.fetchedLicenseMessage);
              const newIsValid = newLicense && (newLicense.get ? newLicense.get('isValid') : newLicense.isValid);
              // If there's an error message (like 404), treat as no license
              if (fetchedMessage) {
                // Mark as invalid to prevent further fetches
                this.fetchAttempted = true;
                rejectFetch(new Error(fetchedMessage));
              } else {
                resolveFetch();
              }
            }
          });
        });
        // Wait for fetch completion then re-validate
        this.licenseFetchPromise.then(() => {
          setTimeout(() => {
            this.handleLicenseValidation(resolve, reject, requestedUrl);
          }, 0);
        }).catch((error) => {
          // If license fetch failed (404), redirect to profile to show license dialog
          if (requestedUrl !== '/profile') {
            this.router.navigate(['/profile']);
          }
          resolve(true);
        });
        return;
      }
    }
    // If license is still unknown after fetch attempt, redirect to profile
    if (isValid === null && this.fetchAttempted) {
      // Only redirect if not already on profile page
      if (requestedUrl !== '/profile') {
        this.router.navigate(['/profile']);
      }
      resolve(true);
      return;
    }
    // If license exists, check validity and route accordingly
    if (isValid) {
      // If requesting a specific URL, allow navigation to proceed
      if (requestedUrl && requestedUrl !== '/') {
        resolve(true);
      } else {
        // Only redirect to /origins when accessing root route
        this.router.navigate(['/origins']);
        resolve(true);
      }
    } else {
      // Invalid or expired license, redirect to profile only if not already on profile page
      if (requestedUrl !== '/profile') {
        this.router.navigate(['/profile']);
      }
      resolve(true);
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
        if (config.is_saas) {
          // Create wrapper functions that handle navigation and remove the cookie after processing
          const resolveWrapper = (result) => {
            Browser.removeCookie(name);
            // If we have a valid redirect path, navigate to it
            if (path && path !== '/') {
              this.router.navigate([path]);
            }
            resolve(result);
          };
          const rejectWrapper = (error) => {
            Browser.removeCookie(name);
            reject(error);
          };
          this.handleLicenseValidation(resolveWrapper, rejectWrapper, path);
          unsub();
        } else {
          Browser.removeCookie(name);
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
