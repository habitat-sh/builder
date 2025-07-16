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
    // If license fetch is in progress or user is signing in, allow navigation (SignedInGuard handles the redirect)
    if (licenseFetchInProgress || isSigningIn) {
      return true;
    }
    // If user is signed in but license is null, allow navigation (SignedInGuard will fetch license)
    if (isSignedIn && isValid === null) {
      return true;
    }
    // If license is explicitly invalid (false), redirect to profile page
    if (isValid === false) {
      if (this.router.url !== '/profile') {
        this.router.navigate(['/profile']);
      }
      return false;
    }
    // If license is valid, allow navigation
    if (isValid === true) {
      return true;
    }
    // If not signed in, deny navigation (SignedInGuard will handle redirect to sign-in)
    if (!isSignedIn) {
      return false;
    }
    // Default: deny navigation for unexpected states
    return false;
  }
}
