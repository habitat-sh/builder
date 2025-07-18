import { Injectable } from '@angular/core';
import { CanActivate, Router, ActivatedRouteSnapshot, RouterStateSnapshot } from '@angular/router';
import { AppStore } from '../../app.store';
import { fetchLicenseKey } from '../../actions/index';
import config from '../../config';

@Injectable()
export class LicenseRequiredGuard implements CanActivate {
  private fetchAttempted = false;
  private licenseFetchPromise: Promise<void> = null;

  constructor(private store: AppStore, private router: Router) {}

  canActivate(route: ActivatedRouteSnapshot, state: RouterStateSnapshot): Promise<boolean> {
    // Allow all navigation if not SaaS mode
    if (!config.is_saas) {
      return Promise.resolve(true);
    }
    const appState = this.store.getState();
    const isSignedIn = !!appState.session.token;
    // If user is not signed in, deny navigation (SignedInGuard will handle sign-in)
    if (!isSignedIn) {
      return Promise.resolve(false);
    }
    // Allow access to profile page even without valid license (for SaaS users to add license)
    if (state.url === '/profile') {
      return Promise.resolve(true);
    }
    // For all other routes, check license validity
    return new Promise((resolve) => {
      this.handleLicenseValidation(resolve, state.url);
    });
  }

  private handleLicenseValidation(resolve, requestedUrl?: string) {
    const appState = this.store.getState();
    const license = appState.users.current.license;
    const isValid = license && (license.get ? license.get('isValid') : license.isValid);
    const licenseFetchInProgress = license && (license.get ? license.get('licenseFetchInProgress') : license.licenseFetchInProgress);
    const isSigningIn = appState.users.current.isSigningIn;
    // If license fetch is in progress or user is signing in, wait for completion
    if (licenseFetchInProgress || isSigningIn) {
      const unsub = this.store.subscribe(newState => {
        const newLicense = newState.users.current.license;
        const newLicenseFetchInProgress = newLicense && (newLicense.get ? newLicense.get('licenseFetchInProgress') : newLicense.licenseFetchInProgress);
        const newIsSigningIn = newState.users.current.isSigningIn;
        if (!newLicenseFetchInProgress && !newIsSigningIn) {
          unsub();
          setTimeout(() => {
            this.handleLicenseValidation(resolve, requestedUrl);
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
            this.handleLicenseValidation(resolve, requestedUrl);
          }, 0);
        }).catch(() => {
          // If license fetch failed, redirect to profile
          this.router.navigate(['/profile']);
          resolve(true);
        });
        return;
      }
      if (!this.fetchAttempted) {
        // Starting license fetch
        this.fetchAttempted = true;
        // Create promise to track license fetch and prevent multiple calls
        this.licenseFetchPromise = new Promise<void>((resolveFetch, rejectFetch) => {
          this.store.dispatch(fetchLicenseKey(appState.session.token));
          // Set timeout to prevent infinite waiting
          const timeout = setTimeout(() => {
            this.licenseFetchPromise = null;
            this.fetchAttempted = false; // Reset to allow retry
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
              if (fetchedMessage) {
                // License fetch failed (e.g., 404 - user has no license)
                rejectFetch(new Error(fetchedMessage));
              } else {
                // License fetch completed successfully
                resolveFetch();
              }
            }
          });
        });
        // Wait for fetch completion then re-validate
        this.licenseFetchPromise.then(() => {
          setTimeout(() => {
            this.handleLicenseValidation(resolve, requestedUrl);
          }, 0);
        }).catch(() => {
          // If license fetch failed (e.g., 404), redirect to profile
          this.router.navigate(['/profile']);
          resolve(true);
        });
        return;
      } else {
        // License fetch was already attempted but still null - redirect to profile
        this.router.navigate(['/profile']);
        resolve(true);
        return;
      }
    }
    // If license is valid, allow navigation
    if (isValid === true) {
      resolve(true);
    } else {
      // License is invalid/expired, redirect to profile
      this.router.navigate(['/profile']);
      resolve(true);
    }
  }
}
