import { Injectable } from '@angular/core';
import { CanActivate, Router, ActivatedRouteSnapshot, RouterStateSnapshot } from '@angular/router';
import { AppStore } from '../../app.store';
import config from '../../config';

@Injectable()
export class LicenseRequiredGuard implements CanActivate {

  constructor(private store: AppStore, private router: Router) {}

  canActivate(route: ActivatedRouteSnapshot, state: RouterStateSnapshot): boolean {
    // Allow all navigation if not SaaS mode
    if (!config.is_saas) {
      return true;
    }
    const appState = this.store.getState();
    const license = appState.users.current.license;
    const isValid = license && (license.get ? license.get('isValid') : license.isValid);
    const licenseFetchInProgress = license && (license.get ? license.get('licenseFetchInProgress') : license.licenseFetchInProgress);
    const isSigningIn = appState.users.current.isSigningIn;
    const isSignedIn = !!appState.session.token;
    console.log('LicenseRequiredGuard: Checking license for route:', state.url, {
      isValid: isValid,
      licenseFetchInProgress: licenseFetchInProgress,
      isSigningIn: isSigningIn,
      isSignedIn: isSignedIn
    });
    // If license fetch is in progress or user is signing in, allow navigation (SignedInGuard handles the redirect)
    if (licenseFetchInProgress || isSigningIn) {
      console.log('LicenseRequiredGuard: Allowing navigation - license fetch in progress or signing in');
      return true;
    }
    // If user is signed in but license is null, allow navigation (SignedInGuard will fetch license)
    if (isSignedIn && isValid === null) {
      console.log('LicenseRequiredGuard: User signed in but license not yet fetched - allowing navigation');
      return true;
    }
    // If license is explicitly invalid (false), redirect to profile page
    if (isValid === false) {
      console.log('LicenseRequiredGuard: License invalid, redirecting to profile');
      if (this.router.url !== '/profile') {
        this.router.navigate(['/profile']);
      }
      return false;
    }
    // If license is valid, allow navigation
    if (isValid === true) {
      console.log('LicenseRequiredGuard: License valid, allowing navigation');
      return true;
    }
    // If not signed in and license is null, allow navigation (SignedInGuard will handle redirect to sign-in)
    console.log('LicenseRequiredGuard: Not signed in - allowing navigation for SignedInGuard to handle');
    return true;
  }
}
