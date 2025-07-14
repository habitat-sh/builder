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

  constructor(private store: AppStore, private router: Router) { }

  canActivate(route: ActivatedRouteSnapshot, routerState: RouterStateSnapshot): Promise<boolean> {
    const state = this.store.getState();
    const signedIn = !!state.session.token;
    const signingIn = state.users.current.isSigningIn;
    const signInFailed = state.users.current.failedSignIn;
    console.log('Guard canActivate called:', {
      url: routerState.url,
      signedIn: signedIn,
      signingIn: signingIn,
      signInFailed: signInFailed
    });
    // Reset fetchAttempted flag to ensure fresh license check on each navigation
    console.log('Resetting fetchAttempted flag for URL:', routerState.url);
    this.fetchAttempted = false;
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
        console.log('SigningIn: Checking browser location hash:', hash, 'extracted path:', currentPath);
        if (currentPath && currentPath !== '/' && currentPath !== '/sign-in' && !currentPath.includes('access_token')) {
          console.log('SigningIn: Saving browser location path as redirect path:', currentPath);
          Browser.setCookie(key, currentPath);
        } else if (routerState.url && routerState.url !== '/' && routerState.url !== '/sign-in') {
          console.log('SigningIn: Saving router URL as redirect path:', routerState.url);
          Browser.setCookie(key, routerState.url);
        }
      }
      // If already handling sign-in, return the existing promise
      if (this.signingInPromise) {
        console.log('SignIn already in progress, returning existing promise');
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
          console.log('User not signed in, sending home instead of redirecting to sign-in');
          this.sendHome();
        }
        else {
          console.log('User not signed in, redirecting to sign-in with URL:', routerState.url);
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
    // Debug logging
    console.log('License validation debug:', {
      license: license,
      isValid: isValid,
      licenseFetchInProgress: licenseFetchInProgress,
      requestedUrl: requestedUrl,
      fetchAttempted: this.fetchAttempted
    });
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
      console.log('License is null - fetching license');
      this.fetchAttempted = true;
      this.store.dispatch(fetchLicenseKey(state.session.token));
      // Wait for license fetch to complete
      const unsub = this.store.subscribe(newState => {
        const newLicense = newState.users.current.license;
        const newLicenseFetchInProgress = newLicense && (newLicense.get ? newLicense.get('licenseFetchInProgress') : newLicense.licenseFetchInProgress);
        console.log('License fetch progress changed:', newLicenseFetchInProgress);
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
    // If license is still unknown after fetch attempt, redirect to profile
    if (isValid === null && this.fetchAttempted) {
      console.log('License unknown after fetch - redirecting to profile');
      // Only redirect if not already on profile page
      if (requestedUrl !== '/profile') {
        this.router.navigate(['/profile']);
      }
      resolve(true);
      return;
    }
    // If license exists, check validity and route accordingly
    if (isValid) {
      console.log('License is valid - allowing navigation', {
        requestedUrl: requestedUrl,
        shouldAllow: requestedUrl && requestedUrl !== '/',
        willRedirect: !requestedUrl || requestedUrl === '/'
      });
      // If requesting a specific URL, allow navigation to proceed
      if (requestedUrl && requestedUrl !== '/') {
        console.log('License valid - allowing navigation to requested URL:', requestedUrl);
        resolve(true);
      } else {
        // Only redirect to /origins when accessing root route
        console.log('License valid - no specific URL requested, redirecting to /origins');
        this.router.navigate(['/origins']);
        resolve(true);
      }
    } else {
      // Invalid or expired license, redirect to profile
      console.log('License is invalid/expired - redirecting to profile', {
        requestedUrl: requestedUrl,
        currentUrl: this.router.url,
        shouldRedirect: requestedUrl !== '/profile'
      });
      // Only redirect if not already on profile page
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
        console.log('HandleSigningIn: Retrieved redirect path from cookie:', path);
        // If no path in cookie, try to get it from browser location first, then current URL
        if (!path) {
          // First try to get the intended destination from browser location hash
          const hash = window.location.hash;
          const hashPath = hash.replace('#', '');
          console.log('HandleSigningIn: No cookie path, checking browser hash:', hash, 'extracted:', hashPath);
          if (hashPath && hashPath !== '/' && hashPath !== '/sign-in' && !hashPath.includes('access_token')) {
            path = hashPath;
            console.log('HandleSigningIn: Using browser hash path as redirect path:', path);
          } else {
            // Fallback to current router URL
            const currentUrl = this.router.url;
            console.log('HandleSigningIn: No valid hash, using current URL:', currentUrl);
            if (currentUrl && currentUrl !== '/' && currentUrl !== '/sign-in') {
              path = currentUrl;
              console.log('HandleSigningIn: Using current URL as redirect path:', path);
            }
          }
        }
        if (config.is_saas) {
          console.log('HandleSigningIn: Calling handleLicenseValidation with path:', path);
          // Create wrapper functions that handle navigation and remove the cookie after processing
          const resolveWrapper = (result) => {
            Browser.removeCookie(name);
            console.log('HandleSigningIn: Cookie removed after successful resolution');
            // If we have a valid redirect path, navigate to it
            if (path && path !== '/') {
              console.log('HandleSigningIn: Navigating to saved redirect path:', path);
              this.router.navigate([path]);
            }
            resolve(result);
          };
          const rejectWrapper = (error) => {
            Browser.removeCookie(name);
            console.log('HandleSigningIn: Cookie removed after rejection');
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
    console.log('RedirectToSignIn called with URL:', url);
    // Only show the sign-in message for SaaS mode
    if (config['is_saas']) {
      // Save the redirect path for after sign-in
      if (url) {
        const key = 'redirectPath';
        console.log('RedirectToSignIn: Saving redirect path to cookie:', url);
        if (!Browser.getCookie(key)) {
          Browser.setCookie(key, url);
          console.log('RedirectToSignIn: Cookie set successfully');
        } else {
          console.log('RedirectToSignIn: Cookie already exists:', Browser.getCookie(key));
        }
      } else {
        console.log('RedirectToSignIn: No URL provided, not saving redirect path');
      }
      this.router.navigate(['/sign-in'], { queryParams: { message: 'You need to sign-in to access Public Builder' } });
      this.store.dispatch(signOut(false)); // Only clear session, do not navigate
    } else {
      this.store.dispatch(signOut(true, url));
    }
  }
}
